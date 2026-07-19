//! 同步与系统命令：Git 同步、快捷键、语言、数据目录、URL 打开。

use tauri::{AppHandle, State};

use crate::AppState;

// ============ 同步命令 ============

/// 获取同步配置
#[tauri::command]
pub async fn get_sync_config(state: State<'_, AppState>) -> Result<super::super::sync_config::SyncConfig, String> {
    state.git_sync.load_config()
}

/// 保存同步配置
#[tauri::command]
pub async fn save_sync_config(state: State<'_, AppState>, config: super::super::sync_config::SyncConfig) -> Result<(), String> {
    state.git_sync.save_config(&config)
}

/// 执行同步（导出JSON → git commit/fetch/merge → 导入JSON → push）
#[tauri::command]
pub async fn sync_notes(app: AppHandle, state: State<'_, AppState>, create_branch: Option<bool>) -> Result<String, String> {
    eprintln!("[同步] 开始执行同步... create_branch={:?}", create_branch);
    let result = super::super::note_service::sync_notes(
        state.note_repo.as_ref(),
        state.reminder_repo.as_ref(),
        state.template_repo.as_ref(),
        &state.git_sync,
        create_branch.unwrap_or(false),
    );
    eprintln!("[同步] 同步完成: {:?}", result);
    use tauri_plugin_notification::NotificationExt;
    match &result {
        Ok(msg) => { let _ = app.notification().builder().title(super::super::locale_manager::notify_sync_ok()).body(msg).show(); }
        Err(e) => { let _ = app.notification().builder().title(super::super::locale_manager::notify_sync_fail()).body(e).show(); }
    }
    result
}

/// 检查 git 是否已安装
#[tauri::command]
pub async fn check_git() -> bool {
    super::super::git_ops::check_git_installed()
}

// ============ 快捷键命令 ============

/// 获取快捷键配置
#[tauri::command]
pub fn get_shortcut_config(state: State<AppState>) -> super::super::shortcut_manager::ShortcutConfig {
    state.shortcut_manager.get_config()
}

/// 保存快捷键配置并重新注册
#[tauri::command]
pub fn save_shortcut_config(
    app: AppHandle,
    state: State<AppState>,
    config: super::super::shortcut_manager::ShortcutConfig,
) -> Result<(), String> {
    state.shortcut_manager.save_and_reregister(&app, config)
}

// ============ 国际化命令 ============

/// 设置语言并重建托盘菜单
#[tauri::command]
pub fn set_locale(app: AppHandle, locale: String) -> Result<(), String> {
    let code = if locale == "en" { 1u8 } else { 0u8 };
    super::super::locale_manager::set_locale_code(code);
    super::super::tray_manager::rebuild_tray_menu(&app)
}

// ============ 数据目录命令 ============

/// 获取数据目录路径
#[tauri::command]
pub fn get_data_dir() -> Result<String, String> {
    let db_dir = std::env::current_exe()
        .map_err(|e| format!("获取 exe 路径失败: {}", e))?
        .parent()
        .ok_or("无法获取父目录")?
        .join("data");
    db_dir.to_str()
        .map(|s| s.to_string())
        .ok_or("路径转换失败".to_string())
}

/// 在系统文件管理器中打开数据目录
#[tauri::command]
pub fn open_data_dir() -> Result<(), String> {
    let db_dir = std::env::current_exe()
        .map_err(|e| format!("获取 exe 路径失败: {}", e))?
        .parent()
        .ok_or("无法获取父目录")?
        .join("data");
    std::process::Command::new("explorer")
        .arg(&db_dir)
        .spawn()
        .map_err(|e| format!("打开目录失败: {}", e))?;
    Ok(())
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
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
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
