use tauri::{AppHandle, State};
use chrono::Datelike;

use crate::domain::{Note, Reminder};
use crate::AppState;

use super::{note_service, window_manager};

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

/// 通过便签 ID 激活/弹出便签窗口（Hub 便签列表点击时调用）
#[tauri::command]
pub async fn activate_note_by_id(
    app: AppHandle,
    state: State<'_, AppState>,
    note_id: String,
) -> Result<(), String> {
    let note = state.note_repo.find_by_id(&note_id)?
        .ok_or("便签不存在")?;
    window_manager::open_note_window(&app, &note)
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
    let result = note_service::open_note(&app, state.note_repo.as_ref(), &id);
    state.git_sync.schedule_auto_sync(app);
    result
}

/// 打开便签窗口并附带一个 flag（如 "reminder" 自动打开提醒面板）
#[tauri::command]
pub async fn open_note_with_flag(app: AppHandle, state: State<'_, AppState>, id: String, flag: String) -> Result<(), String> {
    let result = note_service::open_note_with_flag(&app, state.note_repo.as_ref(), &id, &flag);
    state.git_sync.schedule_auto_sync(app);
    result
}

/// 更新便签内容
#[tauri::command]
pub async fn update_note_content(app: AppHandle, state: State<'_, AppState>, id: String, content: String) -> Result<(), String> {
    let mut note = state.note_repo.find_by_id(&id)?.ok_or("便签不存在")?;
    let old_content = note.content.clone();
    // 清理被删除的图片文件
    cleanup_removed_images(&old_content, &content);
    note.update_content(content);
    let result = state.note_repo.save(&note);
    state.git_sync.schedule_auto_sync(app);
    result
}

