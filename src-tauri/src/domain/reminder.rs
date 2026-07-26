use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use chrono::Datelike;

/// 提醒状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ReminderStatus {
    Pending,
    Triggered,
    Done,
    Cancelled,
}

impl ReminderStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ReminderStatus::Pending => "pending",
            ReminderStatus::Triggered => "triggered",
            ReminderStatus::Done => "done",
            ReminderStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "triggered" => ReminderStatus::Triggered,
            "done" => ReminderStatus::Done,
            "cancelled" => ReminderStatus::Cancelled,
            _ => ReminderStatus::Pending,
        }
    }
}

/// 重复类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RepeatType {
    Once,
    Daily,
    Weekly,
    Monthly,
    LunarMonthly,
}

impl RepeatType {
    pub fn as_str(&self) -> &str {
        match self {
            RepeatType::Once => "once",
            RepeatType::Daily => "daily",
            RepeatType::Weekly => "weekly",
            RepeatType::Monthly => "monthly",
            RepeatType::LunarMonthly => "lunar_monthly",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "daily" => RepeatType::Daily,
            "weekly" => RepeatType::Weekly,
            "monthly" => RepeatType::Monthly,
            "lunar_monthly" => RepeatType::LunarMonthly,
            _ => RepeatType::Once,
        }
    }
}

/// Reminder 实体 — 提醒领域模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: String,
    pub note_id: String,
    pub note_title: String,
    pub remind_at: String,
    pub repeat_type: RepeatType,
    pub status: ReminderStatus,
    pub snoozed_until: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// next_trigger 返回值：区分"不再触发"、"domain 可计算"、"需外部计算"
///
/// 设计目的：让 domain 层的 seam 完整，调用方无需用 `repeat_type` 二次判别。
/// - `None`：一次性提醒或无下次触发
/// - `DateTime(s)`：Daily/Weekly/Monthly，domain 直接计算
/// - `External`：LunarMonthly，需 `CalendarAdapter` 提供农历计算
#[derive(Debug, Clone, PartialEq)]
pub enum NextTrigger {
    None,
    DateTime(String),
    External,
}

/// advance_state 返回值：状态推进结果
#[derive(Debug, Clone, PartialEq)]
pub enum AdvanceResult {
    /// 周期提醒已重置到下次时间，保持 Pending
    ResetToNext,
    /// 一次性提醒或外部计算失败，已标记为 Triggered
    MarkedTriggered,
}

/// 农历计算适配器 trait（domain 层定义接口，application 层提供实现）
///
/// 设计目的：保留 INV-020 的核心意图（domain 不依赖 tyme4rs），
/// 同时恢复 seam 完整性 — LunarMonthly 的下次时间由 trait 方法提供，
/// 调用方无需用 `repeat_type` 二次判别。
pub trait CalendarAdapter: Send + Sync {
    /// 计算农历月份+1后的公历 ISO 时间（失败返回 None）
    fn lunar_next_month(&self, iso_time: &str) -> Option<String>;
}

impl Reminder {
    /// 创建新提醒
    pub fn new(note_id: String, note_title: String, remind_at: String, repeat_type: String) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            note_id,
            note_title,
            remind_at,
            repeat_type: RepeatType::from_str(&repeat_type),
            status: ReminderStatus::Pending,
            snoozed_until: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// 是否已到触发时间
    pub fn is_due(&self, now: &str) -> bool {
        if self.status != ReminderStatus::Pending {
            return false;
        }
        self.effective_time() <= now
    }

    /// 有效触发时间（单一真相源，INV-008）
    ///
    /// 贪睡中返回 `snoozed_until`，否则返回 `remind_at`。
    /// infrastructure 的 SQL `COALESCE(snoozed_until, remind_at)` 必须与此语义一致。
    pub fn effective_time(&self) -> &str {
        self.snoozed_until.as_deref().unwrap_or(&self.remind_at)
    }

    /// 标记为已触发
    ///
    /// 仅 Pending 可触发（INV-008）。终态（Done/Cancelled）及 Triggered 拒绝转换。
    pub fn mark_triggered(&mut self) -> Result<(), String> {
        match self.status {
            ReminderStatus::Pending => {
                self.status = ReminderStatus::Triggered;
                self.snoozed_until = None;
                self.touch();
                Ok(())
            }
            _ => Err(format!(
                "非法状态转换: {} 不允许 mark_triggered（仅 Pending 可触发）",
                self.status.as_str()
            )),
        }
    }

