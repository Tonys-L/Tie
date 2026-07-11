use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tokio::sync::Notify;
use tokio::time::{Instant, sleep_until};

use super::reminder_service;

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
    reminder_service::fire_reminders(app, state.note_repo.as_ref(), state.reminder_repo.as_ref());
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