/// 更新便签标题
#[tauri::command]
pub async fn update_note_title(app: AppHandle, state: State<'_, AppState>, id: String, title: String) -> Result<(), String> {
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
pub async fn update_note_style(
    app: AppHandle,
    state: State<'_, AppState>,
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
pub async fn update_note_window_state(
    app: AppHandle,
    state: State<'_, AppState>,
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
    // 删除前清理便签中的图片文件
    if let Ok(Some(note)) = state.note_repo.find_by_id(&id) {
        cleanup_removed_images(&note.content, "");
    }
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

/// 搜索便签（标题 + 内容 + 标签）
#[tauri::command]
pub async fn search_notes(state: State<'_, AppState>, query: String) -> Result<Vec<Note>, String> {
    state.note_repo.search_notes(&query)
}

/// 更新便签标签
#[tauri::command]
pub async fn update_note_tags(app: AppHandle, state: State<'_, AppState>, id: String, tags: Vec<String>) -> Result<(), String> {
    let mut note = state.note_repo.find_by_id(&id)?.ok_or("便签不存在")?;
    note.set_tags(tags);
    let result = state.note_repo.save(&note);
    state.git_sync.schedule_auto_sync(app);
    result
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

/// 按月份查询提醒（日历视图用，含所有状态）
#[tauri::command]
pub async fn get_reminders_by_month(state: State<'_, AppState>, year: i32, month: u32) -> Result<Vec<Reminder>, String> {
    let start = chrono::NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or("无效的年月")?;
    let end = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .ok_or("无效的年月")?;
    let start_iso = start.and_hms_opt(0, 0, 0).unwrap().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let end_iso = end.and_hms_opt(0, 0, 0).unwrap().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    state.reminder_repo.find_by_date_range(&start_iso, &end_iso)
}

/// 查询月份内每天的农历日文本（日历视图用）
#[derive(serde::Serialize)]
pub struct LunarDateInfo {
    day: u32,
    lunar_text: String,
}

#[tauri::command]
pub async fn get_lunar_dates(_state: State<'_, AppState>, year: i32, month: u32) -> Result<Vec<LunarDateInfo>, String> {
    use tyme4rs::tyme::solar::SolarDay;
    use tyme4rs::tyme::Culture;

    let days_in_month = {
        let next = if month == 12 {
            chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
        };
        next.ok_or("无效年月")?.pred_opt().ok_or("无效年月")?.day()
    };

    let mut result = Vec::new();
    for day in 1..=days_in_month {
        let solar = SolarDay::from_ymd(year as isize, month as usize, day as usize);
        let lunar_day = solar.get_lunar_day();
        let is_first = lunar_day.get_day() == 1;
        let lunar_text = if is_first {
            format!("{}{}", lunar_day.get_lunar_month().get_name(), lunar_day.get_name())
        } else {
            lunar_day.get_name()
        };
        result.push(LunarDateInfo { day, lunar_text });
    }
    Ok(result)
}

/// 查询月份内有便签活动的日期（日历视图用）
#[tauri::command]
pub async fn get_notes_activity_by_month(state: State<'_, AppState>, year: i32, month: u32) -> Result<Vec<u32>, String> {
    state.note_repo.find_activity_by_month(year, month)
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
pub async fn sync_notes(app: AppHandle, state: State<'_, AppState>, create_branch: Option<bool>) -> Result<String, String> {
    eprintln!("[同步] 开始执行同步... create_branch={:?}", create_branch);
    let result = note_service::sync_notes(state.note_repo.as_ref(), state.reminder_repo.as_ref(), &state.git_sync, create_branch.unwrap_or(false));
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
pub async fn check_git() -> bool {
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

// ============ 图片存储命令 ============

/// 从内容中提取所有图片文件名
/// 匹配格式：img:uuid.png
fn extract_image_filenames(content: &str) -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();
    // 查找所有 img:xxx.ext 模式
    for part in content.split("img:").skip(1) {
        // 取到第一个空白或 ) 或 ] 为止
        let filename: String = part.chars()
            .take_while(|c| !c.is_whitespace() && *c != ')' && *c != ']' && *c != '(')
            .collect();
        if !filename.is_empty() {
            let lower = filename.to_lowercase();
            if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")
                || lower.ends_with(".gif") || lower.ends_with(".webp") || lower.ends_with(".bmp")
            {
                names.insert(filename);
            }
        }
    }
    names
}

/// 对比新旧内容，删除不再被引用的图片文件
fn cleanup_removed_images(old_content: &str, new_content: &str) {
    let old_images = extract_image_filenames(old_content);
    let new_images = extract_image_filenames(new_content);

    // 找出被移除的图片
    let removed: Vec<_> = old_images.difference(&new_images).collect();
    if removed.is_empty() {
        return;
    }

    if let Ok(dir) = image_dir() {
        for filename in removed {
            let filepath = dir.join(filename);
            if let Err(e) = std::fs::remove_file(&filepath) {
                eprintln!("[图片清理] 删除失败: {}, 文件: {:?}", e, filepath);
            } else {
                eprintln!("[图片清理] 已删除: {}", filename);
            }
        }
    }
}

/// 获取图片存储目录
fn image_dir() -> Result<std::path::PathBuf, String> {
    let dir = std::env::current_exe()
        .map_err(|e| format!("获取 exe 路径失败: {}", e))?
        .parent()
        .ok_or("无法获取父目录")?
        .join("data")
        .join("sync")
        .join("images");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建图片目录失败: {}", e))?;
    Ok(dir)
}

/// 保存图片文件，返回文件名（如 "uuid.png"）
#[tauri::command]
pub fn save_image(data: Vec<u8>, ext: String) -> Result<String, String> {
    let allowed = ["png", "jpg", "jpeg", "gif", "webp", "bmp"];
    if !allowed.contains(&ext.as_str()) {
        return Err(format!("不支持的图片格式: {}", ext));
    }
    let dir = image_dir()?;
    let id = uuid::Uuid::new_v4().to_string();
    let filename = format!("{}.{}", id, ext);
    std::fs::write(dir.join(&filename), &data).map_err(|e| format!("保存图片失败: {}", e))?;
    Ok(filename)
}

/// 获取图片目录完整路径，前端用于拼接 convertFileSrc
#[tauri::command]
pub fn get_image_dir() -> Result<String, String> {
    let dir = image_dir()?;
    dir.to_str()
        .map(|s| s.to_string())
        .ok_or("路径转换失败".to_string())
}
