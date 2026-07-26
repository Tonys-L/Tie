//! 国际化命令：语言切换 + 重建托盘菜单。
//!
//! 调用方：前端 api.ts 通过 invoke 调用。
//! 依赖：locale_manager + tray_manager。

use tauri::AppHandle;

/// 设置语言并重建托盘菜单
#[tauri::command]
pub fn set_locale(app: AppHandle, locale: String) -> Result<(), String> {
    let code = if locale == "en" { 1u8 } else { 0u8 };
    super::super::locale_manager::set_locale_code(code);
    super::super::tray_manager::rebuild_tray_menu(&app)
}
