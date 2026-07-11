use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::domain::{NoteRepository, ReminderRepository};
use super::window_manager;

/// 触发所有到期提醒（编排逻辑）
///
/// 查询到期提醒 → 发送系统通知 → 弹出便签窗口 → 更新提醒状态。
pub fn fire_reminders(
    app: &AppHandle,
    note_repo: &dyn NoteRepository,
    reminder_repo: &dyn ReminderRepository,
) {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    eprintln!("[调度器] 轮询, now={}", now);

    let due_reminders = match reminder_repo.find_due(&now) {
        Ok(r) => {
            eprintln!("[调度器] 查到 {} 条到期提醒", r.len());
            r
        }
        Err(e) => {
            eprintln!("[调度器] 查询到期提醒失败: {}", e);
            return;
        }
    };

    for reminder in due_reminders {
        eprintln!("[调度器] 处理提醒: id={} remind_at={} repeat={:?}",
            reminder.id, reminder.remind_at, reminder.repeat_type);

        // 查询便签内容用于通知显示
        let note_content = match note_repo.find_by_id(&reminder.note_id) {
            Ok(Some(note)) => {
                let summary: String = note.content.chars().take(80).collect();
                if note.content.chars().count() > 80 {
                    format!("{}...", summary)
                } else {
                    summary
                }
            }
            Ok(None) => String::new(),
            Err(e) => {
                eprintln!("[调度器] 查询便签内容失败: {}", e);
                String::new()
            }
        };

        // 发送系统通知
        let title = if reminder.note_title.is_empty() {
            "便签提醒".to_string()
        } else {
            reminder.note_title.clone()
        };
        let body = if note_content.is_empty() {
            "点击查看便签".to_string()
        } else {
            note_content
        };

        match app
            .notification()
            .builder()
            .title(&title)
            .body(&body)
            .show()
        {
            Ok(_) => eprintln!("[调度器] 通知发送成功"),
            Err(e) => eprintln!("[调度器] 发送通知失败: {}", e),
        }

        // 弹出便签窗口
        match note_repo.find_by_id(&reminder.note_id) {
            Ok(Some(note)) => {
                let url = format!("index.html?reminder=1");
                let label = format!("note-{}", reminder.note_id);

                // 先检查窗口是否已存在
                if app.get_webview_window(&label).is_some() {
                    // 窗口已存在 → 聚焦 + 发送事件让前端显示横幅
                    if let Some(win) = app.get_webview_window(&label) {
                        let _ = win.show();
                        let _ = win.set_focus();
                        let _ = win.set_always_on_top(true);
                        // 发送提醒事件让前端显示横幅
                        let _ = app.emit_to(&label, "reminder-triggered", ());
                        eprintln!("[调度器] 窗口已存在，发送 reminder-triggered 事件: note_id={}", reminder.note_id);
                        let win_clone = win.clone();
                        let is_pinned = note.is_pinned;
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(300));
                            let _ = win_clone.set_always_on_top(is_pinned);
                        });
                    }
                } else {
                    // 窗口不存在 → 创建新窗口（URL 带 reminder 参数）
                    match window_manager::open_note_window_with_url(app, &note, &url) {
                        Ok(_) => {
                            eprintln!("[调度器] 便签窗口已弹出: note_id={}", reminder.note_id);
                            if let Some(win) = app.get_webview_window(&label) {
                                let _ = win.show();
                                let _ = win.set_focus();
                                let _ = win.set_always_on_top(true);
                            }
                        }
                        Err(e) => eprintln!("[调度器] 弹出便签窗口失败: {}", e),
                    }
                }
            }
            Ok(None) => eprintln!("[调度器] 便签不存在: {}", reminder.note_id),
            Err(e) => eprintln!("[调度器] 查询便签失败: {}", e),
        }

        // 更新状态
        if reminder.is_repeating() {
            let mut updated = reminder.clone();
            if updated.reset_for_next_trigger() {
                if let Err(e) = reminder_repo.save(&updated) {
                    eprintln!("[调度器] 更新周期提醒失败: {}", e);
                }
            } else {
                let _ = reminder_repo.update_status(&reminder.id, "triggered");
            }
        } else {
            if let Err(e) = reminder_repo.update_status(&reminder.id, "triggered") {
                eprintln!("[调度器] 更新提醒状态失败: {}", e);
            }
        }
    }
}