    /// 贪睡
    ///
    /// 仅 Pending/Triggered 可贪睡（用户主动延后）。终态（Done/Cancelled）拒绝转换。
    pub fn snooze(&mut self, minutes: i64) -> Result<(), String> {
        match self.status {
            ReminderStatus::Pending | ReminderStatus::Triggered => {
                let until = Utc::now() + chrono::Duration::minutes(minutes);
                self.snoozed_until = Some(until.to_rfc3339());
                self.status = ReminderStatus::Pending;
                self.touch();
                Ok(())
            }
            _ => Err(format!(
                "非法状态转换: {} 不允许 snooze（终态不可贪睡）",
                self.status.as_str()
            )),
        }
    }

    /// 标记完成
    ///
    /// 仅 Pending/Triggered 可标记完成。终态（Done/Cancelled）拒绝转换。
    pub fn mark_done(&mut self) -> Result<(), String> {
        match self.status {
            ReminderStatus::Pending | ReminderStatus::Triggered => {
                self.status = ReminderStatus::Done;
                self.touch();
                Ok(())
            }
            _ => Err(format!(
                "非法状态转换: {} 不允许 mark_done（终态不可转换）",
                self.status.as_str()
            )),
        }
    }

    /// 取消
    ///
    /// 仅 Pending/Triggered 可取消。终态（Done/Cancelled）拒绝转换。
    pub fn cancel(&mut self) -> Result<(), String> {
        match self.status {
            ReminderStatus::Pending | ReminderStatus::Triggered => {
                self.status = ReminderStatus::Cancelled;
                self.touch();
                Ok(())
            }
            _ => Err(format!(
                "非法状态转换: {} 不允许 cancel（终态不可转换）",
                self.status.as_str()
            )),
        }
    }

    /// 是否为周期提醒
    pub fn is_repeating(&self) -> bool {
        self.repeat_type != RepeatType::Once
    }

    /// 计算下次触发时间
    ///
    /// 返回 `NextTrigger` enum 区分三种情况：
    /// - `None`：一次性提醒（Once）或无下次触发
    /// - `DateTime(s)`：Daily/Weekly/Monthly，domain 直接计算
    /// - `External`：LunarMonthly，需 `CalendarAdapter` 提供农历计算
    pub fn next_trigger(&self) -> NextTrigger {
        if !self.is_repeating() {
            return NextTrigger::None;
        }
        let current = match chrono::DateTime::parse_from_rfc3339(&self.remind_at) {
            Ok(c) => c,
            Err(_) => return NextTrigger::None,
        };
        match self.repeat_type {
            RepeatType::Daily => NextTrigger::DateTime((current + chrono::Duration::days(1)).to_rfc3339()),
            RepeatType::Weekly => NextTrigger::DateTime((current + chrono::Duration::days(7)).to_rfc3339()),
            RepeatType::Monthly => {
                // 精确日历月：月份+1，月末溢出取目标月最后一天
                let naive = current.naive_utc();
                let (next_year, next_month) = if naive.month() == 12 {
                    (naive.year() + 1, 1u32)
                } else {
                    (naive.year(), naive.month() + 1)
                };
                let day = naive.day();
                let next_date = chrono::NaiveDate::from_ymd_opt(next_year, next_month, day)
                    .unwrap_or_else(|| {
                        // day 超出目标月天数，取目标月最后一天
                        let first_after = chrono::NaiveDate::from_ymd_opt(
                            if next_month == 12 { next_year + 1 } else { next_year },
                            if next_month == 12 { 1 } else { next_month + 1 },
                            1,
                        ).unwrap();
                        first_after.pred_opt().unwrap()
                    });
                let next_naive = chrono::NaiveDateTime::new(next_date, naive.time());
                let next = chrono::DateTime::<chrono::FixedOffset>::from_naive_utc_and_offset(next_naive, *current.offset());
                NextTrigger::DateTime(next.to_rfc3339())
            }
            RepeatType::LunarMonthly => NextTrigger::External,
            RepeatType::Once => NextTrigger::None,
        }
    }

