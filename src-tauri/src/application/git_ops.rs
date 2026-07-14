use std::path::Path;
use std::process::{Command, Stdio};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Windows: 隐藏控制台窗口的标志
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

use super::sync_json_io;

/// 执行 git 命令（Windows 下隐藏控制台窗口，stdin 重定向防止交互式挂起）
pub fn run_git(sync_dir: &Path, args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .current_dir(sync_dir)
        .stdin(Stdio::null());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let output = cmd
        .output()
        .map_err(|e| format!("执行 git 失败: {}。请确认已安装 git 并添加到 PATH。", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("{} {}", stderr.trim(), stdout.trim()).trim().to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// 初始化 Git 仓库
pub fn init_repo(sync_dir: &Path, branch: &str) -> Result<(), String> {
    let git_dir = sync_dir.join(".git");
    if git_dir.exists() {
        return Ok(());
    }

    std::fs::create_dir_all(sync_dir).map_err(|e| format!("创建目录失败: {}", e))?;
    run_git(sync_dir, &["init", "-b", branch])?;
    std::fs::write(sync_dir.join(".gitignore"), "")
        .map_err(|e| format!("写入 .gitignore 失败: {}", e))?;
    Ok(())
}

/// 检查 git 是否已安装
pub fn check_git_installed() -> bool {
    let mut cmd = Command::new("git");
    cmd.arg("--version").stdin(Stdio::null());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 列出远程仓库的所有分支名
///
/// 返回远程仓库的分支名列表（不含 refs/heads/ 前缀）。
/// 空列表表示远程仓库为空（无分支）。
pub fn list_remote_branches(auth_url: &str) -> Result<Vec<String>, String> {
    let mut cmd = Command::new("git");
    cmd.args(["ls-remote", "--heads"])
        .arg(auth_url)
        .stdin(Stdio::null());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let output = cmd
        .output()
        .map_err(|e| format!("执行 git 失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.trim().to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let branches: Vec<String> = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                parts[1].strip_prefix("refs/heads/").map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();
    Ok(branches)
}

/// 解决合并冲突：按 updated_at 时间戳取最新版本
pub fn resolve_conflicts(sync_dir: &Path) -> Result<(), String> {
    let output = run_git(sync_dir, &["diff", "--name-only", "--diff-filter=U"])?;
    let conflicted_files: Vec<&str> = output.lines().collect();

    for file in conflicted_files {
        let path = sync_dir.join(file);
        if !path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&path).map_err(|e| format!("读取冲突文件失败: {}", e))?;

        if content.contains("<<<<<<<") {
            let resolved = resolve_json_conflict(&content)?;
            std::fs::write(&path, resolved).map_err(|e| format!("写入解决后的文件失败: {}", e))?;
        }
    }

    Ok(())
}

/// 解析 JSON 冲突，按 updated_at 取最新
fn resolve_json_conflict(content: &str) -> Result<String, String> {
    let parts: Vec<&str> = content.splitn(3, "=======").collect();
    if parts.len() != 3 {
        // 无法解析，取 ours 版本
        let ours = content.split(">>>>>>>").next().unwrap_or(content);
        let ours = ours
            .lines()
            .filter(|l| !l.starts_with("<<<<<<<") && !l.starts_with("=======") && !l.starts_with(">>>>>>>"))
            .collect::<Vec<_>>()
            .join("\n");
        return Ok(ours);
    }

    let ours_raw = parts[0];
    let theirs_raw = parts[1];

    let ours: String = ours_raw
        .lines()
        .filter(|l| !l.starts_with("<<<<<<<"))
        .collect::<Vec<_>>()
        .join("\n");
    let theirs: String = theirs_raw
        .lines()
        .filter(|l| !l.starts_with("======="))
        .collect::<Vec<_>>()
        .join("\n");

    let ours_time = sync_json_io::extract_updated_at(&ours);
    let theirs_time = sync_json_io::extract_updated_at(&theirs);

    if theirs_time > ours_time {
        Ok(theirs)
    } else {
        Ok(ours)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_conflict(ours: &str, theirs: &str) -> String {
        format!("<<<<<<< HEAD\n{}=======\n{}>>>>>>> origin/main\n", ours, theirs)
    }

    #[test]
    fn test_resolve_conflict_theirs_newer() {
        let ours = r#"{"id":"1","updated_at":"2026-07-01T00:00:00Z"}"#;
        let theirs = r#"{"id":"1","updated_at":"2026-07-02T00:00:00Z"}"#;
        let conflict = make_conflict(ours, theirs);
        let resolved = resolve_json_conflict(&conflict).unwrap();
        assert!(resolved.contains("2026-07-02"));
    }

    #[test]
    fn test_resolve_conflict_ours_newer() {
        let ours = r#"{"id":"1","updated_at":"2026-07-03T00:00:00Z"}"#;
        let theirs = r#"{"id":"1","updated_at":"2026-07-01T00:00:00Z"}"#;
        let conflict = make_conflict(ours, theirs);
        let resolved = resolve_json_conflict(&conflict).unwrap();
        assert!(resolved.contains("2026-07-03"));
    }

    #[test]
    fn test_resolve_conflict_equal_times_keeps_ours() {
        let ours = r#"{"id":"1","updated_at":"2026-07-01T00:00:00Z"}"#;
        let theirs = r#"{"id":"1","updated_at":"2026-07-01T00:00:00Z"}"#;
        let conflict = make_conflict(ours, theirs);
        let resolved = resolve_json_conflict(&conflict).unwrap();
        // 时间相等时取 ours（theirs_time > ours_time 为 false）
        assert!(resolved.contains("2026-07-01"));
    }

    #[test]
    fn test_resolve_conflict_no_conflict_markers() {
        // 无冲突标记的内容，应返回过滤后的 ours
        let content = r#"{"id":"1","updated_at":"2026-07-01T00:00:00Z"}"#;
        let resolved = resolve_json_conflict(content).unwrap();
        assert!(resolved.contains("2026-07-01"));
    }

    #[test]
    fn test_resolve_conflict_malformed_falls_back_to_ours() {
        // 无法按 ====== 分割时，取 ours 版本
        let content = "<<<<<<< HEAD\n{\"id\":\"1\"}\n>>>>>>> origin/main\n";
        let resolved = resolve_json_conflict(content).unwrap();
        assert!(resolved.contains("\"id\":\"1\""));
    }
}
