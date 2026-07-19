use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::Notify;
use tokio::time::{Instant, sleep_until};

use crate::domain::{NoteRepository, ReminderRepository, RepeatType};
use super::{lunar_calendar, window_manager};

/// 提醒调度器：事件驱动 + 单定时器
///
/// 核心机制：
/// - 维护一个 Arc<Notify>，提醒数据变更时通知调度器重新计算定时器
/// - tokio::select! 同时等待定时器到期和 Notify 通知
/// - 定时器到期 → fire_reminders → 重新计算下一次
/// - Notify 被触发 → 重新计算定时器（可能更早）
pub struct ReminderScheduler {
    notify: Arc<Notify>,
}

impl ReminderScheduler {
    pub fn new() -> Self {
        Self {
            notify: Arc::new(Notify::new()),
        }
    }

    /// 通知调度器重新计算定时器（提醒数据变更时调用）
    pub fn schedule_recalc(&self) {
        self.notify.notify_one();
    }

    /// 获取 Notify 的 Arc 引用（供调度循环使用）
    pub fn notify(&self) -> Arc<Notify> {
        self.notify.clone()
    }
}

/// 启动调度器循环
pub fn start(app: AppHandle) {
    let notify = app.state::<crate::AppState>().scheduler.notify();

    tauri::async_runtime::spawn(async move {
        // 启动后等待 5 秒再开始，避免与初始化竞争
        tokio::time::sleep(Duration::from_secs(5)).await;

        loop {
            let next_time = {
                let state = app.state::<crate::AppState>();
                state.reminder_repo.find_next_due_time()
            };

            let deadline = match &next_time {
                Ok(Some(t)) => parse_instant(t),
                _ => {
                    // 没有到期提醒，等待 Notify 唤醒
                    eprintln!("[调度器] 无到期提醒，等待新提醒...");
                    notify.notified().await;
                    continue;
                }
            };

            eprintln!("[调度器] 下次到期: {:?}", deadline);

            // 等待：定时器到期 或 被通知重新计算
            tokio::select! {
                _ = sleep_until(deadline) => {
                    check_and_fire(&app);
                }
                _ = notify.notified() => {
                    eprintln!("[调度器] 收到重新计算通知");
                    // 不 fire，回到循环顶部重新计算
                }
            }
        }
    });
}

fn check_and_fire(app: &AppHandle) {
    let state = app.state::<crate::AppState>();
    fire_reminders(app, state.note_repo.as_ref(), state.reminder_repo.as_ref());
}

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

/// 将 ISO 时间字符串转为 tokio Instant
fn parse_instant(iso_time: &str) -> Instant {
    let target = chrono::DateTime::parse_from_rfc3339(iso_time)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    let now = chrono::Utc::now();
    let duration = if target > now {
        (target - now).to_std().unwrap_or(Duration::from_millis(100))
    } else {
        // 已到期，立即触发
        Duration::from_millis(100)
    };

    Instant::now() + duration
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_instant_future_time() {
        // 未来 10 秒的时间
        let future = (chrono::Utc::now() + chrono::Duration::seconds(10)).to_rfc3339();
        let instant = parse_instant(&future);
        // Instant 应该在未来（大于当前 Instant）
        assert!(instant > Instant::now());
        // 但不超过 10 秒（允许微小误差）
        assert!(instant <= Instant::now() + Duration::from_secs(11));
    }

    #[test]
    fn test_parse_instant_past_time() {
        // 过去的时间 → 应立即触发（约 100ms 后）
        let past = "2020-01-01T00:00:00Z";
        let instant = parse_instant(past);
        // 应该非常接近现在（100ms 内）
        let now = Instant::now();
        assert!(instant >= now);
        assert!(instant <= now + Duration::from_millis(500));
    }

    #[test]
    fn test_parse_instant_invalid_format() {
        // 无效格式 → fallback 到 now → 立即触发
        let instant = parse_instant("not-a-date");
        let now = Instant::now();
        assert!(instant >= now);
        assert!(instant <= now + Duration::from_millis(500));
    }

    #[test]
    fn test_parse_instant_with_timezone() {
        // 带时区偏移的时间（+08:00）
        let future_local = (chrono::Utc::now() + chrono::Duration::seconds(5))
            .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
            .to_rfc3339();
        let instant = parse_instant(&future_local);
        assert!(instant > Instant::now());
        assert!(instant <= Instant::now() + Duration::from_secs(6));
    }

    #[test]
    fn test_parse_instant_far_future() {
        // 很远的未来（1 小时后）
        let far_future = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let instant = parse_instant(&far_future);
        assert!(instant > Instant::now());
        assert!(instant <= Instant::now() + Duration::from_secs(3601));
    }

    #[test]
    fn test_notify_schedule_recalc() {
        // 验证 ReminderScheduler 可正常创建和通知
        let scheduler = ReminderScheduler::new();
        let notify = scheduler.notify();
        // notify_one 不应阻塞
        scheduler.schedule_recalc();
        // 验证 Arc 引用计数正确
        assert_eq!(Arc::strong_count(&notify), 2); // scheduler 内部 1 + notify 变量 1
    }
}
