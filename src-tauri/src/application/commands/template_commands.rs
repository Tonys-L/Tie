//! 模板命令：CRUD + 从模板创建便签。

use tauri::{AppHandle, State};

use crate::domain::{Note, Template};
use crate::AppState;

/// 查询所有模板
#[tauri::command]
pub async fn get_templates(state: State<'_, AppState>) -> Result<Vec<Template>, String> {
    state.template_repo.find_all()
}

/// 保存模板（新增或更新）
#[tauri::command]
pub async fn save_template(state: State<'_, AppState>, template: Template) -> Result<(), String> {
    state.template_repo.save(&template)
}

/// 删除模板
#[tauri::command]
pub async fn delete_template(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.template_repo.delete(&id)
}

/// 从模板创建便签（返回新便签 ID）
#[tauri::command]
pub async fn create_note_from_template(app: AppHandle, state: State<'_, AppState>, template_id: String) -> Result<String, String> {
    let template = state.template_repo.find_by_id(&template_id)?
        .ok_or_else(|| format!("模板不存在: {}", template_id))?;
    // 创建便签并写入模板内容
    let mut note = Note::new(template.name.clone(), "amber".to_string());
    note.update_content(template.content);
    state.note_repo.save(&note)?;
    // 打开便签窗口
    super::super::window_manager::open_note_window(&app, &note)?;
    state.git_sync.schedule_auto_sync(app);
    Ok(note.id)
}