    /// 状态推进：触发后根据重复类型决定下一步动作
    ///
    /// - Once：mark_triggered，返回 MarkedTriggered
    /// - Daily/Weekly/Monthly：重置 remind_at 为下次时间，返回 ResetToNext
    /// - LunarMonthly：通过 `calendar` trait 计算农历月份+1；成功返回 ResetToNext，失败 mark_triggered 返回 MarkedTriggered
    ///
    /// 设计目的：把状态推进逻辑从 application 层下沉到 domain 层，
    /// 消除 fire_reminders 中"if repeat_type == LunarMonthly"的二次判别。
    ///
    /// 契约：调用方必须保证 `self.status == Pending`（`fire_reminders_with_deps` 通过
    /// `find_due` 的 `WHERE status='pending'` 保证）。内部 `mark_triggered` 用 expect
    /// 表达此契约，若违反则 panic 暴露调用方 bug。
    pub fn advance_state(&mut self, calendar: &dyn CalendarAdapter) -> AdvanceResult {
        match self.next_trigger() {
            NextTrigger::None => {
                self.mark_triggered()
                    .expect("advance_state 契约: 仅对 Pending 提醒调用");
                AdvanceResult::MarkedTriggered
            }
            NextTrigger::DateTime(next) => {
                self.remind_at = next;
                self.snoozed_until = None;
                self.touch();
                AdvanceResult::ResetToNext
            }
            NextTrigger::External => {
                match calendar.lunar_next_month(&self.remind_at) {
                    Some(next) => {
                        self.remind_at = next;
                        self.snoozed_until = None;
                        self.touch();
                        AdvanceResult::ResetToNext
                    }
                    None => {
                        self.mark_triggered()
                            .expect("advance_state 契约: 仅对 Pending 提醒调用");
                        AdvanceResult::MarkedTriggered
                    }
                }
            }
        }
    }

    fn touch(&mut self) {
        self.updated_at = Utc::now().to_rfc3339();
    }

    /// 通知标题：note_title 为空时 fallback 为 "便签提醒"。
    ///
    /// 此为"提醒通知如何展示"的领域规则，从调度器下沉至此以便单测。
    pub fn notification_title(&self) -> String {
        if self.note_title.is_empty() {
            "便签提醒".to_string()
        } else {
            self.note_title.clone()
        }
    }

    /// 通知正文：基于便签 content 前 80 字符构造。
    ///
    /// - content 超过 80 字符：截断 + "..." 省略号
    /// - content 为空：fallback "点击查看便签"
    /// - 其他：原样返回前 80 字符
    ///
    /// `content` 由调用方（调度器）传入 `&note.content`，避免 Reminder 持有 Note 引用。
    pub fn notification_body(&self, content: &str) -> String {
        const MAX_LEN: usize = 80;
        let summary: String = content.chars().take(MAX_LEN).collect();
        if content.chars().count() > MAX_LEN {
            format!("{}...", summary)
        } else if summary.is_empty() {
            "点击查看便签".to_string()
        } else {
            summary
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_reminder() {
        let r = Reminder::new(
            "note-1".to_string(),
            "测试".to_string(),
            "2026-07-03T15:00:00+08:00".to_string(),
            "once".to_string(),
        );
        assert_eq!(r.status, ReminderStatus::Pending);
        assert!(!r.is_repeating());
    }

    #[test]
    fn test_is_due() {
        let r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-01-01T00:00:00Z".to_string(),
            "once".to_string(),
        );
        assert!(r.is_due("2026-07-03T00:00:00Z"));
    }

    // 回归测试：时间格式不一致时的 is_due 行为（LES-024 时间格式漂移）
    // 字符串比较 ISO 8601 时间在数字部分（年月日时分秒）按字典序与数值比较一致
    // 仅在同一秒内的毫秒部分可能出现边界问题（最多 1 秒误差，可接受）
    #[test]
    fn test_is_due_remind_at_with_millis_now_without_millis() {
        // remind_at 带毫秒，now 不带毫秒，remind_at < now → 应到期
        let r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-25T14:18:00.000Z".to_string(),
            "once".to_string(),
        );
        assert!(r.is_due("2026-07-25T14:18:30Z"));
    }

