//! 便签命令：CRUD、归档/恢复、搜索、标签、窗口状态、批量操作。
//!
//! 命令层为薄壳：业务编排下沉到 `note_service`，命令仅负责调用 service +
//! 执行 Tauri 副作用（emit / schedule_recalc / window_manager）。
//!
//! schedule_auto_sync 已下沉到 service 层 emit 事件，由 lib.rs 监听器统一处理（ADR-007）。

use tauri::{AppHandle, Emitter, State};

use crate::domain::Note;
use crate::AppState;

use super::super::{note_service, window_manager};

/// 新建便签并打开窗口
#[tauri::command]
pub async fn create_note(
    app: AppHandle,
    state: State<'_, AppState>,
    color: Option<String>,
) -> Result<String, String> {
    note_service::create_note(&app, state.note_repo.as_ref(), state.event_bus.as_ref(), color)
}

/// 通过便签 ID 激活/弹出便签窗口（Hub 便签列表点击时调用）
#[tauri::command]
pub async fn activate_note_by_id(
    app: AppHandle,
    state: State<'_, AppState>,
    note_id: String,
) -> Result<(), String> {
    let note = state.note_repo.find_by_id(&note_id)?
        .ok_or("便签不存在")?;
    super::super::window_manager::open_note_window(&app, &note)
}

/// 获取便签详情
#[tauri::command]
pub async fn get_note(state: State<'_, AppState>, id: String) -> Result<Option<Note>, String> {
    state.note_repo.find_by_id(&id)
}

/// 获取全部便签
#[tauri::command]
pub async fn get_all_notes(state: State<'_, AppState>) -> Result<Vec<Note>, String> {
    state.note_repo.find_all()
}

/// 打开便签窗口（从归档列表等场景调用）
#[tauri::command]
pub async fn open_note(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    note_service::open_note(&app, state.note_repo.as_ref(), &id)
}

/// 打开便签窗口并附带一个 flag（如 "reminder" 自动打开提醒面板）
#[tauri::command]
pub async fn open_note_with_flag(app: AppHandle, state: State<'_, AppState>, id: String, flag: String) -> Result<(), String> {
    note_service::open_note_with_flag(&app, state.note_repo.as_ref(), &id, &flag)
}

/// 更新便签内容
#[tauri::command]
pub async fn update_note_content(app: AppHandle, state: State<'_, AppState>, id: String, content: String) -> Result<(), String> {
    let _ = app;
    note_service::update_note_content(state.note_repo.as_ref(), state.event_bus.as_ref(), &id, content)
}

/// 更新便签标题
#[tauri::command]
pub async fn update_note_title(app: AppHandle, state: State<'_, AppState>, id: String, title: String) -> Result<(), String> {
    let _ = app;
    note_service::update_note_title(state.note_repo.as_ref(), state.event_bus.as_ref(), &id, title)
}

/// 更新便签样式（颜色、透明度、置顶）
#[tauri::command]
pub async fn update_note_style(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    color: String,
    opacity: f64,
    is_pinned: bool,
) -> Result<(), String> {
    note_service::update_note_style(&app, state.note_repo.as_ref(), state.event_bus.as_ref(), &id, color, opacity, is_pinned)
}

/// 更新窗口位置和尺寸
#[tauri::command]
pub async fn update_note_window_state(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    pos_x: i32,
    pos_y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let _ = app;
    note_service::update_note_window_state(state.note_repo.as_ref(), state.event_bus.as_ref(), &id, pos_x, pos_y, width, height)
}

/// 删除便签（同时删除关联提醒 + 关闭窗口）
#[tauri::command]
pub async fn delete_note(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    // 删除前清理便签中的图片文件
    if let Ok(Some(note)) = state.note_repo.find_by_id(&id) {
        super::super::image_service::cleanup_removed_images(&note.content, "");
    }
    note_service::delete_note(state.note_repo.as_ref(), state.reminder_repo.as_ref(), state.event_bus.as_ref(), &id)?;
    // 删除成功后关闭便签窗口（destroy 强制销毁，避免 close 不可靠）
    window_manager::close_note_window(&app, &id);
    state.scheduler.schedule_recalc();
    Ok(())
}

