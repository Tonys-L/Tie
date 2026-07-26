//! 模板命令：CRUD + 从模板创建便签。
//!
//! 命令层为薄壳：业务编排下沉到 `template_service`。
//! schedule_auto_sync 已下沉到 service 层 emit 事件，由 lib.rs 监听器统一处理（ADR-007）。

use tauri::{AppHandle, State};

use crate::domain::Template;
use crate::AppState;

use super::super::template_service;

/// 查询所有模板
#[tauri::command]
pub async fn get_templates(state: State<'_, AppState>) -> Result<Vec<Template>, String> {
    state.template_repo.find_all()
}

/// 保存模板（新增或更新）
#[tauri::command]
pub async fn save_template(state: State<'_, AppState>, template: Template) -> Result<(), String> {
    template_service::save_template(state.template_repo.as_ref(), state.event_bus.as_ref(), &template)
}

/// 删除模板
#[tauri::command]
pub async fn delete_template(state: State<'_, AppState>, id: String) -> Result<(), String> {
    template_service::delete_template(state.template_repo.as_ref(), state.event_bus.as_ref(), &id)
}

/// 从模板创建便签（返回新便签 ID）
#[tauri::command]
pub async fn create_note_from_template(app: AppHandle, state: State<'_, AppState>, template_id: String) -> Result<String, String> {
    template_service::create_note_from_template(
        &app,
        state.note_repo.as_ref(),
        state.template_repo.as_ref(),
        state.event_bus.as_ref(),
        &template_id,
    )
}