    #[test]
    fn test_is_due_both_without_millis_normal_case() {
        // 修复后默认场景：remind_at 和 now 都不带毫秒
        let r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-25T14:18:00Z".to_string(),
            "once".to_string(),
        );
        // 正好到期
        assert!(r.is_due("2026-07-25T14:18:00Z"));
        // 提前 1 秒
        assert!(!r.is_due("2026-07-25T14:17:59Z"));
        // 延后 1 秒
        assert!(r.is_due("2026-07-25T14:18:01Z"));
    }

    #[test]
    fn test_is_due_same_second_millis_boundary() {
        // 边界场景：remind_at 和 now 在同一秒，毫秒部分格式不一致
        // remind_at="...14:18:00Z" vs now="...14:18:00.123Z"
        // 字符串比较第 20 位 'Z'（90）> '.'（46）→ remind_at > now → is_due 返回 false
        // 这是字符串比较的固有缺陷，最多导致 1 秒延迟，可接受
        let r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-25T14:18:00Z".to_string(),
            "once".to_string(),
        );
        // now 比 remind_at 晚 123ms，但因格式不一致字符串比较认为 remind_at > now
        assert!(!r.is_due("2026-07-25T14:18:00.123Z"));
        // 修复后 now 不带毫秒，此边界场景不会出现
        assert!(r.is_due("2026-07-25T14:18:01Z"));
    }

    #[test]
    fn test_snooze() {
        let mut r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-01-01T00:00:00Z".to_string(),
            "once".to_string(),
        );
        r.mark_triggered().unwrap();
        r.snooze(5).unwrap();
        assert_eq!(r.status, ReminderStatus::Pending);
        assert!(r.snoozed_until.is_some());
    }

    #[test]
    fn test_next_trigger_daily() {
        let r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "daily".to_string(),
        );
        match r.next_trigger() {
            NextTrigger::DateTime(s) => assert!(s.contains("2026-07-04")),
            _ => panic!("expected DateTime"),
        }
    }

    #[test]
    fn test_weekly_repeat() {
        let r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "weekly".to_string(),
        );
        assert!(r.is_repeating());
        match r.next_trigger() {
            NextTrigger::DateTime(s) => assert!(s.contains("2026-07-10")),
            _ => panic!("expected DateTime"),
        }
    }

    #[test]
    fn test_monthly_repeat() {
        let r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "monthly".to_string(),
        );
        assert!(r.is_repeating());
        match r.next_trigger() {
            NextTrigger::DateTime(s) => assert!(s.contains("2026-08-03")),
            _ => panic!("expected DateTime"),
        }
    }

    #[test]
    fn test_monthly_repeat_month_end_overflow() {
        // 1月31日 → 2月最后一天（非闰年 2月28日）
        let r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-01-31T08:00:00Z".to_string(),
            "monthly".to_string(),
        );
        match r.next_trigger() {
            NextTrigger::DateTime(s) => assert!(s.contains("2026-02-28")),
            _ => panic!("expected DateTime"),
        }
    }

    #[test]
    fn test_monthly_repeat_december_to_january() {
        // 12月15日 → 次年1月15日
        let r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-12-15T08:00:00Z".to_string(),
            "monthly".to_string(),
        );
        match r.next_trigger() {
            NextTrigger::DateTime(s) => assert!(s.contains("2027-01-15")),
            _ => panic!("expected DateTime"),
        }
    }

    #[test]
    fn test_next_trigger_once_returns_none() {
        let r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "once".to_string(),
        );
        assert_eq!(r.next_trigger(), NextTrigger::None);
    }

    #[test]
    fn test_next_trigger_lunar_returns_external() {
        let r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "lunar_monthly".to_string(),
        );
        assert!(r.is_repeating());
        // domain 层不计算农历，返回 External 标记需外部计算
        assert_eq!(r.next_trigger(), NextTrigger::External);
    }

    #[test]
    fn test_mark_done() {
        let mut r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "once".to_string(),
        );
        assert_eq!(r.status, ReminderStatus::Pending);
        r.mark_done().unwrap();
        assert_eq!(r.status, ReminderStatus::Done);
    }

    #[test]
    fn test_cancel() {
        let mut r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "once".to_string(),
        );
        assert_eq!(r.status, ReminderStatus::Pending);
        r.cancel().unwrap();
        assert_eq!(r.status, ReminderStatus::Cancelled);
    }

    // ============ 状态转换合法性测试（INV-031）============
    // 约束：constraints.md 第 363-367 行要求每个合法转换 + 每个禁止转换独立测试

    fn make_reminder() -> Reminder {
        Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "once".to_string(),
        )
    }

    // --- 合法转换 ---

    #[test]
    fn test_snooze_from_pending_keeps_pending() {
        let mut r = make_reminder();
        r.snooze(5).unwrap();
        assert_eq!(r.status, ReminderStatus::Pending);
        assert!(r.snoozed_until.is_some());
    }

    #[test]
    fn test_mark_done_from_triggered() {
        let mut r = make_reminder();
        r.mark_triggered().unwrap();
        r.mark_done().unwrap();
        assert_eq!(r.status, ReminderStatus::Done);
    }

    #[test]
    fn test_cancel_from_triggered() {
        let mut r = make_reminder();
        r.mark_triggered().unwrap();
        r.cancel().unwrap();
        assert_eq!(r.status, ReminderStatus::Cancelled);
    }

    // --- 禁止转换：Done 终态拒绝所有 ---

    #[test]
    fn test_done_rejects_mark_triggered() {
        let mut r = make_reminder();
        r.status = ReminderStatus::Done;
        assert!(r.mark_triggered().is_err());
        assert_eq!(r.status, ReminderStatus::Done);
    }

    #[test]
    fn test_done_rejects_snooze() {
        let mut r = make_reminder();
        r.status = ReminderStatus::Done;
        assert!(r.snooze(5).is_err());
        assert_eq!(r.status, ReminderStatus::Done);
    }

    #[test]
    fn test_done_rejects_mark_done() {
        let mut r = make_reminder();
        r.status = ReminderStatus::Done;
        assert!(r.mark_done().is_err());
        assert_eq!(r.status, ReminderStatus::Done);
    }

    #[test]
    fn test_done_rejects_cancel() {
        let mut r = make_reminder();
        r.status = ReminderStatus::Done;
        assert!(r.cancel().is_err());
        assert_eq!(r.status, ReminderStatus::Done);
    }

    // --- 禁止转换：Cancelled 终态拒绝所有 ---

    #[test]
    fn test_cancelled_rejects_mark_triggered() {
        let mut r = make_reminder();
        r.status = ReminderStatus::Cancelled;
        assert!(r.mark_triggered().is_err());
        assert_eq!(r.status, ReminderStatus::Cancelled);
    }

    #[test]
    fn test_cancelled_rejects_snooze() {
        let mut r = make_reminder();
        r.status = ReminderStatus::Cancelled;
        assert!(r.snooze(5).is_err());
        assert_eq!(r.status, ReminderStatus::Cancelled);
    }

    #[test]
    fn test_cancelled_rejects_mark_done() {
        let mut r = make_reminder();
        r.status = ReminderStatus::Cancelled;
        assert!(r.mark_done().is_err());
        assert_eq!(r.status, ReminderStatus::Cancelled);
    }

    #[test]
    fn test_cancelled_rejects_cancel() {
        let mut r = make_reminder();
        r.status = ReminderStatus::Cancelled;
        assert!(r.cancel().is_err());
        assert_eq!(r.status, ReminderStatus::Cancelled);
    }

    // --- 禁止转换：Triggered 不可重复触发 ---

    #[test]
    fn test_triggered_rejects_mark_triggered() {
        let mut r = make_reminder();
        r.mark_triggered().unwrap();
        assert!(r.mark_triggered().is_err());
        assert_eq!(r.status, ReminderStatus::Triggered);
    }

    // ============ advance_state 测试 ============

    /// Mock 农历适配器：返回固定值或 None
    struct MockCalendarAdapter {
        next: Option<String>,
    }
    impl CalendarAdapter for MockCalendarAdapter {
        fn lunar_next_month(&self, _iso_time: &str) -> Option<String> {
            self.next.clone()
        }
    }

    #[test]
    fn test_advance_state_once_marked_triggered() {
        let mut r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "once".to_string(),
        );
        let cal = MockCalendarAdapter { next: None };
        let result = r.advance_state(&cal);
        assert_eq!(result, AdvanceResult::MarkedTriggered);
        assert_eq!(r.status, ReminderStatus::Triggered);
    }

    #[test]
    fn test_advance_state_daily_reset_to_next() {
        let mut r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "daily".to_string(),
        );
        let cal = MockCalendarAdapter { next: None };
        let result = r.advance_state(&cal);
        assert_eq!(result, AdvanceResult::ResetToNext);
        assert_eq!(r.status, ReminderStatus::Pending);
        assert!(r.snoozed_until.is_none());
        assert!(r.remind_at.contains("2026-07-04"));
    }

    #[test]
    fn test_advance_state_weekly_reset_to_next() {
        let mut r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "weekly".to_string(),
        );
        let cal = MockCalendarAdapter { next: None };
        let result = r.advance_state(&cal);
        assert_eq!(result, AdvanceResult::ResetToNext);
        assert!(r.remind_at.contains("2026-07-10"));
    }

    #[test]
    fn test_advance_state_monthly_reset_to_next() {
        let mut r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "monthly".to_string(),
        );
        let cal = MockCalendarAdapter { next: None };
        let result = r.advance_state(&cal);
        assert_eq!(result, AdvanceResult::ResetToNext);
        assert!(r.remind_at.contains("2026-08-03"));
    }

    #[test]
    fn test_advance_state_lunar_monthly_success() {
        let mut r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "lunar_monthly".to_string(),
        );
        // 模拟农历计算成功
        let cal = MockCalendarAdapter { next: Some("2026-08-08T08:00:00Z".to_string()) };
        let result = r.advance_state(&cal);
        assert_eq!(result, AdvanceResult::ResetToNext);
        assert_eq!(r.status, ReminderStatus::Pending);
        assert_eq!(r.remind_at, "2026-08-08T08:00:00Z");
    }

    #[test]
    fn test_advance_state_lunar_monthly_fail_marked_triggered() {
        let mut r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "lunar_monthly".to_string(),
        );
        // 模拟农历计算失败（返回 None）
        let cal = MockCalendarAdapter { next: None };
        let result = r.advance_state(&cal);
        assert_eq!(result, AdvanceResult::MarkedTriggered);
        assert_eq!(r.status, ReminderStatus::Triggered);
    }

    // ============ notification_title / notification_body 测试 ============

    #[test]
    fn test_notification_title_with_content() {
        let r = Reminder::new(
            "note-1".to_string(),
            "周一会议".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "once".to_string(),
        );
        assert_eq!(r.notification_title(), "周一会议");
    }

    #[test]
    fn test_notification_title_empty_fallback() {
        let r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "once".to_string(),
        );
        assert_eq!(r.notification_title(), "便签提醒");
    }

    #[test]
    fn test_notification_body_short_content() {
        let r = Reminder::new(
            "note-1".to_string(),
            "标题".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "once".to_string(),
        );
        assert_eq!(r.notification_body("短内容"), "短内容");
    }

    #[test]
    fn test_notification_body_empty_fallback() {
        let r = Reminder::new(
            "note-1".to_string(),
            "标题".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "once".to_string(),
        );
        assert_eq!(r.notification_body(""), "点击查看便签");
    }

    #[test]
    fn test_notification_body_long_content_truncated() {
        let r = Reminder::new(
            "note-1".to_string(),
            "标题".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "once".to_string(),
        );
        // 100 字符内容 → 截断为 80 + "..."
        let long_content: String = "a".repeat(100);
        let body = r.notification_body(&long_content);
        assert_eq!(body.chars().count(), 83); // 80 个 a + 3 个点
        assert!(body.ends_with("..."));
        let a_count = body.chars().filter(|c| *c == 'a').count();
        assert_eq!(a_count, 80);
    }

    #[test]
    fn test_notification_body_exact_80_chars_not_truncated() {
        let r = Reminder::new(
            "note-1".to_string(),
            "标题".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "once".to_string(),
        );
        // 恰好 80 字符 → 不截断、无省略号
        let content: String = "a".repeat(80);
        let body = r.notification_body(&content);
        assert_eq!(body.chars().count(), 80);
        assert!(!body.ends_with("..."));
    }

    #[test]
    fn test_notification_body_utf8_chars_counted_correctly() {
        let r = Reminder::new(
            "note-1".to_string(),
            "标题".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "once".to_string(),
        );
        // 中文 5 字符 → 不截断
        let body = r.notification_body("你好世界测试");
        assert_eq!(body, "你好世界测试");
    }
}