/// 恢复便签窗口的置顶状态为便签自身的 is_pinned 值
/// 用于提醒触发时临时置顶后，用户操作横幅后恢复原始状态
#[tauri::command]
pub async fn restore_window_on_top(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let note = state.note_repo.find_by_id(&id)?.ok_or("便签不存在")?;
    window_manager::restore_note_on_top(&app, &note);
    Ok(())
}

#[tauri::command]
pub async fn archive_note(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    note_service::archive_note(state.note_repo.as_ref(), state.event_bus.as_ref(), &id)?;
    let _ = app.emit("note-archived", &id);
    Ok(())
}

/// 取消归档
#[tauri::command]
pub async fn unarchive_note(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    note_service::unarchive_note(state.note_repo.as_ref(), state.event_bus.as_ref(), &id)?;
    let _ = app.emit("note-unarchived", &id);
    Ok(())
}

/// 获取已归档的便签列表
#[tauri::command]
pub async fn get_archived_notes(state: State<'_, AppState>) -> Result<Vec<Note>, String> {
    state.note_repo.find_archived()
}

/// 搜索便签（标题 + 内容 + 标签）
#[tauri::command]
pub async fn search_notes(state: State<'_, AppState>, query: String) -> Result<Vec<Note>, String> {
    state.note_repo.search_notes(&query)
}

/// 更新便签标签
#[tauri::command]
pub async fn update_note_tags(app: AppHandle, state: State<'_, AppState>, id: String, tags: Vec<String>) -> Result<(), String> {
    let _ = app;
    note_service::update_note_tags(state.note_repo.as_ref(), state.event_bus.as_ref(), &id, tags)
}

// ============ 批量操作命令 ============

/// 批量归档便签
#[tauri::command]
pub async fn batch_archive_notes(app: AppHandle, state: State<'_, AppState>, ids: Vec<String>) -> Result<usize, String> {
    let succeeded = note_service::batch_archive(state.note_repo.as_ref(), state.event_bus.as_ref(), &ids)?;
    for id in &succeeded {
        let _ = app.emit("note-archived", id);
    }
    Ok(succeeded.len())
}

/// 批量恢复便签（从归档状态恢复）
#[tauri::command]
pub async fn batch_unarchive_notes(app: AppHandle, state: State<'_, AppState>, ids: Vec<String>) -> Result<usize, String> {
    let succeeded = note_service::batch_unarchive(state.note_repo.as_ref(), state.event_bus.as_ref(), &ids)?;
    for id in &succeeded {
        let _ = app.emit("note-unarchived", id);
    }
    Ok(succeeded.len())
}

/// 批量删除便签（同时关闭对应窗口）
#[tauri::command]
pub async fn batch_delete_notes(app: AppHandle, state: State<'_, AppState>, ids: Vec<String>) -> Result<usize, String> {
    let succeeded = note_service::batch_delete(
        state.note_repo.as_ref(),
        state.reminder_repo.as_ref(),
        state.event_bus.as_ref(),
        &ids,
    )?;
    for id in &succeeded {
        // destroy 强制销毁窗口（close 在 onCloseRequested 注册后不可靠）
        window_manager::close_note_window(&app, id);
    }
    state.scheduler.schedule_recalc();
    Ok(succeeded.len())
}

/// 批量修改便签颜色
#[tauri::command]
pub async fn batch_update_color(app: AppHandle, state: State<'_, AppState>, ids: Vec<String>, color: String) -> Result<usize, String> {
    let succeeded = note_service::batch_update_color(state.note_repo.as_ref(), state.event_bus.as_ref(), &ids, color.clone())?;
    for id in &succeeded {
        let _ = app.emit("note-color-changed", serde_json::json!({ "id": id, "color": color }));
    }
    Ok(succeeded.len())
}
