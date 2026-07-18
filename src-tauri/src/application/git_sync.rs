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

    /// 同步流程：fetch → merge → 导入 → 导出 → commit → push
    ///
    /// 核心原则：先拉后推。确保远程数据先进入本地数据库，再导出合并后的数据推送。
    /// 这样即使本地仓库与远程无共同祖先（unrelated histories），也不会丢失远程数据。
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

        // 1. fetch 远程
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

        // 2. merge 远程数据（优先于导出本地数据，确保远程数据先进入本地）
        if has_remote {
            let remote_ref = format!("origin/{}", config.branch);
            let has_local_commits = git_ops::run_git(&self.sync_dir, &["rev-parse", "HEAD"]).is_ok();

            if !has_local_commits {
                // 本地无提交（新设备首次同步/.git 被删）
                // git merge 需要至少一个本地提交，先创建一个空提交作为基线
                eprintln!("[同步] 本地无提交，创建初始提交后 merge 远程分支 {}", remote_ref);
                // 先 add + commit 当前文件（.gitignore 等），创建初始提交
                git_ops::run_git(&self.sync_dir, &["add", "-A"])?;
                let _ = git_ops::run_git(
                    &self.sync_dir,
                    &["commit", "-m", "Initial commit before sync", "--allow-empty"],
                );
            }

            // 现在本地一定有提交，可以 merge
            let rev_local = git_ops::run_git(&self.sync_dir, &["rev-parse", "HEAD"])
                .unwrap_or_default()
                .trim()
                .to_string();
            let rev_remote = git_ops::run_git(&self.sync_dir, &["rev-parse", &remote_ref])
                .unwrap_or_default()
                .trim()
                .to_string();

            if rev_local != rev_remote {
                // 远程有更新 → merge（使用 --allow-unrelated-histories 处理首次同步/换源场景）
                let merge_result = git_ops::run_git(
                    &self.sync_dir,
                    &["merge", &remote_ref, "--no-edit", "--allow-unrelated-histories"],
                );
                if merge_result.is_err() {
                    // 合并冲突 → 用 last-write-wins 解决
                    git_ops::resolve_conflicts(&self.sync_dir)?;

                    // 检查是否仍有未解决的冲突
                    let unresolved = git_ops::run_git(
                        &self.sync_dir,
                        &["diff", "--name-only", "--diff-filter=U"],
                    )
                    .unwrap_or_default();
                    if !unresolved.trim().is_empty() {
                        return Err(format!(
                            "同步合并失败，存在无法自动解决的冲突。请检查同步目录。\n未解决文件: {}",
                            unresolved.trim()
                        ));
                    }

                    git_ops::run_git(&self.sync_dir, &["add", "-A"])?;
                    let _ = git_ops::run_git(&self.sync_dir, &["commit", "--no-edit"]);
                }
            }
        }

        // 3. 导入远程 JSON 到数据库（merge 后 JSON 包含双方数据，last-write-wins 保护）
        let imported = sync_json_io::import_from_json(&self.sync_dir, note_repo, reminder_repo, template_repo)?;

        // 4. 导出本地数据为 JSON（此时 DB 已包含远程数据，clear_dir_json 不会丢失）
        sync_json_io::export_to_json(&self.sync_dir, note_repo, reminder_repo, template_repo)?;

        // 5. 添加并 commit
        git_ops::run_git(&self.sync_dir, &["add", "-A"])?;
        let status = git_ops::run_git(&self.sync_dir, &["status", "--porcelain"])?;
        let has_local_changes = !status.trim().is_empty();
        eprintln!("[同步] git status: {:?}, has_local_changes: {}", status.trim(), has_local_changes);

        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
        let commit_msg = format!("Sync {}", now);

        if has_local_changes {
            git_ops::run_git(&self.sync_dir, &["commit", "-m", &commit_msg])?;
        }

        // 6. push 前安全检查：防止推送导致远程大量文件被删除
        if has_remote {
            let remote_ref = format!("origin/{}", config.branch);
            let diff_output = git_ops::run_git(
                &self.sync_dir,
                &["diff", "--name-status", &remote_ref, "HEAD"],
            )
            .unwrap_or_default();

            let deletions = diff_output.lines().filter(|l| l.starts_with('D')).count();
            let total_changes = diff_output.lines().count();
            // 超过 50% 的变更是删除 → 疑似覆盖远程数据，拒绝推送
            if total_changes > 0 && deletions as f64 / total_changes as f64 > 0.5 {
                return Err(format!(
                    "同步安全检查：本次推送将删除远程 {} 个文件（共 {} 个变更），可能覆盖远程数据。请确认远程仓库配置是否正确。",
                    deletions, total_changes
                ));
            }
        }

        // 7. push
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

    /// 模拟场景 #1：新设备首次同步——本地无数据，远程已有大量数据
    /// 修复前：unrelated histories → merge 失败 → force push 覆盖远程
    /// 修复后：allow-unrelated-histories → 远程数据先导入 DB → 再导出推送 → 远程数据保留
    #[test]
    fn test_sync_new_device_with_remote_data() {
        let dir = temp_dir();

        // === 第一台设备：推送数据到远程 ===
        let bare_repo = dir.join("remote.git");
        init_bare_repo(&bare_repo);

        let device1 = dir.join("device1");
        std::fs::create_dir_all(&device1).unwrap();
        let git_sync1 = GitSync::new(&device1);
        let config1 = SyncConfig {
            repo_url: bare_repo.to_str().unwrap().to_string(),
            username: "test".to_string(),
            token: "test".to_string(),
            branch: "main".to_string(),
            auto_sync: false,
        };
        git_sync1.save_config(&config1).unwrap();

        let note_repo1 = InMemoryNoteRepository::new();
        let reminder_repo1 = InMemoryReminderRepository::new();
        let template_repo1 = InMemoryTemplateRepository::new();
        // 第一台设备有 3 张便签
        let note_a = Note::new("便签A".to_string(), "amber".to_string());
        let note_b = Note::new("便签B".to_string(), "blue".to_string());
        let note_c = Note::new("便签C".to_string(), "green".to_string());
        note_repo1.save(&note_a).unwrap();
        note_repo1.save(&note_b).unwrap();
        note_repo1.save(&note_c).unwrap();

        let result1 = git_sync1.sync(&note_repo1, &reminder_repo1, &template_repo1, false);
        assert!(result1.is_ok(), "第一台设备同步失败: {:?}", result1);

        // 验证远程有数据
        let remote_files: Vec<_> = std::fs::read_dir(git_sync1.sync_dir.join("notes"))
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
            .collect();
        assert_eq!(remote_files.len(), 3, "远程应有 3 个便签 JSON");

        // === 第二台设备：新安装，本地只有 1 张便签 ===
        let device2 = dir.join("device2");
        std::fs::create_dir_all(&device2).unwrap();
        let git_sync2 = GitSync::new(&device2);
        let config2 = SyncConfig {
            repo_url: bare_repo.to_str().unwrap().to_string(),
            username: "test".to_string(),
            token: "test".to_string(),
            branch: "main".to_string(),
            auto_sync: false,
        };
        git_sync2.save_config(&config2).unwrap();

        let note_repo2 = InMemoryNoteRepository::new();
        let reminder_repo2 = InMemoryReminderRepository::new();
        let template_repo2 = InMemoryTemplateRepository::new();
        // 新设备只有 1 张便签
        let note_d = Note::new("新便签D".to_string(), "red".to_string());
        note_repo2.save(&note_d).unwrap();

        // 执行同步——这是之前导致远程数据丢失的场景
        let result2 = git_sync2.sync(&note_repo2, &reminder_repo2, &template_repo2, false);
        assert!(result2.is_ok(), "新设备同步失败: {:?}", result2);

        // ✅ 验证：远程数据没有被覆盖，DB 应包含 4 张便签（3 远程 + 1 本地）
        let all_notes = note_repo2.find_all().unwrap();
        let archived = note_repo2.find_archived().unwrap();
        let total: usize = all_notes.len() + archived.len();
        assert_eq!(total, 4, "新设备同步后应有 4 张便签（3 远程 + 1 本地），实际: {}", total);

        // 验证远程仓库仍有所有数据（通过再次同步验证）
        let device3 = dir.join("device3");
        std::fs::create_dir_all(&device3).unwrap();
        let git_sync3 = GitSync::new(&device3);
        let config3 = SyncConfig {
            repo_url: bare_repo.to_str().unwrap().to_string(),
            username: "test".to_string(),
            token: "test".to_string(),
            branch: "main".to_string(),
            auto_sync: false,
        };
        git_sync3.save_config(&config3).unwrap();

        let note_repo3 = InMemoryNoteRepository::new();
        let reminder_repo3 = InMemoryReminderRepository::new();
        let template_repo3 = InMemoryTemplateRepository::new();
        let result3 = git_sync3.sync(&note_repo3, &reminder_repo3, &template_repo3, false);
        assert!(result3.is_ok(), "第三台设备拉取失败: {:?}", result3);

        let all_notes3 = note_repo3.find_all().unwrap();
        let archived3 = note_repo3.find_archived().unwrap();
        let total3: usize = all_notes3.len() + archived3.len();
        assert_eq!(total3, 4, "第三台设备应拉取到 4 张便签，实际: {}", total3);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// 模拟场景 #4：换源——本地仓库关联 A 仓库，后切换到 B 仓库（B 有数据）
    /// 修复前：unrelated histories → merge 失败 → force push 覆盖 B 仓库数据
    /// 修复后：allow-unrelated-histories → 双方数据合并
    #[test]
    fn test_sync_switch_remote_repo() {
        let dir = temp_dir();

        // === 仓库 A ===
        let repo_a = dir.join("repo_a.git");
        init_bare_repo(&repo_a);

        // === 仓库 B ===
        let repo_b = dir.join("repo_b.git");
        init_bare_repo(&repo_b);

        // 先向仓库 B 推送数据（模拟 B 仓库已有数据）
        let prep_dir = dir.join("prep");
        std::fs::create_dir_all(&prep_dir).unwrap();
        let git_sync_prep = GitSync::new(&prep_dir);
        let config_prep = SyncConfig {
            repo_url: repo_b.to_str().unwrap().to_string(),
            username: "test".to_string(),
            token: "test".to_string(),
            branch: "main".to_string(),
            auto_sync: false,
        };
        git_sync_prep.save_config(&config_prep).unwrap();

        let note_repo_prep = InMemoryNoteRepository::new();
        let reminder_repo_prep = InMemoryReminderRepository::new();
        let template_repo_prep = InMemoryTemplateRepository::new();
        let note_b1 = Note::new("仓库B便签1".to_string(), "amber".to_string());
        let note_b2 = Note::new("仓库B便签2".to_string(), "blue".to_string());
        note_repo_prep.save(&note_b1).unwrap();
        note_repo_prep.save(&note_b2).unwrap();
        let prep_result = git_sync_prep.sync(&note_repo_prep, &reminder_repo_prep, &template_repo_prep, false);
        assert!(prep_result.is_ok(), "仓库B准备数据失败: {:?}", prep_result);

        // === 主设备：先同步到仓库 A ===
        let device = dir.join("device");
        std::fs::create_dir_all(&device).unwrap();
        let git_sync = GitSync::new(&device);

        let config_a = SyncConfig {
            repo_url: repo_a.to_str().unwrap().to_string(),
            username: "test".to_string(),
            token: "test".to_string(),
            branch: "main".to_string(),
            auto_sync: false,
        };
        git_sync.save_config(&config_a).unwrap();

        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();
        let note_a1 = Note::new("仓库A便签".to_string(), "green".to_string());
        note_repo.save(&note_a1).unwrap();

        let result_a = git_sync.sync(&note_repo, &reminder_repo, &template_repo, false);
        assert!(result_a.is_ok(), "同步到仓库A失败: {:?}", result_a);

        // 验证本地有仓库A的便签
        let notes_after_a = note_repo.find_all().unwrap();
        assert_eq!(notes_after_a.len(), 1);

        // === 换源：切换到仓库 B ===
        let config_b = SyncConfig {
            repo_url: repo_b.to_str().unwrap().to_string(),
            username: "test".to_string(),
            token: "test".to_string(),
            branch: "main".to_string(),
            auto_sync: false,
        };
        git_sync.save_config(&config_b).unwrap();

        // 执行同步——这是之前导致仓库B数据被覆盖的场景
        let result_b = git_sync.sync(&note_repo, &reminder_repo, &template_repo, false);
        assert!(result_b.is_ok(), "换源同步失败: {:?}", result_b);

        // ✅ 验证：仓库 B 的数据没有被覆盖，DB 应包含 3 张便签（1 本地 + 2 远程B）
        let all_notes = note_repo.find_all().unwrap();
        let archived = note_repo.find_archived().unwrap();
        let total: usize = all_notes.len() + archived.len();
        assert_eq!(total, 3, "换源同步后应有 3 张便签（1 本地 + 2 远程），实际: {}", total);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// 模拟场景：.git 目录被删后重新同步（等同于新设备首次同步）
    #[test]
    fn test_sync_git_dir_deleted() {
        let dir = temp_dir();

        // 先推送数据到远程
        let bare_repo = dir.join("remote.git");
        init_bare_repo(&bare_repo);

        let git_sync = GitSync::new(&dir);
        let config = SyncConfig {
            repo_url: bare_repo.to_str().unwrap().to_string(),
            username: "test".to_string(),
            token: "test".to_string(),
            branch: "main".to_string(),
            auto_sync: false,
        };
        git_sync.save_config(&config).unwrap();

        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();
        let note1 = Note::new("已有便签".to_string(), "amber".to_string());
        note_repo.save(&note1).unwrap();

        let result1 = git_sync.sync(&note_repo, &reminder_repo, &template_repo, false);
        assert!(result1.is_ok(), "首次同步失败: {:?}", result1);

        // 模拟 .git 目录被删除
        std::fs::remove_dir_all(git_sync.sync_dir.join(".git")).unwrap();

        // 再添加一张新便签
        let note2 = Note::new("新便签".to_string(), "blue".to_string());
        note_repo.save(&note2).unwrap();

        // 重新同步——本地仓库与远程无共同祖先
        let result2 = git_sync.sync(&note_repo, &reminder_repo, &template_repo, false);
        assert!(result2.is_ok(), ".git被删后重新同步失败: {:?}", result2);

        // 验证远程数据没有被覆盖
        let all_notes = note_repo.find_all().unwrap();
        let archived = note_repo.find_archived().unwrap();
        let total: usize = all_notes.len() + archived.len();
        assert_eq!(total, 2, ".git被删后同步应保留所有便签，实际: {}", total);

        std::fs::remove_dir_all(&dir).ok();
    }
}
