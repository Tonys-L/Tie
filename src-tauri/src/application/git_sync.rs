use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use super::git_ops;
use super::sync_config::SyncConfig;
use super::sync_json_io;
use crate::domain::{NoteRepository, ReminderRepository, TemplateRepository};
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
    ///
    /// `create_branch`: 当远程分支不存在时是否创建分支。
    /// - false（默认）：分支不存在且远程有其他分支时返回特殊错误 `BRANCH_NOT_FOUND:<已有分支>`
    /// - true：跳过分支验证，直接 push（Git 自动创建远程分支）
    pub fn sync(
        &self,
        note_repo: &dyn NoteRepository,
        reminder_repo: &dyn ReminderRepository,
        template_repo: &dyn TemplateRepository,
        create_branch: bool,
    ) -> Result<String, String> {
        let config = self.load_config()?;
        if config.repo_url.is_empty() {
            return Err("请先配置同步仓库地址".to_string());
        }

        git_ops::init_repo(&self.sync_dir, &config.branch)?;

        // 确保本地分支名与配置一致（用户可能修改过分支名）
        // 如果当前分支名与配置不同，重命名本地分支
        let current_branch = git_ops::run_git(&self.sync_dir, &["branch", "--show-current"])
            .unwrap_or_default()
            .trim()
            .to_string();
        if !current_branch.is_empty() && current_branch != config.branch {
            eprintln!("[同步] 本地分支 {} 与配置 {} 不一致，重命名", current_branch, config.branch);
            let _ = git_ops::run_git(&self.sync_dir, &["branch", "-m", &current_branch, &config.branch]);
        }

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
        sync_json_io::export_to_json(&self.sync_dir, note_repo, reminder_repo, template_repo)?;

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
        let _ = git_ops::run_git(&self.sync_dir, &["fetch", "origin", &config.branch]);

        // 验证 origin/<branch> ref 是否真实存在
        // git fetch 即使远程分支不存在也可能返回成功（exit code 0），必须检查 ref
        let remote_ref = format!("origin/{}", config.branch);
        let rev_remote_result = git_ops::run_git(&self.sync_dir, &["rev-parse", &remote_ref]);
        let has_remote = rev_remote_result.is_ok();
        eprintln!("[同步] remote ref exists: {}", has_remote);

        if !has_remote {
            // origin/<branch> 不存在，检查远程仓库是否有任何分支
            match git_ops::list_remote_branches(&auth_url) {
                Ok(branches) if !branches.is_empty() => {
                    // 远程有分支但不是配置的分支名
                    if create_branch {
                        // 用户已确认创建分支，跳过检查直接 push
                        eprintln!("[同步] 用户确认创建分支 {}", config.branch);
                    } else {
                        // 返回特殊错误，前端检测前缀后弹窗让用户选择
                        return Err(format!(
                            "BRANCH_NOT_FOUND:{}",
                            branches.join(", ")
                        ));
                    }
                }
                Ok(_) => {
                    // 远程仓库为空（无分支），首次推送
                    eprintln!("[同步] 远程仓库为空，首次推送");
                }
                Err(e) => {
                    return Err(format!("无法连接远程仓库: {}", e));
                }
            }
        }

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
        let imported = sync_json_io::import_from_json(&self.sync_dir, note_repo, reminder_repo, template_repo)?;

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
                match state.git_sync.sync(state.note_repo.as_ref(), state.reminder_repo.as_ref(), state.template_repo.as_ref(), false) {
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
        template_repo: &dyn TemplateRepository,
    ) {
        let config = match self.load_config() {
            Ok(c) => c,
            Err(_) => return,
        };
        if !config.auto_sync || config.repo_url.is_empty() {
            return;
        }

        eprintln!("[自动同步] 启动时拉取远程数据...");
        match self.sync(note_repo, reminder_repo, template_repo, false) {
            Ok(msg) => eprintln!("[自动同步] {}", msg),
            Err(e) => eprintln!("[自动同步] 启动拉取失败: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mock_repo::{InMemoryNoteRepository, InMemoryReminderRepository, InMemoryTemplateRepository};
    use crate::domain::Note;

    use std::process::Stdio;

    /// 创建 bare 仓库（带 CREATE_NO_WINDOW 和 stdin 重定向）
    fn init_bare_repo(path: &PathBuf) {
        let mut cmd = std::process::Command::new("git");
        cmd.args(["init", "--bare"])
            .arg(path)
            .stdin(Stdio::null());
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let output = cmd.output().expect("git init --bare 失败，请确认 git 已安装");
        if !output.status.success() {
            panic!("git init --bare 失败: {}", String::from_utf8_lossy(&output.stderr));
        }
    }

    /// 创建临时目录
    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tie_gitsync_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_new_creates_correct_paths() {
        let dir = temp_dir();
        let git_sync = GitSync::new(&dir);

        // config_path 应为 data_dir/sync_config.json
        assert_eq!(git_sync.config_path, dir.join("sync_config.json"));
        // sync_dir 应为 data_dir/sync/
        assert_eq!(git_sync.sync_dir, dir.join("sync"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_load_config_nonexistent_returns_default() {
        let dir = temp_dir();
        let git_sync = GitSync::new(&dir);

        // 配置文件不存在时应返回默认值
        let config = git_sync.load_config().unwrap();
        assert_eq!(config.repo_url, "");
        assert_eq!(config.branch, "main");
        assert!(!config.auto_sync);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_save_and_load_config_roundtrip() {
        let dir = temp_dir();
        let git_sync = GitSync::new(&dir);

        let config = SyncConfig {
            repo_url: "https://github.com/test/repo.git".to_string(),
            username: "testuser".to_string(),
            token: "testtoken".to_string(),
            branch: "master".to_string(),
            auto_sync: true,
        };

        git_sync.save_config(&config).unwrap();
        let loaded = git_sync.load_config().unwrap();

        assert_eq!(loaded.repo_url, config.repo_url);
        assert_eq!(loaded.username, config.username);
        assert_eq!(loaded.token, config.token);
        assert_eq!(loaded.branch, config.branch);
        assert_eq!(loaded.auto_sync, config.auto_sync);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_sync_empty_repo_url_returns_error() {
        let dir = temp_dir();
        let git_sync = GitSync::new(&dir);
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();

        // 未配置仓库地址应返回错误
        let result = git_sync.sync(&note_repo, &reminder_repo, &template_repo, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("请先配置"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_auto_pull_skips_when_not_configured() {
        let dir = temp_dir();
        let git_sync = GitSync::new(&dir);
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();

        // 未配置自动同步 → 应直接返回，不报错
        git_sync.auto_pull_on_startup(&note_repo, &reminder_repo, &template_repo);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// 使用本地 bare git 仓库模拟远程仓库的集成测试
    #[test]
    fn test_sync_with_local_bare_repo() {
        let dir = temp_dir();

        // 创建 bare 仓库作为远程
        let bare_repo = dir.join("remote.git");
        init_bare_repo(&bare_repo);

        // 配置同步指向本地 bare 仓库
        let git_sync = GitSync::new(&dir);
        let config = SyncConfig {
            repo_url: bare_repo.to_str().unwrap().to_string(),
            username: "test".to_string(),
            token: "test".to_string(),
            branch: "main".to_string(),
            auto_sync: false,
        };
        git_sync.save_config(&config).unwrap();

        // 准备测试数据
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();
        let note = Note::new("测试便签".to_string(), "amber".to_string());
        note_repo.save(&note).unwrap();

        // 第一次同步：远程为空 → 首次推送
        let result = git_sync.sync(&note_repo, &reminder_repo, &template_repo, false);
        assert!(result.is_ok(), "首次同步失败: {:?}", result);
        let msg = result.unwrap();
        assert!(msg.contains("已推送") || msg.contains("已是最新"), "意外消息: {}", msg);

        // 验证 JSON 文件已生成
        assert!(git_sync.sync_dir.join("notes").join(format!("{}.json", note.id)).exists());

        // 第二次同步：无变更 → 已是最新
        let result2 = git_sync.sync(&note_repo, &reminder_repo, &template_repo, false);
        assert!(result2.is_ok(), "第二次同步失败: {:?}", result2);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// 测试分支不匹配时返回 BRANCH_NOT_FOUND 错误
    #[test]
    fn test_sync_branch_not_found() {
        let dir = temp_dir();

        // 创建 bare 仓库，先推送 main 分支
        let bare_repo = dir.join("remote.git");
        init_bare_repo(&bare_repo);

        // 先用 main 分支推送一次
        let git_sync = GitSync::new(&dir);
        let config_main = SyncConfig {
            repo_url: bare_repo.to_str().unwrap().to_string(),
            username: "test".to_string(),
            token: "test".to_string(),
            branch: "main".to_string(),
            auto_sync: false,
        };
        git_sync.save_config(&config_main).unwrap();

        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();
        let note = Note::new("测试".to_string(), "amber".to_string());
        note_repo.save(&note).unwrap();

        // 首次推送 main
        let _ = git_sync.sync(&note_repo, &reminder_repo, &template_repo, false).unwrap();

        // 切换到不存在的分支 master
        let config_master = SyncConfig {
            repo_url: bare_repo.to_str().unwrap().to_string(),
            username: "test".to_string(),
            token: "test".to_string(),
            branch: "master".to_string(),
            auto_sync: false,
        };
        git_sync.save_config(&config_master).unwrap();

        // 同步应返回 BRANCH_NOT_FOUND 错误
        let result = git_sync.sync(&note_repo, &reminder_repo, &template_repo, false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.starts_with("BRANCH_NOT_FOUND:"), "意外错误: {}", err);
        assert!(err.contains("main"), "应提示已有分支 main: {}", err);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// 测试 create_branch=true 时跳过分支检查
    #[test]
    fn test_sync_create_branch() {
        let dir = temp_dir();

        let bare_repo = dir.join("remote.git");
        init_bare_repo(&bare_repo);

        let git_sync = GitSync::new(&dir);
        let config_main = SyncConfig {
            repo_url: bare_repo.to_str().unwrap().to_string(),
            username: "test".to_string(),
            token: "test".to_string(),
            branch: "main".to_string(),
            auto_sync: false,
        };
        git_sync.save_config(&config_main).unwrap();

        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();
        let note = Note::new("测试".to_string(), "amber".to_string());
        note_repo.save(&note).unwrap();

        // 先推送 main
        let _ = git_sync.sync(&note_repo, &reminder_repo, &template_repo, false).unwrap();

        // 切换到 master 并用 create_branch=true
        let config_master = SyncConfig {
            repo_url: bare_repo.to_str().unwrap().to_string(),
            username: "test".to_string(),
            token: "test".to_string(),
            branch: "master".to_string(),
            auto_sync: false,
        };
        git_sync.save_config(&config_master).unwrap();

        // create_branch=true → 应成功创建新分支
        let result = git_sync.sync(&note_repo, &reminder_repo, &template_repo, true);
        assert!(result.is_ok(), "创建分支同步失败: {:?}", result);

        std::fs::remove_dir_all(&dir).ok();
    }
}
