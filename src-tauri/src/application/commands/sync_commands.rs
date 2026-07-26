//! Git 同步命令：配置读写、执行同步、git 可用性检查。
//!
//! 调用方：前端 api.ts 通过 invoke 调用。
//! 依赖：AppState.git_sync / git_ops / git_sync::sync_with_notification。

use tauri::{AppHandle, State};

use crate::AppState;

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
    super::super::git_sync::sync_with_notification(
        &app,
        &state.git_sync,
        state.note_repo.as_ref(),
        state.reminder_repo.as_ref(),
        state.template_repo.as_ref(),
        create_branch.unwrap_or(false),
    )
}

/// 检查 git 是否已安装
#[tauri::command]
pub async fn check_git() -> bool {
    super::super::git_ops::check_git_installed()
}
