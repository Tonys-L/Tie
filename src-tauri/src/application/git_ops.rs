use std::path::Path;
use std::process::Command;

use super::sync_json_io;

/// 执行 git 命令
pub fn run_git(sync_dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(sync_dir)
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
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
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
