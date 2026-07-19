use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::Notify;
use tokio::time::{Instant, sleep_until};

use crate::domain::reminder::{AdvanceResult, CalendarAdapter};
use crate::domain::{Note, NoteRepository, ReminderRepository};
use super::{lunar_calendar::TymeCalendarAdapter, window_manager};

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

/// 提醒通知器 trait：把"发送通知 + 弹出窗口"抽象为可注入接口
///
/// 设计目的：让 `fire_reminders_with_deps` 的核心逻辑可注入 mock 测试，
/// 脱离 Tauri AppHandle 依赖（INV-028）。
pub trait ReminderNotifier {
    /// 发送系统通知（标题 + 正文 + note_id）
    fn notify(&self, title: &str, body: &str, note_id: &str) -> Result<(), String>;
    /// 弹出便签窗口（委托 window_manager）
    fn activate_window(&self, note: &Note, reminder_id: &str) -> Result<(), String>;
}

/// `ReminderNotifier` 的 Tauri 实现：包装 AppHandle 调用系统通知 + window_manager
pub struct TauriReminderNotifier {
    app: AppHandle,
}

impl TauriReminderNotifier {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl ReminderNotifier for TauriReminderNotifier {
    fn notify(&self, title: &str, body: &str, note_id: &str) -> Result<(), String> {
        self.app
            .notification()
            .builder()
            .title(title)
            .body(body)
            .extra("note_id", note_id)
            .auto_cancel()
            .show()
            .map_err(|e| e.to_string())
    }

