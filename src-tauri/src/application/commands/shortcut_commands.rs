//! 快捷键命令：配置读写 + 重新注册。
//!
//! 调用方：前端 api.ts 通过 invoke 调用。
//! 依赖：AppState.shortcut_manager。

use tauri::{AppHandle, State};

use crate::AppState;

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
