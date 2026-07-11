use tauri::{AppHandle, State};

use crate::domain::{Note, Reminder};
use crate::AppState;

use super::note_service;

// ============ Note 命令 ============

/// 新建便签并打开窗口
#[tauri::command]
pub async fn create_note(
    app: AppHandle,
    state: State<'_, AppState>,
    color: Option<String>,
) -> Result<String, String> {
    let result = note_service::create_note(&app, state.note_repo.as_ref(), color);
    state.git_sync.schedule_auto_sync(app);
    result
}

/// 获取便签详情
#[tauri::command]
pub fn get_note(state: State<AppState>, id: String) -> Result<Option<Note>, String> {
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
    let result = note_service::open_note(&app, state.note_repo.as_ref(), &id);
    state.git_sync.schedule_auto_sync(app);
    result
}

/// 打开便签窗口并附带一个 flag（如 "reminder" 自动打开提醒面板）
#[tauri::command]
pub fn open_note_with_flag(app: AppHandle, state: State<AppState>, id: String, flag: String) -> Result<(), String> {
    let result = note_service::open_note_with_flag(&app, state.note_repo.as_ref(), &id, &flag);
    state.git_sync.schedule_auto_sync(app);
    result
}

/// 更新便签内容
#[tauri::command]
pub fn update_note_content(app: AppHandle, state: State<AppState>, id: String, content: String) -> Result<(), String> {
    let mut note = state.note_repo.find_by_id(&id)?.ok_or("便签不存在")?;
    note.update_content(content);
    let result = state.note_repo.save(&note);
    state.git_sync.schedule_auto_sync(app);
    result
}

/// 更新便签标题
#[tauri::command]
pub fn update_note_title(app: AppHandle, state: State<AppState>, id: String, title: String) -> Result<(), String> {
    let mut note = state
        .note_repo
        .find_by_id(&id)?
        .ok_or("便签不存在")?;
    note.update_title(title);
    let result = state.note_repo.save(&note);
    state.git_sync.schedule_auto_sync(app);
    result
}

/// 更新便签样式（颜色、透明度、置顶）
#[tauri::command]
pub fn update_note_style(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    color: String,
    opacity: f64,
    is_pinned: bool,
) -> Result<(), String> {
    let result = note_service::update_note_style(&app, state.note_repo.as_ref(), &id, color, opacity, is_pinned);
    state.git_sync.schedule_auto_sync(app);
    result
}

/// 更新窗口位置和尺寸
#[tauri::command]
pub fn update_note_window_state(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    pos_x: i32,
    pos_y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let mut note = state.note_repo.find_by_id(&id)?.ok_or("便签不存在")?;
    note.update_window_state(pos_x, pos_y, width, height);
    let result = state.note_repo.save(&note);
    state.git_sync.schedule_auto_sync(app);
    result
}

/// 删除便签（同时删除关联提醒）
#[tauri::command]
pub async fn delete_note(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let result = note_service::delete_note(state.note_repo.as_ref(), state.reminder_repo.as_ref(), &id);
    state.scheduler.schedule_recalc();
    state.git_sync.schedule_auto_sync(app);
    result
}

/// 归档便签
#[tauri::command]
pub async fn archive_note(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut note = state.note_repo.find_by_id(&id)?.ok_or("便签不存在")?;
    note.archive();
    let result = state.note_repo.save(&note);
    state.git_sync.schedule_auto_sync(app);
    result
}

/// 取消归档
#[tauri::command]
pub async fn unarchive_note(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut note = state.note_repo.find_by_id(&id)?.ok_or("便签不存在")?;
    note.unarchive();
    let result = state.note_repo.save(&note);
    state.git_sync.schedule_auto_sync(app);
    result
}

/// 获取已归档的便签列表
#[tauri::command]
pub async fn get_archived_notes(state: State<'_, AppState>) -> Result<Vec<Note>, String> {
    state.note_repo.find_archived()
}

// ============ Reminder 命令 ============

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
    let reminder = Reminder::new(note_id, note_title, remind_at, repeat_type);
    state.reminder_repo.save(&reminder)?;
    state.scheduler.schedule_recalc();
    state.git_sync.schedule_auto_sync(app);
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
    let mut reminder = state.reminder_repo.find_by_id(&id)?.ok_or("提醒不存在")?;
    reminder.snooze(minutes);
    let result = state.reminder_repo.save(&reminder);
    state.scheduler.schedule_recalc();
    state.git_sync.schedule_auto_sync(app);
    result
}

/// 关闭提醒（标记为已完成）
#[tauri::command]
pub async fn dismiss_reminder(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut reminder = state.reminder_repo.find_by_id(&id)?.ok_or("提醒不存在")?;
    reminder.mark_done();
    let result = state.reminder_repo.save(&reminder);
    state.scheduler.schedule_recalc();
    state.git_sync.schedule_auto_sync(app);
    result
}

/// 删除提醒
#[tauri::command]
pub async fn delete_reminder(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let result = state.reminder_repo.delete(&id);
    state.scheduler.schedule_recalc();
    state.git_sync.schedule_auto_sync(app);
    result
}

// ============ 同步命令 ============

/// 获取同步配置
#[tauri::command]
pub async fn get_sync_config(state: State<'_, AppState>) -> Result<super::sync_config::SyncConfig, String> {
    state.git_sync.load_config()
}

/// 保存同步配置
#[tauri::command]
pub async fn save_sync_config(state: State<'_, AppState>, config: super::sync_config::SyncConfig) -> Result<(), String> {
    state.git_sync.save_config(&config)
}

/// 执行同步（导出JSON → git commit/fetch/merge → 导入JSON → push）
#[tauri::command]
pub async fn sync_notes(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    eprintln!("[同步] 开始执行同步...");
    let result = note_service::sync_notes(state.note_repo.as_ref(), state.reminder_repo.as_ref(), &state.git_sync);
    eprintln!("[同步] 同步完成: {:?}", result);
    use tauri_plugin_notification::NotificationExt;
    match &result {
        Ok(msg) => { let _ = app.notification().builder().title(super::locale_manager::notify_sync_ok()).body(msg).show(); }
        Err(e) => { let _ = app.notification().builder().title(super::locale_manager::notify_sync_fail()).body(e).show(); }
    }
    result
}

/// 检查 git 是否已安装
#[tauri::command]
pub fn check_git() -> bool {
    super::git_ops::check_git_installed()
}

// ============ 快捷键命令 ============

/// 获取快捷键配置
#[tauri::command]
pub fn get_shortcut_config(state: State<AppState>) -> super::shortcut_manager::ShortcutConfig {
    state.shortcut_manager.get_config()
}

/// 保存快捷键配置并重新注册
#[tauri::command]
pub fn save_shortcut_config(
    app: AppHandle,
    state: State<AppState>,
    config: super::shortcut_manager::ShortcutConfig,
) -> Result<(), String> {
    state.shortcut_manager.save_and_reregister(&app, config)
}

// ============ 国际化命令 ============

/// 设置语言并重建托盘菜单
#[tauri::command]
pub fn set_locale(app: AppHandle, locale: String) -> Result<(), String> {
    let code = if locale == "en" { 1u8 } else { 0u8 };
    super::locale_manager::set_locale_code(code);
    super::tray_manager::rebuild_tray_menu(&app)
}