    fn activate_window(&self, note: &Note, reminder_id: &str) -> Result<(), String> {
        window_manager::activate_note_for_reminder(&self.app, note, reminder_id)
    }
}

/// 触发所有到期提醒（Tauri 入口包装）
///
/// 构造 `TauriReminderNotifier` + `TymeCalendarAdapter`，调用可测试的 `fire_reminders_with_deps`。
/// 外部调用方（check_and_fire）接口保持不变。
pub fn fire_reminders(
    app: &AppHandle,
    note_repo: &dyn NoteRepository,
    reminder_repo: &dyn ReminderRepository,
) {
    let notifier = TauriReminderNotifier::new(app.clone());
    let calendar = TymeCalendarAdapter;
    fire_reminders_with_deps(&notifier, &calendar, note_repo, reminder_repo);
}

/// 触发所有到期提醒（可测试入口）
///
/// 接收 trait object 而非 AppHandle，核心逻辑可注入 mock 测试（INV-028）。
/// 编排流程：查询到期提醒 → 发送通知 → 弹出窗口 → 推进状态 → save。
pub fn fire_reminders_with_deps(
    notifier: &dyn ReminderNotifier,
    calendar: &dyn CalendarAdapter,
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

        match notifier.notify(&title, &body, &reminder.note_id) {
            Ok(_) => eprintln!("[调度器] 通知发送成功"),
            Err(e) => eprintln!("[调度器] 发送通知失败: {}", e),
        }

        // 弹出便签窗口
        match notifier.activate_window(&note, &reminder.id) {
            Ok(_) => {}
            Err(e) => eprintln!("[调度器] 弹出便签窗口失败: {}", e),
        }

        // 推进状态：domain 层 advance_state 统一处理 Once/Daily/Weekly/Monthly/LunarMonthly
        let mut updated = reminder.clone();
        let result = updated.advance_state(calendar);
        match result {
            AdvanceResult::ResetToNext => {
                eprintln!("[调度器] 周期提醒已重置: id={} next={}", updated.id, updated.remind_at);
            }
            AdvanceResult::MarkedTriggered => {
                eprintln!("[调度器] 一次性提醒已标记触发: id={}", updated.id);
            }
        }
        if let Err(e) = reminder_repo.save(&updated) {
            eprintln!("[调度器] 保存提醒状态失败: {}", e);
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
    use crate::domain::reminder::{Reminder, ReminderStatus, RepeatType};
    use crate::domain::mock_repo::{InMemoryNoteRepository, InMemoryReminderRepository};
    use crate::domain::Note;

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

    // ============ fire_reminders_with_deps mock 测试 ============

    /// Mock 通知器：记录所有调用
    struct MockNotifier {
        notify_calls: std::sync::Mutex<Vec<(String, String, String)>>,
        activate_calls: std::sync::Mutex<Vec<(String, String)>>,
    }
    impl MockNotifier {
        fn new() -> Self {
            Self {
                notify_calls: std::sync::Mutex::new(vec![]),
                activate_calls: std::sync::Mutex::new(vec![]),
            }
        }
    }
    impl ReminderNotifier for MockNotifier {
        fn notify(&self, title: &str, body: &str, note_id: &str) -> Result<(), String> {
            self.notify_calls.lock().unwrap().push((
                title.to_string(),
                body.to_string(),
                note_id.to_string(),
            ));
            Ok(())
        }
        fn activate_window(&self, note: &Note, reminder_id: &str) -> Result<(), String> {
            self.activate_calls.lock().unwrap().push((
                note.id.clone(),
                reminder_id.to_string(),
            ));
            Ok(())
        }
    }

    /// Mock 农历适配器：返回固定值
    struct MockCalendar { next: Option<String> }
    impl CalendarAdapter for MockCalendar {
        fn lunar_next_month(&self, _iso: &str) -> Option<String> { self.next.clone() }
    }

    fn make_note(id: &str, title: &str, content: &str, archived: bool) -> Note {
        let mut note = Note::new(title.to_string(), content.to_string());
        note.id = id.to_string();
        note.is_archived = archived;
        note
    }

    fn make_due_reminder(id: &str, note_id: &str, note_title: &str, repeat: RepeatType) -> Reminder {
        let past = "2020-01-01T00:00:00Z";
        let mut r = Reminder::new(note_id.to_string(), note_title.to_string(), past.to_string(), repeat.as_str().to_string());
        r.id = id.to_string();
        r
    }

    #[test]
    fn test_fire_reminders_with_deps_no_due() {
        // 无到期提醒 → 不调用 notifier
        let notifier = MockNotifier::new();
        let calendar = MockCalendar { next: None };
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();

        fire_reminders_with_deps(&notifier, &calendar, &note_repo, &reminder_repo);

        assert_eq!(notifier.notify_calls.lock().unwrap().len(), 0);
        assert_eq!(notifier.activate_calls.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_fire_reminders_with_deps_archived_note_skipped() {
        // 归档便签 → 跳过提醒（不发通知、不弹窗）
        let notifier = MockNotifier::new();
        let calendar = MockCalendar { next: None };
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();

        let note = make_note("note-1", "归档便签", "内容", true);
        note_repo.save(&note).unwrap();
        let reminder = make_due_reminder("rem-1", "note-1", "归档便签", RepeatType::Once);
        reminder_repo.save(&reminder).unwrap();

        fire_reminders_with_deps(&notifier, &calendar, &note_repo, &reminder_repo);

        assert_eq!(notifier.notify_calls.lock().unwrap().len(), 0, "归档便签不应发通知");
        assert_eq!(notifier.activate_calls.lock().unwrap().len(), 0, "归档便签不应弹窗");
        // 提醒状态保持 Pending（未触发）
        let saved = reminder_repo.find_by_id("rem-1").unwrap().unwrap();
        assert_eq!(saved.status, ReminderStatus::Pending);
    }

    #[test]
    fn test_fire_reminders_with_deps_once_reminder_marked_triggered() {
        // 一次性到期提醒 → 发通知 + 弹窗 + 标记 Triggered
        let notifier = MockNotifier::new();
        let calendar = MockCalendar { next: None };
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();

        let note = make_note("note-1", "测试便签", "内容", false);
        note_repo.save(&note).unwrap();
        let reminder = make_due_reminder("rem-1", "note-1", "测试便签", RepeatType::Once);
        reminder_repo.save(&reminder).unwrap();

        fire_reminders_with_deps(&notifier, &calendar, &note_repo, &reminder_repo);

        assert_eq!(notifier.notify_calls.lock().unwrap().len(), 1);
        assert_eq!(notifier.activate_calls.lock().unwrap().len(), 1);
        let saved = reminder_repo.find_by_id("rem-1").unwrap().unwrap();
        assert_eq!(saved.status, ReminderStatus::Triggered);
    }

    #[test]
    fn test_fire_reminders_with_deps_daily_reminder_reset_to_next() {
        // Daily 周期提醒 → 发通知 + 保持 Pending + remind_at 推进到下一天
        let notifier = MockNotifier::new();
        let calendar = MockCalendar { next: None };
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();

        let note = make_note("note-1", "周期便签", "内容", false);
        note_repo.save(&note).unwrap();
        let reminder = make_due_reminder("rem-1", "note-1", "周期便签", RepeatType::Daily);
        reminder_repo.save(&reminder).unwrap();

        fire_reminders_with_deps(&notifier, &calendar, &note_repo, &reminder_repo);

        let saved = reminder_repo.find_by_id("rem-1").unwrap().unwrap();
        assert_eq!(saved.status, ReminderStatus::Pending, "Daily 应保持 Pending");
        assert!(saved.remind_at.contains("2020-01-02"), "remind_at 应推进到次日");
    }

    #[test]
    fn test_fire_reminders_with_deps_lunar_monthly_success() {
        // LunarMonthly 周期提醒 + 农历计算成功 → 保持 Pending + remind_at 推进
        let notifier = MockNotifier::new();
        let calendar = MockCalendar { next: Some("2020-02-01T00:00:00Z".to_string()) };
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();

        let note = make_note("note-1", "农历便签", "内容", false);
        note_repo.save(&note).unwrap();
        let reminder = make_due_reminder("rem-1", "note-1", "农历便签", RepeatType::LunarMonthly);
        reminder_repo.save(&reminder).unwrap();

        fire_reminders_with_deps(&notifier, &calendar, &note_repo, &reminder_repo);

        let saved = reminder_repo.find_by_id("rem-1").unwrap().unwrap();
        assert_eq!(saved.status, ReminderStatus::Pending, "LunarMonthly 成功应保持 Pending");
        assert_eq!(saved.remind_at, "2020-02-01T00:00:00Z");
    }

    #[test]
    fn test_fire_reminders_with_deps_lunar_monthly_fail_marked_triggered() {
        // LunarMonthly 周期提醒 + 农历计算失败 → 标记 Triggered
        let notifier = MockNotifier::new();
        let calendar = MockCalendar { next: None };
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();

        let note = make_note("note-1", "农历便签", "内容", false);
        note_repo.save(&note).unwrap();
        let reminder = make_due_reminder("rem-1", "note-1", "农历便签", RepeatType::LunarMonthly);
        reminder_repo.save(&reminder).unwrap();

        fire_reminders_with_deps(&notifier, &calendar, &note_repo, &reminder_repo);

        let saved = reminder_repo.find_by_id("rem-1").unwrap().unwrap();
        assert_eq!(saved.status, ReminderStatus::Triggered, "LunarMonthly 失败应标记 Triggered");
    }
}
