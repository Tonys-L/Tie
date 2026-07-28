//! 系统命令：数据目录访问 + URL 打开（跨平台）。
//!
//! 调用方：前端 api.ts 通过 invoke 调用；lib.rs setup 通过 `paths::data_dir_path()` 获取数据目录。
//! 依赖：std::env / std::process + git_ops::CREATE_NO_WINDOW（Windows）。
//!
//! 注意：`data_dir_path` 的实现已提升到 `application/paths`，本模块仅转发调用，
//! 保持命令层薄壳化（不持有路径解析逻辑）。

/// 数据目录路径解析（转发到 application::paths，单一所有者）
pub fn data_dir_path() -> Result<std::path::PathBuf, String> {
    crate::application::paths::data_dir_path()
}

/// 获取数据目录路径
#[tauri::command]
pub fn get_data_dir() -> Result<String, String> {
    let db_dir = data_dir_path()?;
    db_dir.to_str()
        .map(|s| s.to_string())
        .ok_or("路径转换失败".to_string())
}

/// 在系统文件管理器中打开数据目录
#[tauri::command]
pub fn open_data_dir() -> Result<(), String> {
    let db_dir = data_dir_path()?;
    std::process::Command::new("explorer")
        .arg(&db_dir)
        .spawn()
        .map_err(|e| format!("打开目录失败: {}", e))?;
    Ok(())
}

/// 获取置顶免疫显示桌面配置
#[tauri::command]
pub async fn get_pin_desktop() -> Result<bool, String> {
    let config = crate::application::pin_desktop_config::PinDesktopConfig::load()?;
    Ok(config.enabled)
}

/// 设置置顶免疫显示桌面配置，并立即对所有已置顶窗口应用/取消 pin
#[tauri::command]
pub async fn set_pin_desktop(app: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    let config = crate::application::pin_desktop_config::PinDesktopConfig { enabled };
    config.save()?;

    // 立即对所有 note-* 窗口应用或取消 pin
    use tauri::Manager;

    for window in app.webview_windows().values() {
        let label = window.label();
        if !label.starts_with("note-") { continue; }
        let is_on_top = window.is_always_on_top().unwrap_or(false);
        if !is_on_top { continue; }
        #[cfg(target_os = "windows")]
        if enabled {
            let _ = crate::application::win_pin::pin_window(window);
        } else {
            let _ = crate::application::win_pin::unpin_window(window);
        }
    }

    Ok(enabled)
}

/// 在系统默认浏览器中打开 URL
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    // 仅允许 http/https
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("仅支持 http/https 链接".to_string());
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &url])
            .stdin(std::process::Stdio::null())
            .creation_flags(super::super::git_ops::CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("打开链接失败: {}", e))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("打开链接失败: {}", e))?;
    }
    Ok(())
}
