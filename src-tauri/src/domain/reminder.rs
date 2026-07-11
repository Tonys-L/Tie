use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

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
}

impl RepeatType {
    pub fn as_str(&self) -> &str {
        match self {
            RepeatType::Once => "once",
            RepeatType::Daily => "daily",
            RepeatType::Weekly => "weekly",
            RepeatType::Monthly => "monthly",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "daily" => RepeatType::Daily,
            "weekly" => RepeatType::Weekly,
            "monthly" => RepeatType::Monthly,
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
    pub repeat_config: String,
    pub status: ReminderStatus,
    pub snoozed_until: Option<String>,
    pub created_at: String,
    pub updated_at: String,
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
            repeat_config: String::new(),
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
        // 如果贪睡中，检查贪睡截止时间
        if let Some(ref snoozed) = self.snoozed_until {
            return snoozed.as_str() <= now;
        }
        self.remind_at.as_str() <= now
    }

    /// 标记为已触发
    pub fn mark_triggered(&mut self) {
        self.status = ReminderStatus::Triggered;
        self.snoozed_until = None;
        self.touch();
    }

    /// 贪睡
    pub fn snooze(&mut self, minutes: i64) {
        let until = Utc::now() + chrono::Duration::minutes(minutes);
        self.snoozed_until = Some(until.to_rfc3339());
        self.status = ReminderStatus::Pending;
        self.touch();
    }

    /// 标记完成
    pub fn mark_done(&mut self) {
        self.status = ReminderStatus::Done;
        self.touch();
    }

    /// 取消
    pub fn cancel(&mut self) {
        self.status = ReminderStatus::Cancelled;
        self.touch();
    }

    /// 是否为周期提醒
    pub fn is_repeating(&self) -> bool {
        self.repeat_type != RepeatType::Once
    }

    /// 计算下次触发时间
    pub fn next_trigger(&self) -> Option<String> {
        if !self.is_repeating() {
            return None;
        }
        let current = chrono::DateTime::parse_from_rfc3339(&self.remind_at).ok()?;
        let next = match self.repeat_type {
            RepeatType::Daily => current + chrono::Duration::days(1),
            RepeatType::Weekly => current + chrono::Duration::days(7),
            RepeatType::Monthly => {
                // 简化处理：加 30 天
                current + chrono::Duration::days(30)
            }
            RepeatType::Once => return None,
        };
        Some(next.to_rfc3339())
    }

    /// 周期提醒触发后重置为下一个周期（保持 Pending 状态）
    /// 如果不是周期提醒，返回 false（调用方应改为 mark_triggered）
    pub fn reset_for_next_trigger(&mut self) -> bool {
        if let Some(next) = self.next_trigger() {
            self.remind_at = next;
            self.snoozed_until = None;
            self.touch();
            true
        } else {
            false
        }
    }

    fn touch(&mut self) {
        self.updated_at = Utc::now().to_rfc3339();
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

    #[test]
    fn test_snooze() {
        let mut r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-01-01T00:00:00Z".to_string(),
            "once".to_string(),
        );
        r.mark_triggered();
        r.snooze(5);
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
        let next = r.next_trigger().unwrap();
        assert!(next.contains("2026-07-04"));
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
        let next = r.next_trigger().unwrap();
        // 7 天后
        assert!(next.contains("2026-07-10"));
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
        let next = r.next_trigger().unwrap();
        // 简化处理：加 30 天 -> 2026-08-02
        assert!(next.contains("2026-08-02"));
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
        r.mark_done();
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
        r.cancel();
        assert_eq!(r.status, ReminderStatus::Cancelled);
    }

    #[test]
    fn test_reset_for_next_trigger_repeating() {
        let mut r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "daily".to_string(),
        );
        assert!(r.is_repeating());
        let result = r.reset_for_next_trigger();
        assert!(result);
        assert_eq!(r.status, ReminderStatus::Pending);
        assert!(r.snoozed_until.is_none());
        assert!(r.remind_at.contains("2026-07-04"));
    }

    #[test]
    fn test_reset_for_next_trigger_once() {
        let mut r = Reminder::new(
            "note-1".to_string(),
            "".to_string(),
            "2026-07-03T08:00:00Z".to_string(),
            "once".to_string(),
        );
        let result = r.reset_for_next_trigger();
        assert!(!result);
    }
}
