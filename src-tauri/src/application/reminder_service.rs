use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::domain::{NoteRepository, ReminderRepository, RepeatType};
use super::{lunar_calendar, window_manager};

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
        let note = match note_repo.find_by_id(&reminder.note_id) {
            Ok(Some(n)) => n,
            Ok(None) => {
                eprintln!("[调度器] 便签不存在: {}", reminder.note_id);
                continue;
            }
            Err(e) => {
                eprintln!("[调度器] 查询便签失败: {}", e);
                continue;
            }
        };

        // 归档便签不触发提醒
        if note.is_archived {
            eprintln!("[调度器] 便签已归档，跳过提醒: note_id={}", reminder.note_id);
            continue;
        }

        // 发送系统通知
        let title = if reminder.note_title.is_empty() {
            "便签提醒".to_string()
        } else {
            reminder.note_title.clone()
        };
        let summary: String = note.content.chars().take(80).collect();
        let body = if note.content.chars().count() > 80 {
            format!("{}...", summary)
        } else if summary.is_empty() {
            "点击查看便签".to_string()
        } else {
            summary
        };

        match app
            .notification()
            .builder()
            .title(&title)
            .body(&body)
            .extra("note_id", &reminder.note_id)
            .auto_cancel()
            .show()
        {
            Ok(_) => eprintln!("[调度器] 通知发送成功"),
            Err(e) => eprintln!("[调度器] 发送通知失败: {}", e),
        }

        // 弹出便签窗口（委托 window_manager）
        match window_manager::activate_note_for_reminder(app, &note, &reminder.id) {
            Ok(_) => {}
            Err(e) => eprintln!("[调度器] 弹出便签窗口失败: {}", e),
        }

        // 更新状态（经 domain 方法 + save）
        if reminder.is_repeating() {
            let mut updated = reminder.clone();
            if updated.reset_for_next_trigger() {
                // Daily/Weekly/Monthly：domain 层计算下次触发
                if let Err(e) = reminder_repo.save(&updated) {
                    eprintln!("[调度器] 更新周期提醒失败: {}", e);
                }
            } else if reminder.repeat_type == RepeatType::LunarMonthly {
                // LunarMonthly：domain 层无法计算，由 application 层调用农历库
                match lunar_calendar::lunar_next_month(&updated.remind_at) {
                    Some(next_time) => {
                        updated.remind_at = next_time;
                        if let Err(e) = reminder_repo.save(&updated) {
                            eprintln!("[调度器] 更新农历周期提醒失败: {}", e);
                        }
                    }
                    None => {
                        eprintln!("[调度器] 农历计算失败，标记为已触发: {}", updated.remind_at);
                        updated.mark_triggered();
                        if let Err(e) = reminder_repo.save(&updated) {
                            eprintln!("[调度器] 标记提醒已触发失败: {}", e);
                        }
                    }
                }
            } else {
                updated.mark_triggered();
                if let Err(e) = reminder_repo.save(&updated) {
                    eprintln!("[调度器] 标记提醒已触发失败: {}", e);
                }
            }
        } else {
            let mut updated = reminder.clone();
            updated.mark_triggered();
            if let Err(e) = reminder_repo.save(&updated) {
                eprintln!("[调度器] 标记提醒已触发失败: {}", e);
            }
        }
    }
}
