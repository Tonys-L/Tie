//! 提醒 service：提醒写操作编排 + emit 事件（ADR-007）
//!
//! 职责：
//! - 提醒 CRUD 编排（create/snooze/dismiss/delete）
//! - 写操作完成后 emit `ReminderWritten` 事件（携带 `WriteAction`）
//!
//! 调用方：
//! - `commands/reminder_commands.rs`：薄壳调用本 service
//! - `commands/note_commands.rs`：delete_note 级联删除提醒时调用 reminder_repo（不经过本 service）
//!
//! 依赖：
//! - `domain::{Reminder, ReminderRepository}`
//! - `application::event_bus::{EventPublisher, DomainEvent, WriteAction}`

use crate::application::event_bus::{DomainEvent, EventPublisher, WriteAction};
use crate::domain::{Reminder, ReminderRepository};

/// 创建提醒，emit `ReminderWritten(Created)` 事件
///
/// 仅承载仓储与 domain 交互逻辑，不涉及 Tauri 副作用（emit/schedule）。
/// 副作用由 lib.rs 监听器统一处理。
pub fn create_reminder(
    reminder_repo: &dyn ReminderRepository,
    publisher: &dyn EventPublisher,
    note_id: String,
    note_title: String,
    remind_at: String,
    repeat_type: String,
) -> Result<Reminder, String> {
    let reminder = Reminder::new(note_id, note_title, remind_at, repeat_type);
    reminder_repo.save(&reminder)?;
    publisher.emit(DomainEvent::ReminderWritten {
        action: WriteAction::Created,
        id: reminder.id.clone(),
    });
    Ok(reminder)
}

/// 贪睡提醒，emit `ReminderWritten(Updated)` 事件
///
/// 编排：find → snooze → save → 返回 note_id 用于通知
pub fn snooze_reminder(
    reminder_repo: &dyn ReminderRepository,
    publisher: &dyn EventPublisher,
    id: &str,
    minutes: i64,
) -> Result<String, String> {
    let mut reminder = reminder_repo.find_by_id(id)?.ok_or("提醒不存在")?;
    let note_id = reminder.note_id.clone();
    reminder.snooze(minutes);
    reminder_repo.save(&reminder)?;
    publisher.emit(DomainEvent::ReminderWritten {
        action: WriteAction::Updated,
        id: id.to_string(),
    });
    Ok(note_id)
}

/// 关闭提醒，emit `ReminderWritten(Updated)` 事件
///
/// 编排：find → mark_done → save → 返回 note_id 用于通知
pub fn dismiss_reminder(
    reminder_repo: &dyn ReminderRepository,
    publisher: &dyn EventPublisher,
    id: &str,
) -> Result<String, String> {
    let mut reminder = reminder_repo.find_by_id(id)?.ok_or("提醒不存在")?;
    let note_id = reminder.note_id.clone();
    reminder.mark_done();
    reminder_repo.save(&reminder)?;
    publisher.emit(DomainEvent::ReminderWritten {
        action: WriteAction::Updated,
        id: id.to_string(),
    });
    Ok(note_id)
}

