use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use super::git_ops;
use super::sync_config::SyncConfig;
use super::sync_json_io;
use crate::domain::{NoteRepository, ReminderRepository};
use tauri::Manager;

/// Git 同步管理器：持有路径与防抖状态，编排同步流程
pub struct GitSync {
    /// 配置文件路径（在 data_dir 下，不在 sync_dir 内）
    config_path: PathBuf,
    /// Git 仓库目录（data_dir/sync/）
    sync_dir: PathBuf,
    /// 自动同步防抖：记录最后一次触发时间
    last_sync_trigger: Mutex<std::time::Instant>,
}

impl GitSync {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            config_path: data_dir.join("sync_config.json"),
            sync_dir: data_dir.join("sync"),
            last_sync_trigger: Mutex::new(std::time::Instant::now()),
        }
    }

    /// 读取同步配置
    pub fn load_config(&self) -> Result<SyncConfig, String> {
        SyncConfig::load(&self.config_path)
    }

    /// 保存同步配置
    pub fn save_config(&self, config: &SyncConfig) -> Result<(), String> {
        config.save(&self.config_path)
    }

    /// 同步流程：导出 → commit → fetch → merge → 导入 → push
    pub fn sync(
        &self,
        note_repo: &dyn NoteRepository,
        reminder_repo: &dyn ReminderRepository,
    ) -> Result<String, String> {
        let config = self.load_config()?;
        if config.repo_url.is_empty() {
            return Err("请先配置同步仓库地址".to_string());
        }

        git_ops::init_repo(&self.sync_dir, &config.branch)?;

        // git config
        let _ = git_ops::run_git(&self.sync_dir, &["config", "user.name", &config.username]);
        let _ = git_ops::run_git(
            &self.sync_dir,
            &["config", "user.email", &format!("{}@sync.local", config.username)],
        );

        let auth_url = config.auth_url()?;

        // 设置远程
        let remote_check = git_ops::run_git(&self.sync_dir, &["remote", "get-url", "origin"]);
        if remote_check.is_err() {
            git_ops::run_git(&self.sync_dir, &["remote", "add", "origin", &auth_url])?;
        } else {
            git_ops::run_git(&self.sync_dir, &["remote", "set-url", "origin", &auth_url])?;
        }

        // 1. 导出本地数据为 JSON
        sync_json_io::export_to_json(&self.sync_dir, note_repo, reminder_repo)?;

        // 2. 添加并 commit 本地变更
        git_ops::run_git(&self.sync_dir, &["add", "-A"])?;
        let status = git_ops::run_git(&self.sync_dir, &["status", "--porcelain"])?;
        let has_local_changes = !status.trim().is_empty();
        eprintln!("[同步] git status: {:?}, has_local_changes: {}", status.trim(), has_local_changes);

        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
        let commit_msg = format!("Sync {}", now);

        if has_local_changes {
            git_ops::run_git(&self.sync_dir, &["commit", "-m", &commit_msg])?;
        }

        // 3. fetch 远程
        let fetch_result = git_ops::run_git(&self.sync_dir, &["fetch", "origin", &config.branch]);
        let has_remote = fetch_result.is_ok();
        eprintln!("[同步] fetch result: {:?}, has_remote: {}", fetch_result, has_remote);

        if has_remote {
            let remote_ref = format!("origin/{}", config.branch);

            let rev_local = git_ops::run_git(&self.sync_dir, &["rev-parse", "HEAD"])
                .unwrap_or_default()
                .trim()
                .to_string();
            let rev_remote = git_ops::run_git(&self.sync_dir, &["rev-parse", &remote_ref])
                .unwrap_or_default()
                .trim()
                .to_string();

            if rev_local != rev_remote {
                // 远程有更新 → merge（JSON 文件可自动合并）
                let merge_result = git_ops::run_git(&self.sync_dir, &["merge", &remote_ref, "--no-edit"]);
                if merge_result.is_err() {
                    // 合并冲突 → 用 last-write-wins 解决
                    git_ops::resolve_conflicts(&self.sync_dir)?;
                    git_ops::run_git(&self.sync_dir, &["add", "-A"])?;
                    let _ = git_ops::run_git(&self.sync_dir, &["commit", "--no-edit"]);
                }
            }
        }

        // 4. 导入合并后的 JSON 到数据库
        let imported = sync_json_io::import_from_json(&self.sync_dir, note_repo, reminder_repo)?;

        // 5. push
        git_ops::run_git(
            &self.sync_dir,
            &["push", "-u", "origin", &config.branch, "--force-with-lease"],
        )
        .map_err(|e| format!("推送失败: {}", e))?;

        if has_local_changes && has_remote {
            if imported > 0 {
                Ok(format!("已同步（{}，远程更新 {} 条）", commit_msg, imported))
            } else {
                Ok(format!("已推送本地变更（{}）", commit_msg))
            }
        } else if has_local_changes {
            Ok(format!("已推送本地变更（{}）", commit_msg))
        } else if has_remote {
            if imported > 0 {
                Ok(format!("已拉取远程更新（{} 条）", imported))
            } else {
                Ok("已是最新".to_string())
            }
        } else {
            Ok("已是最新".to_string())
        }
    }

    /// 触发自动同步（防抖 30 秒）
    pub fn schedule_auto_sync(&self, app: tauri::AppHandle) {
        let config = match self.load_config() {
            Ok(c) => c,
            Err(_) => return,
        };
        if !config.auto_sync || config.repo_url.is_empty() {
            return;
        }

        // 更新触发时间
        {
            let mut last = self.last_sync_trigger.lock().unwrap();
            *last = std::time::Instant::now();
        }

        // 30 秒后检查是否需要同步
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(30)).await;

            let state = app.state::<crate::AppState>();
            let should_sync = {
                let last = state.git_sync.last_sync_trigger.lock().unwrap();
                last.elapsed() >= Duration::from_secs(25)
            };

            if should_sync {
                eprintln!("[自动同步] 开始同步...");
                match state.git_sync.sync(state.note_repo.as_ref(), state.reminder_repo.as_ref()) {
                    Ok(msg) => eprintln!("[自动同步] {}", msg),
                    Err(e) => eprintln!("[自动同步] 失败: {}", e),
                }
            }
        });
    }

    /// 启动时自动拉取（如果配置了自动同步）
    pub fn auto_pull_on_startup(
        &self,
        note_repo: &dyn NoteRepository,
        reminder_repo: &dyn ReminderRepository,
    ) {
        let config = match self.load_config() {
            Ok(c) => c,
            Err(_) => return,
        };
        if !config.auto_sync || config.repo_url.is_empty() {
            return;
        }

        eprintln!("[自动同步] 启动时拉取远程数据...");
        match self.sync(note_repo, reminder_repo) {
            Ok(msg) => eprintln!("[自动同步] {}", msg),
            Err(e) => eprintln!("[自动同步] 启动拉取失败: {}", e),
        }
    }
}
