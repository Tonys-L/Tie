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
/// 编排：find → snooze → save → 返回 note_id 用于通知。
/// 若当前状态不允许 snooze（终态），返回错误且不 save 不 emit。
pub fn snooze_reminder(
    reminder_repo: &dyn ReminderRepository,
    publisher: &dyn EventPublisher,
    id: &str,
    minutes: i64,
) -> Result<String, String> {
    let mut reminder = reminder_repo.find_by_id(id)?.ok_or("提醒不存在")?;
    let note_id = reminder.note_id.clone();
    reminder.snooze(minutes)?;
    reminder_repo.save(&reminder)?;
    publisher.emit(DomainEvent::ReminderWritten {
        action: WriteAction::Updated,
        id: id.to_string(),
    });
    Ok(note_id)
}

/// 关闭提醒，emit `ReminderWritten(Updated)` 事件
///
/// 编排：find → mark_done → save → 返回 note_id 用于通知。
/// 若当前状态不允许 mark_done（终态），返回错误且不 save 不 emit。
pub fn dismiss_reminder(
    reminder_repo: &dyn ReminderRepository,
    publisher: &dyn EventPublisher,
    id: &str,
) -> Result<String, String> {
    let mut reminder = reminder_repo.find_by_id(id)?.ok_or("提醒不存在")?;
    let note_id = reminder.note_id.clone();
    reminder.mark_done()?;
    reminder_repo.save(&reminder)?;
    publisher.emit(DomainEvent::ReminderWritten {
        action: WriteAction::Updated,
        id: id.to_string(),
    });
    Ok(note_id)
}

/// 删除提醒（软删除，墓碑机制 INV-032），emit `ReminderWritten(Deleted)` 事件
///
/// 编排：find → delete（软删除，设 deleted_at + updated_at）→ save → 返回 note_id 用于通知。
/// 返回 `Option<String>`：被删除提醒对应的 note_id（若提醒不存在则为 None）。
///
/// 存在性守卫：提醒不存在时幂等返回 `Ok(None)`，不 emit 事件（INV-013 保真度缺口修复）。
pub fn delete_reminder(
    reminder_repo: &dyn ReminderRepository,
    publisher: &dyn EventPublisher,
    id: &str,
) -> Result<Option<String>, String> {
    let mut reminder = match reminder_repo.find_by_id(id)? {
        Some(r) => r,
        None => return Ok(None),
    };
    let note_id = reminder.note_id.clone();
    reminder.delete(); // 软删除（墓碑机制 INV-032）
    reminder_repo.save(&reminder)?;
    publisher.emit(DomainEvent::ReminderWritten {
        action: WriteAction::Deleted,
        id: id.to_string(),
    });
    Ok(Some(note_id))
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

    // ============ 终态拒绝转换测试（INV-031）============

    #[test]
    fn test_snooze_reminder_on_done_returns_err_no_emit() {
        let repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();
        let r = sample_reminder(&repo);

        // 手动置为 Done 终态
        let mut done = r.clone();
        done.status = crate::domain::reminder::ReminderStatus::Done;
        repo.save(&done).unwrap();

        let result = snooze_reminder(&repo, &mock, &r.id, 10);
        assert!(result.is_err());
        assert_eq!(count_events(&events), 0, "终态拒绝转换不应 emit");
    }

    #[test]
    fn test_dismiss_reminder_on_done_returns_err_no_emit() {
        let repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();
        let r = sample_reminder(&repo);

        // 手动置为 Done 终态
        let mut done = r.clone();
        done.status = crate::domain::reminder::ReminderStatus::Done;
        repo.save(&done).unwrap();

        let result = dismiss_reminder(&repo, &mock, &r.id);
        assert!(result.is_err());
        assert_eq!(count_events(&events), 0, "终态拒绝转换不应 emit");
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
    fn test_delete_reminder_nonexistent_idempotent_no_emit() {
        // 幂等删除：提醒不存在时返回 Ok(None) 且不 emit 事件（与 sqlite 行为对齐）
        let repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();

        let result = delete_reminder(&repo, &mock, "nonexistent");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
        assert_eq!(count_events(&events), 0);
    }

    // ============ delete_reminder 软删除测试（INV-032）============

    #[test]
    fn delete_reminder_is_soft_delete() {
        let repo = InMemoryReminderRepository::new();
        let (mock, _events) = mock_publisher();
        let r = sample_reminder(&repo);

        delete_reminder(&repo, &mock, &r.id).unwrap();

        // find_all 不含已软删除的 reminder（业务查询默认过滤墓碑）
        let active = repo.find_all().unwrap();
        assert!(active.iter().all(|x| x.id != r.id));

        // find_all_including_deleted 含该 reminder 且 is_deleted 为 true（墓碑保留）
        let all_inc = repo.find_all_including_deleted().unwrap();
        let tombstone = all_inc.iter().find(|x| x.id == r.id).expect("墓碑应保留");
        assert!(tombstone.is_deleted());
    }

    #[test]
    fn delete_reminder_emits_event() {
        let repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();
        let r = sample_reminder(&repo);

        delete_reminder(&repo, &mock, &r.id).unwrap();

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
    fn delete_reminder_returns_note_id() {
        let repo = InMemoryReminderRepository::new();
        let (mock, _events) = mock_publisher();
        let r = sample_reminder(&repo);

        let note_id = delete_reminder(&repo, &mock, &r.id).unwrap();
        assert_eq!(note_id, Some("note-1".to_string()));
    }

    #[test]
    fn delete_reminder_not_found_returns_none() {
        let repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();

        let result = delete_reminder(&repo, &mock, "nonexistent");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
        assert_eq!(count_events(&events), 0);
    }
}