/// 删除提醒，emit `ReminderWritten(Deleted)` 事件
///
/// 编排：查 note_id → delete → 返回 note_id 用于通知。
/// 返回 `Option<String>`：被删除提醒对应的 note_id（若提醒不存在则为 None）。
pub fn delete_reminder(
    reminder_repo: &dyn ReminderRepository,
    publisher: &dyn EventPublisher,
    id: &str,
) -> Result<Option<String>, String> {
    let note_id = reminder_repo
        .find_by_id(id)
        .ok()
        .flatten()
        .map(|r| r.note_id.clone());
    reminder_repo.delete(id)?;
    publisher.emit(DomainEvent::ReminderWritten {
        action: WriteAction::Deleted,
        id: id.to_string(),
    });
    Ok(note_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::event_bus::MockEventPublisher;
    use crate::domain::mock_repo::InMemoryReminderRepository;

    fn mock_publisher() -> (MockEventPublisher, std::sync::Arc<std::sync::Mutex<Vec<DomainEvent>>>) {
        let mock = MockEventPublisher::new();
        let events = mock.events_clone();
        (mock, events)
    }

    fn count_events(events: &std::sync::Arc<std::sync::Mutex<Vec<DomainEvent>>>) -> usize {
        events.lock().unwrap().len()
    }

    fn sample_reminder(repo: &InMemoryReminderRepository) -> Reminder {
        let r = Reminder::new(
            "note-1".to_string(),
            "测试提醒".to_string(),
            "2099-01-01T00:00:00Z".to_string(),
            "once".to_string(),
        );
        repo.save(&r).unwrap();
        r
    }

    // ============ create_reminder 测试 ============

    #[test]
    fn test_create_reminder_persists_and_emits_created() {
        let repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();

        let reminder = create_reminder(
            &repo,
            &mock,
            "note-1".to_string(),
            "标题".to_string(),
            "2099-01-01T00:00:00Z".to_string(),
            "once".to_string(),
        )
        .unwrap();

        // 已持久化
        let loaded = repo.find_by_id(&reminder.id).unwrap().unwrap();
        assert_eq!(loaded.note_id, "note-1");
        assert_eq!(loaded.note_title, "标题");
        assert_eq!(loaded.remind_at, "2099-01-01T00:00:00Z");

        // emit ReminderWritten(Created)
        assert_eq!(count_events(&events), 1);
        let events_guard = events.lock().unwrap();
        match &events_guard[0] {
            DomainEvent::ReminderWritten { action, id } => {
                assert_eq!(*action, WriteAction::Created);
                assert_eq!(id, &reminder.id);
            }
            _ => panic!("expected ReminderWritten"),
        }
    }

    #[test]
    fn test_create_reminder_returns_constructed_reminder() {
        let repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();

        let reminder = create_reminder(
            &repo,
            &mock,
            "note-1".to_string(),
            "标题".to_string(),
            "2099-01-01T00:00:00Z".to_string(),
            "daily".to_string(),
        )
        .unwrap();

        // 返回的 Reminder 字段正确
        assert_eq!(reminder.note_id, "note-1");
        assert_eq!(reminder.repeat_type, crate::domain::reminder::RepeatType::Daily);
        assert_eq!(count_events(&events), 1);
    }

    // ============ snooze_reminder 测试 ============

    #[test]
    fn test_snooze_reminder_returns_note_id_and_emits_updated() {
        let repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();
        let r = sample_reminder(&repo);

        let note_id = snooze_reminder(&repo, &mock, &r.id, 10).unwrap();
        assert_eq!(note_id, "note-1");

        assert_eq!(count_events(&events), 1);
        let events_guard = events.lock().unwrap();
        match &events_guard[0] {
            DomainEvent::ReminderWritten { action, id } => {
                assert_eq!(*action, WriteAction::Updated);
                assert_eq!(id, &r.id);
            }
            _ => panic!("expected ReminderWritten"),
        }
    }

    #[test]
    fn test_snooze_reminder_updates_snoozed_until() {
        let repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();
        let r = sample_reminder(&repo);

        snooze_reminder(&repo, &mock, &r.id, 15).unwrap();
        let loaded = repo.find_by_id(&r.id).unwrap().unwrap();
        assert!(loaded.snoozed_until.is_some());
        assert_eq!(count_events(&events), 1);
    }

    #[test]
    fn test_snooze_reminder_not_found_no_emit() {
        let repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();

        let result = snooze_reminder(&repo, &mock, "nonexistent", 10);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
        assert_eq!(count_events(&events), 0);
    }

    // ============ dismiss_reminder 测试 ============

    #[test]
    fn test_dismiss_reminder_marks_done_and_emits_updated() {
        let repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();
        let r = sample_reminder(&repo);

        let note_id = dismiss_reminder(&repo, &mock, &r.id).unwrap();
        assert_eq!(note_id, "note-1");

        let loaded = repo.find_by_id(&r.id).unwrap().unwrap();
        assert_eq!(loaded.status, crate::domain::reminder::ReminderStatus::Done);

        assert_eq!(count_events(&events), 1);
        let events_guard = events.lock().unwrap();
        match &events_guard[0] {
            DomainEvent::ReminderWritten { action, id } => {
                assert_eq!(*action, WriteAction::Updated);
                assert_eq!(id, &r.id);
            }
            _ => panic!("expected ReminderWritten"),
        }
    }

    #[test]
    fn test_dismiss_reminder_not_found_no_emit() {
        let repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();

        let result = dismiss_reminder(&repo, &mock, "nonexistent");
        assert!(result.is_err());
        assert_eq!(count_events(&events), 0);
    }

    // ============ delete_reminder 测试 ============

    #[test]
    fn test_delete_reminder_returns_note_id_and_emits_deleted() {
        let repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();
        let r = sample_reminder(&repo);

        let note_id = delete_reminder(&repo, &mock, &r.id).unwrap();
        assert_eq!(note_id, Some("note-1".to_string()));

        // 已删除
        assert!(repo.find_by_id(&r.id).unwrap().is_none());

        assert_eq!(count_events(&events), 1);
        let events_guard = events.lock().unwrap();
        match &events_guard[0] {
            DomainEvent::ReminderWritten { action, id } => {
                assert_eq!(*action, WriteAction::Deleted);
                assert_eq!(id, &r.id);
            }
            _ => panic!("expected ReminderWritten"),
        }
    }

    #[test]
    fn test_delete_reminder_nonexistent_returns_err_no_emit() {
        let repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();

        // 删除不存在的提醒：mock_repo.delete 返回 Err（与 sqlite 行为一致）
        let result = delete_reminder(&repo, &mock, "nonexistent");
        assert!(result.is_err());
        assert_eq!(count_events(&events), 0);
    }
}
