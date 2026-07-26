//! 提醒命令：CRUD、贪睡、关闭，以及日历视图辅助数据（农历 + 便签活动）。
//!
//! schedule_auto_sync 已下沉到 service 层 emit 事件，由 lib.rs 监听器统一处理（ADR-007）。

use tauri::{AppHandle, Emitter, State};

use crate::domain::Reminder;
use crate::AppState;

use super::super::reminder_service;
use super::super::event_names::REMINDER_CHANGED;

/// 创建提醒
#[tauri::command]
pub async fn create_reminder(
    app: AppHandle,
    state: State<'_, AppState>,
    note_id: String,
    note_title: String,
    remind_at: String,
    repeat_type: String,
) -> Result<Reminder, String> {
    let note_id_for_emit = note_id.clone();
    let reminder = reminder_service::create_reminder(
        state.reminder_repo.as_ref(),
        state.event_bus.as_ref(),
        note_id,
        note_title,
        remind_at,
        repeat_type,
    )?;
    // schedule_recalc 由 lib.rs 监听 ReminderWritten 事件触发（ADR-008 扩展）
    let _ = app.emit(REMINDER_CHANGED, &note_id_for_emit);
    Ok(reminder)
}

/// 获取便签的提醒列表
#[tauri::command]
pub async fn get_reminders(state: State<'_, AppState>, note_id: String) -> Result<Vec<Reminder>, String> {
    state.reminder_repo.find_by_note_id(&note_id)
}

/// 贪睡提醒（延长 N 分钟）
#[tauri::command]
pub async fn snooze_reminder(app: AppHandle, state: State<'_, AppState>, id: String, minutes: i64) -> Result<(), String> {
    let note_id = reminder_service::snooze_reminder(state.reminder_repo.as_ref(), state.event_bus.as_ref(), &id, minutes)?;
    // schedule_recalc 由 lib.rs 监听 ReminderWritten 事件触发（ADR-008 扩展）
    let _ = app.emit(REMINDER_CHANGED, &note_id);
    Ok(())
}

/// 关闭提醒（标记为已完成）
#[tauri::command]
pub async fn dismiss_reminder(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let note_id = reminder_service::dismiss_reminder(state.reminder_repo.as_ref(), state.event_bus.as_ref(), &id)?;
    // schedule_recalc 由 lib.rs 监听 ReminderWritten 事件触发（ADR-008 扩展）
    let _ = app.emit(REMINDER_CHANGED, &note_id);
    Ok(())
}

/// 删除提醒
#[tauri::command]
pub async fn delete_reminder(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let note_id = reminder_service::delete_reminder(state.reminder_repo.as_ref(), state.event_bus.as_ref(), &id)?;
    // schedule_recalc 由 lib.rs 监听 ReminderWritten 事件触发（ADR-008 扩展）
    if let Some(nid) = note_id {
        let _ = app.emit(REMINDER_CHANGED, &nid);
    }
    Ok(())
}

/// 按月份查询提醒（日历视图用，含所有状态）
#[tauri::command]
pub async fn get_reminders_by_month(state: State<'_, AppState>, year: i32, month: u32) -> Result<Vec<Reminder>, String> {
    let start_iso = super::super::month_range::month_start_iso(year, month)?;
    let end_iso = super::super::month_range::month_end_iso(year, month)?;
    state.reminder_query.find_by_date_range(&start_iso, &end_iso)
}

/// 查询月份内每天的农历日文本（日历视图用）
#[derive(serde::Serialize)]
pub struct LunarDateInfo {
    day: u32,
    lunar_text: String,
}

#[tauri::command]
pub async fn get_lunar_dates(_state: State<'_, AppState>, year: i32, month: u32) -> Result<Vec<LunarDateInfo>, String> {
    let days_in_month = super::super::month_range::days_in_month(year, month)?;

    let mut result = Vec::new();
    for day in 1..=days_in_month {
        let lunar_text = super::super::lunar_calendar::lunar_date_text(year as isize, month as usize, day as usize);
        result.push(LunarDateInfo { day, lunar_text });
    }
    Ok(result)
}

/// 查询月份内有便签活动的日期（日历视图用）
#[tauri::command]
pub async fn get_notes_activity_by_month(state: State<'_, AppState>, year: i32, month: u32) -> Result<Vec<u32>, String> {
    state.note_query.find_activity_by_month(year, month)
}
