//! 便签 service：便签写操作编排 + emit 事件（ADR-007）
//!
//! 职责：
//! - 便签 CRUD + 批量操作编排
//! - 写操作完成后 emit `NoteWritten` 事件（携带 `WriteAction`）
//!
//! 调用方：
//! - `commands/note_commands.rs`：薄壳调用本 service
//! - `commands/template_commands.rs`：create_note_from_template 间接调用
//! - `tray_manager` / `shortcut_manager`：创建便签入口
//! - `lib.rs` on_window_event：close_note_if_empty（窗口关闭时清理空便签）
//!
//! 依赖：
//! - `domain::{Note, NoteRepository, ReminderRepository}`
//! - `application::window_manager`（开窗/置顶）
//! - `application::image_service`（孤儿图片清理）
//! - `application::event_bus::{EventPublisher, DomainEvent, WriteAction}`

use tauri::AppHandle;

use crate::application::event_bus::{DomainEvent, EventPublisher, WriteAction};
use crate::domain::{Note, NoteRepository, ReminderRepository};

use super::{image_service, window_manager};

/// 创建便签并打开窗口，emit `NoteWritten(Created)` 事件
///
/// color 为 None 时降级为 "amber"。返回新建便签的 id。
///
/// 薄封装：委托 [`create_note_with_deps`] 处理 save + emit，再调用 window_manager 开窗。
/// 核心写逻辑抽取到 `create_note_with_deps`，脱离 AppHandle 依赖可单测（仿 INV-028 模式）。
pub fn create_note(
    app: &AppHandle,
    note_repo: &dyn NoteRepository,
    publisher: &dyn EventPublisher,
    color: Option<String>,
) -> Result<String, String> {
    let note = create_note_with_deps(note_repo, publisher, color)?;
    window_manager::open_note_window(app, &note)?;
    Ok(note.id)
}

/// 创建便签核心逻辑（可测试，不依赖 AppHandle）
///
/// 保存 Note + emit `NoteWritten(Created)` 事件，返回新建的 Note 供调用方开窗。
/// color 为 None 时降级为 "amber"。
pub fn create_note_with_deps(
    note_repo: &dyn NoteRepository,
    publisher: &dyn EventPublisher,
    color: Option<String>,
) -> Result<Note, String> {
    let color = color.unwrap_or_else(|| "amber".to_string());
    let note = Note::new(String::new(), color);
    note_repo.save(&note)?;
    publisher.emit(DomainEvent::NoteWritten {
        action: WriteAction::Created,
        id: note.id.clone(),
    });
    Ok(note)
}

/// 打开便签窗口（查询 + 开窗，非写操作，不 emit 事件）
pub fn open_note(
    app: &AppHandle,
    note_repo: &dyn NoteRepository,
    id: &str,
) -> Result<(), String> {
    let note = note_repo.find_by_id(id)?.ok_or("便签不存在")?;
    window_manager::open_note_window(app, &note)
}

/// 打开便签窗口并附带 flag（如 "reminder" 打开提醒面板）
///
/// 若窗口已存在则聚焦并发送事件，否则创建新窗口。非写操作，不 emit 事件。
pub fn open_note_with_flag(
    app: &AppHandle,
    note_repo: &dyn NoteRepository,
    id: &str,
    flag: &str,
) -> Result<(), String> {
    let note = note_repo.find_by_id(id)?.ok_or("便签不存在")?;
    if window_manager::focus_note_window_and_emit(app, &note.id, "show-reminder-panel") {
        return Ok(());
    }
    let url = format!("index.html?id={}&flag={}", note.id, flag);
    window_manager::open_note_window_with_url(app, &note, &url, note.is_pinned)
}

/// 更新便签样式（颜色、透明度、置顶）并同步窗口置顶状态，emit `NoteWritten(Updated)` 事件
pub fn update_note_style(
    app: &AppHandle,
    note_repo: &dyn NoteRepository,
    publisher: &dyn EventPublisher,
    id: &str,
    color: String,
    opacity: f64,
    is_pinned: bool,
) -> Result<(), String> {
    let mut note = note_repo.find_by_id(id)?.ok_or("便签不存在")?;
    note.set_color(color);
    note.set_opacity(opacity);
    note.set_pinned(is_pinned);
    note_repo.save(&note)?;
    window_manager::set_note_pinned(app, id, is_pinned);
    publisher.emit(DomainEvent::NoteWritten {
        action: WriteAction::Updated,
        id: id.to_string(),
    });
    Ok(())
}

/// 删除便签及关联提醒，清理孤儿图片，emit `NoteWritten(Deleted)` + 每个级联删除的 `ReminderWritten(Deleted)` 事件
///
/// 图片清理职责内聚到本函数（locality），所有调用方（单删除命令、batch_delete、
/// 未来可能的 tray/AI 调用方）自动获得清理行为，无需手动调用 image_service。
///
/// 事件 emit（ADR-007 扩展）：级联删除 reminder 时同步 emit `ReminderWritten(Deleted)`，
/// 让 lib.rs listener 统一触发 `schedule_recalc`，消除命令层手动调用（INV-029）。
pub fn delete_note(
    note_repo: &dyn NoteRepository,
    reminder_repo: &dyn ReminderRepository,
    publisher: &dyn EventPublisher,
    id: &str,
) -> Result<(), String> {
    // 存在性守卫：便签不存在时幂等返回 Ok(())，不 emit 事件。
    // 避免生产环境（sqlite delete 静默成功）对不存在的便签 emit NoteWritten(Deleted)
    // 触发不必要的 schedule_auto_sync（INV-013 保真度缺口修复）。
    let mut note = match note_repo.find_by_id(id)? {
        Some(n) => n,
        None => return Ok(()),
    };
    // 删除前清理便签内容中的图片文件（先取 content 再 delete）
    image_service::cleanup_removed_images(&note.content, "");
    // 级联软删除 reminder（INV-032）：逐个 Reminder::delete() + save，emit ReminderWritten(Deleted)
    let mut reminders = reminder_repo.find_by_note_id(id)?;
    for reminder in &mut reminders {
        reminder.delete();
        reminder_repo.save(reminder)?;
        publisher.emit(DomainEvent::ReminderWritten {
            action: WriteAction::Deleted,
            id: reminder.id.clone(),
        });
    }
    // 软删除 note（INV-032）：Note::delete() + save
    note.delete();
    note_repo.save(&note)?;
    publisher.emit(DomainEvent::NoteWritten {
        action: WriteAction::Deleted,
        id: id.to_string(),
    });
    Ok(())
}

/// 空便签自动删除（INV-003）
///
/// 若便签 title+content 均空则删除并 emit `NoteWritten(Deleted)` 事件，否则不做任何操作。
pub fn close_note_if_empty(
    note_repo: &dyn NoteRepository,
    publisher: &dyn EventPublisher,
    note_id: &str,
) {
    match note_repo.find_by_id(note_id) {
        Ok(Some(mut note)) => {
            if note.is_empty() {
                // 软删除（INV-032）：Note::delete() + save
                note.delete();
                if let Err(e) = note_repo.save(&note) {
                    eprintln!("[窗口] 空便签删除失败: {}", e);
                } else {
                    eprintln!("[窗口] 空便签已自动删除: {}", note_id);
                    publisher.emit(DomainEvent::NoteWritten {
                        action: WriteAction::Deleted,
                        id: note_id.to_string(),
                    });
                }
            }
        }
        Ok(None) => {}
        Err(e) => eprintln!("[窗口] 检查便签失败: {}", e),
    }
}

// ============ 单字段更新 ============

/// 更新便签内容（含孤儿图片清理），emit `NoteWritten(Updated)` 事件
pub fn update_note_content(
    note_repo: &dyn NoteRepository,
    publisher: &dyn EventPublisher,
    id: &str,
    content: String,
) -> Result<(), String> {
    let mut note = note_repo.find_by_id(id)?.ok_or("便签不存在")?;
    let old_content = note.content.clone();
    image_service::cleanup_removed_images(&old_content, &content);
    note.update_content(content);
    note_repo.save(&note)?;
    publisher.emit(DomainEvent::NoteWritten {
        action: WriteAction::Updated,
        id: id.to_string(),
    });
    Ok(())
}

/// 更新便签标题，emit `NoteWritten(Updated)` 事件
pub fn update_note_title(
    note_repo: &dyn NoteRepository,
    publisher: &dyn EventPublisher,
    id: &str,
    title: String,
) -> Result<(), String> {
    let mut note = note_repo.find_by_id(id)?.ok_or("便签不存在")?;
    note.update_title(title);
    note_repo.save(&note)?;
    publisher.emit(DomainEvent::NoteWritten {
        action: WriteAction::Updated,
        id: id.to_string(),
    });
    Ok(())
}

/// 更新窗口位置和尺寸，emit `NoteWritten(Updated)` 事件
pub fn update_note_window_state(
    note_repo: &dyn NoteRepository,
    publisher: &dyn EventPublisher,
    id: &str,
    pos_x: i32,
    pos_y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let mut note = note_repo.find_by_id(id)?.ok_or("便签不存在")?;
    note.update_window_state(pos_x, pos_y, width, height);
    note_repo.save(&note)?;
    publisher.emit(DomainEvent::NoteWritten {
        action: WriteAction::Updated,
        id: id.to_string(),
    });
    Ok(())
}

/// 更新便签标签，emit `NoteWritten(Updated)` 事件
pub fn update_note_tags(
    note_repo: &dyn NoteRepository,
    publisher: &dyn EventPublisher,
    id: &str,
    tags: Vec<String>,
) -> Result<(), String> {
    let mut note = note_repo.find_by_id(id)?.ok_or("便签不存在")?;
    note.set_tags(tags);
    note_repo.save(&note)?;
    publisher.emit(DomainEvent::NoteWritten {
        action: WriteAction::Updated,
        id: id.to_string(),
    });
    Ok(())
}

/// 归档便签，emit `NoteWritten(Updated)` 事件
pub fn archive_note(
    note_repo: &dyn NoteRepository,
    publisher: &dyn EventPublisher,
    id: &str,
) -> Result<(), String> {
    let mut note = note_repo.find_by_id(id)?.ok_or("便签不存在")?;
    note.archive();
    note_repo.save(&note)?;
    publisher.emit(DomainEvent::NoteWritten {
        action: WriteAction::Updated,
        id: id.to_string(),
    });
    Ok(())
}

/// 取消归档，emit `NoteWritten(Updated)` 事件
pub fn unarchive_note(
    note_repo: &dyn NoteRepository,
    publisher: &dyn EventPublisher,
    id: &str,
) -> Result<(), String> {
    let mut note = note_repo.find_by_id(id)?.ok_or("便签不存在")?;
    note.unarchive();
    note_repo.save(&note)?;
    publisher.emit(DomainEvent::NoteWritten {
        action: WriteAction::Updated,
        id: id.to_string(),
    });
    Ok(())
}

// ============ 批量操作 ============

/// 批量归档便签，对每个成功项 emit `NoteWritten(Updated)` 事件，返回成功 id 列表
pub fn batch_archive(
    note_repo: &dyn NoteRepository,
    publisher: &dyn EventPublisher,
    ids: &[String],
) -> Result<Vec<String>, String> {
    let mut succeeded = Vec::new();
    for id in ids {
        if let Ok(Some(mut note)) = note_repo.find_by_id(id) {
            note.archive();
            if note_repo.save(&note).is_ok() {
                publisher.emit(DomainEvent::NoteWritten {
                    action: WriteAction::Updated,
                    id: id.clone(),
                });
                succeeded.push(id.clone());
            }
        }
    }
    Ok(succeeded)
}

/// 批量取消归档，对每个成功项 emit `NoteWritten(Updated)` 事件，返回成功 id 列表
pub fn batch_unarchive(
    note_repo: &dyn NoteRepository,
    publisher: &dyn EventPublisher,
    ids: &[String],
) -> Result<Vec<String>, String> {
    let mut succeeded = Vec::new();
    for id in ids {
        if let Ok(Some(mut note)) = note_repo.find_by_id(id) {
            note.unarchive();
            if note_repo.save(&note).is_ok() {
                publisher.emit(DomainEvent::NoteWritten {
                    action: WriteAction::Updated,
                    id: id.clone(),
                });
                succeeded.push(id.clone());
            }
        }
    }
    Ok(succeeded)
}

/// 批量删除便签（含级联删除提醒 + 图片清理），对每个成功项 emit `NoteWritten(Deleted)` 事件
///
/// 图片清理由 `delete_note` 内部处理（locality），本函数仅做批量编排。
/// 窗口关闭（window_manager::close_note_window）保留在命令层，因为涉及 Tauri 副作用。
pub fn batch_delete(
    note_repo: &dyn NoteRepository,
    reminder_repo: &dyn ReminderRepository,
    publisher: &dyn EventPublisher,
    ids: &[String],
) -> Result<Vec<String>, String> {
    let mut succeeded = Vec::new();
    for id in ids {
        // 幂等删除：delete_note 对不存在的便签返回 Ok(())（与 sqlite 对齐），
        // 但 batch_delete 只应返回"实际删除了"的 id，故预检查存在性。
        if note_repo.find_by_id(id)?.is_none() {
            continue;
        }
        if delete_note(note_repo, reminder_repo, publisher, id).is_ok() {
            succeeded.push(id.clone());
        }
    }
    Ok(succeeded)
}

/// 批量修改便签颜色，对每个成功项 emit `NoteWritten(Updated)` 事件，返回成功 id 列表
pub fn batch_update_color(
    note_repo: &dyn NoteRepository,
    publisher: &dyn EventPublisher,
    ids: &[String],
    color: String,
) -> Result<Vec<String>, String> {
    let mut succeeded = Vec::new();
    for id in ids {
        if let Ok(Some(mut note)) = note_repo.find_by_id(id) {
            note.set_color(color.clone());
            if note_repo.save(&note).is_ok() {
                publisher.emit(DomainEvent::NoteWritten {
                    action: WriteAction::Updated,
                    id: id.clone(),
                });
                succeeded.push(id.clone());
            }
        }
    }
    Ok(succeeded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::event_bus::MockEventPublisher;
    use crate::domain::mock_repo::{InMemoryNoteRepository, InMemoryReminderRepository};
    use crate::domain::Reminder;

    fn mock_publisher() -> (MockEventPublisher, std::sync::Arc<std::sync::Mutex<Vec<DomainEvent>>>) {
        let mock = MockEventPublisher::new();
        let events = mock.events_clone();
        (mock, events)
    }

    fn count_events(events: &std::sync::Arc<std::sync::Mutex<Vec<DomainEvent>>>) -> usize {
        events.lock().unwrap().len()
    }

    // ============ create_note_with_deps 测试 ============

    #[test]
    fn test_create_note_with_deps_emits_created() {
        // 验证 create_note_with_deps（脱离 AppHandle）正确 save + emit NoteWritten(Created)
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();

        let note = create_note_with_deps(&note_repo, &mock, Some("blue".into())).unwrap();

        assert_eq!(note.color, "blue");
        assert!(note.title.is_empty());
        assert!(note_repo.find_by_id(&note.id).unwrap().is_some());

        let events = events.lock().unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            DomainEvent::NoteWritten { action, id } => {
                assert_eq!(*action, WriteAction::Created);
                assert_eq!(id, &note.id);
            }
            _ => panic!("expected NoteWritten(Created)"),
        }
    }

    #[test]
    fn test_create_note_with_deps_defaults_amber() {
        // color 为 None 时降级为 "amber"
        let note_repo = InMemoryNoteRepository::new();
        let (mock, _events) = mock_publisher();

        let note = create_note_with_deps(&note_repo, &mock, None).unwrap();
        assert_eq!(note.color, "amber");
    }

    // ============ delete_note 测试 ============

    #[test]
    fn test_delete_note_with_reminders_emits_deleted() {
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();

        let note = Note::new("测试".to_string(), "amber".to_string());
        let reminder = Reminder::new(
            note.id.clone(),
            "标题".to_string(),
            "2099-01-01T00:00:00Z".to_string(),
            "once".to_string(),
        );
        let reminder_id = reminder.id.clone();
        note_repo.save(&note).unwrap();
        reminder_repo.save(&reminder).unwrap();

        delete_note(&note_repo, &reminder_repo, &mock, &note.id).unwrap();

        assert!(note_repo.find_by_id(&note.id).unwrap().is_none());
        assert!(reminder_repo.find_by_note_id(&note.id).unwrap().is_empty());
        // 应 emit 2 个事件：ReminderWritten(Deleted) + NoteWritten(Deleted)
        assert_eq!(count_events(&events), 2);
        let events_guard = events.lock().unwrap();
        let mut found_reminder_deleted = false;
        let mut found_note_deleted = false;
        for event in events_guard.iter() {
            match event {
                DomainEvent::ReminderWritten { action, id } => {
                    assert_eq!(*action, WriteAction::Deleted);
                    assert_eq!(id, &reminder_id);
                    found_reminder_deleted = true;
                }
                DomainEvent::NoteWritten { action, id } => {
                    assert_eq!(*action, WriteAction::Deleted);
                    assert_eq!(id, &note.id);
                    found_note_deleted = true;
                }
                _ => panic!("unexpected event"),
            }
        }
        assert!(found_reminder_deleted, "应 emit ReminderWritten(Deleted)");
        assert!(found_note_deleted, "应 emit NoteWritten(Deleted)");
    }

    #[test]
    fn test_delete_note_not_exists_idempotent_no_emit() {
        // 幂等删除：便签不存在时返回 Ok(()) 且不 emit 事件（与 sqlite 行为对齐）
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();

        let result = delete_note(&note_repo, &reminder_repo, &mock, "nonexistent");
        assert!(result.is_ok());
        assert_eq!(count_events(&events), 0);
    }

    // ============ delete_note 软删除测试 (INV-032) ============

    #[test]
    fn delete_note_is_soft_delete() {
        // 软删除：find_all 不含该 note，find_all_including_deleted 含墓碑且 is_deleted 为 true（INV-032）
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let (mock, _events) = mock_publisher();
        let note = Note::new("测试".to_string(), "amber".to_string());
        let note_id = note.id.clone();
        note_repo.save(&note).unwrap();

        delete_note(&note_repo, &reminder_repo, &mock, &note_id).unwrap();

        // find_all 不含已软删除的 note
        let active = note_repo.find_all().unwrap();
        assert!(!active.iter().any(|n| n.id == note_id));
        // find_all_including_deleted 含墓碑，且 is_deleted 为 true
        let including = note_repo.find_all_including_deleted().unwrap();
        let tombstone = including
            .iter()
            .find(|n| n.id == note_id)
            .expect("墓碑应存在");
        assert!(tombstone.is_deleted());
    }

    #[test]
    fn delete_note_cascades_soft_delete_reminder() {
        // 级联软删除 reminder：find_by_note_id 不含，find_all_including_deleted 含墓碑且 is_deleted 为 true（INV-032）
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let (mock, _events) = mock_publisher();
        let note = Note::new("测试".to_string(), "amber".to_string());
        let reminder = Reminder::new(
            note.id.clone(),
            "标题".to_string(),
            "2099-01-01T00:00:00Z".to_string(),
            "once".to_string(),
        );
        let reminder_id = reminder.id.clone();
        note_repo.save(&note).unwrap();
        reminder_repo.save(&reminder).unwrap();

        delete_note(&note_repo, &reminder_repo, &mock, &note.id).unwrap();

        // reminder 的 find_by_note_id 不含已软删除的 reminder
        let active = reminder_repo.find_by_note_id(&note.id).unwrap();
        assert!(!active.iter().any(|r| r.id == reminder_id));
        // reminder 的 find_all_including_deleted 含墓碑，且 is_deleted 为 true
        let including = reminder_repo.find_all_including_deleted().unwrap();
        let tombstone = including
            .iter()
            .find(|r| r.id == reminder_id)
            .expect("reminder 墓碑应存在");
        assert!(tombstone.is_deleted());
    }

    #[test]
    fn delete_note_emits_note_written_event() {
        // 验证 emit NoteWritten(Deleted) 事件；有关联 reminder 时同时 emit ReminderWritten(Deleted)
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();
        let note = Note::new("测试".to_string(), "amber".to_string());
        let note_id = note.id.clone();
        let reminder = Reminder::new(
            note.id.clone(),
            "标题".to_string(),
            "2099-01-01T00:00:00Z".to_string(),
            "once".to_string(),
        );
        let reminder_id = reminder.id.clone();
        note_repo.save(&note).unwrap();
        reminder_repo.save(&reminder).unwrap();

        delete_note(&note_repo, &reminder_repo, &mock, &note_id).unwrap();

        let events_guard = events.lock().unwrap();
        // 验证 emit 了 NoteWritten { action: Deleted, id }
        let has_note_deleted = events_guard.iter().any(|e| {
            matches!(e, DomainEvent::NoteWritten { action: WriteAction::Deleted, id } if id == &note_id)
        });
        assert!(has_note_deleted, "应 emit NoteWritten(Deleted)");
        // 验证 emit 了 ReminderWritten { action: Deleted, ... }（有关联 reminder 时）
        let has_reminder_deleted = events_guard.iter().any(|e| {
            matches!(e, DomainEvent::ReminderWritten { action: WriteAction::Deleted, id } if id == &reminder_id)
        });
        assert!(has_reminder_deleted, "应 emit ReminderWritten(Deleted)");
    }

    // ============ close_note_if_empty 测试 (INV-003) ============

    #[test]
    fn test_close_if_empty_both_empty_emits_deleted() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let mut note = Note::new(String::new(), "amber".to_string());
        note.title = String::new();
        note.content = String::new();
        note_repo.save(&note).unwrap();

        close_note_if_empty(&note_repo, &mock, &note.id);

        assert!(note_repo.find_by_id(&note.id).unwrap().is_none());
        assert_eq!(count_events(&events), 1);
    }

    #[test]
    fn test_close_if_empty_has_content_no_emit() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let mut note = Note::new("标题".to_string(), "amber".to_string());
        note.title = "有内容".to_string();
        note.content = "".to_string();
        note_repo.save(&note).unwrap();

        close_note_if_empty(&note_repo, &mock, &note.id);

        assert!(note_repo.find_by_id(&note.id).unwrap().is_some());
        assert_eq!(count_events(&events), 0);
    }

    #[test]
    fn test_close_if_empty_title_only_no_emit() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let mut note = Note::new(String::new(), "amber".to_string());
        note.title = "仅标题".to_string();
        note.content = String::new();
        note_repo.save(&note).unwrap();

        close_note_if_empty(&note_repo, &mock, &note.id);

        assert!(note_repo.find_by_id(&note.id).unwrap().is_some());
        assert_eq!(count_events(&events), 0);
    }

    #[test]
    fn test_close_if_empty_content_only_no_emit() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let mut note = Note::new(String::new(), "amber".to_string());
        note.title = String::new();
        note.content = "有内容".to_string();
        note_repo.save(&note).unwrap();

        close_note_if_empty(&note_repo, &mock, &note.id);

        assert!(note_repo.find_by_id(&note.id).unwrap().is_some());
        assert_eq!(count_events(&events), 0);
    }

    #[test]
    fn test_close_if_empty_not_exist_no_emit() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        close_note_if_empty(&note_repo, &mock, "nonexistent");
        assert_eq!(count_events(&events), 0);
    }

    #[test]
    fn close_note_if_empty_is_soft_delete() {
        // 软删除：find_all_including_deleted 含墓碑且 is_deleted 为 true（INV-032）
        let note_repo = InMemoryNoteRepository::new();
        let (mock, _events) = mock_publisher();
        let mut note = Note::new(String::new(), "amber".to_string());
        note.title = String::new();
        note.content = String::new();
        let note_id = note.id.clone();
        note_repo.save(&note).unwrap();

        close_note_if_empty(&note_repo, &mock, &note_id);

        // find_all_including_deleted 含墓碑，且 is_deleted 为 true
        let including = note_repo.find_all_including_deleted().unwrap();
        let tombstone = including
            .iter()
            .find(|n| n.id == note_id)
            .expect("墓碑应存在");
        assert!(tombstone.is_deleted());
    }

    // ============ update_note_content 测试 ============

    #[test]
    fn test_update_note_content_persists_and_emits() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let note = Note::new("标题".to_string(), "amber".to_string());
        let id = note.id.clone();
        note_repo.save(&note).unwrap();

        update_note_content(&note_repo, &mock, &id, "新内容".to_string()).unwrap();

        let saved = note_repo.find_by_id(&id).unwrap().unwrap();
        assert_eq!(saved.content, "新内容");
        assert_eq!(count_events(&events), 1);
        let events_guard = events.lock().unwrap();
        match &events_guard[0] {
            DomainEvent::NoteWritten { action, id: eid } => {
                assert_eq!(*action, WriteAction::Updated);
                assert_eq!(eid, &id);
            }
            _ => panic!("expected NoteWritten"),
        }
    }

    #[test]
    fn test_update_note_content_not_found_no_emit() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let result = update_note_content(&note_repo, &mock, "nonexistent", "x".to_string());
        assert!(result.is_err());
        assert_eq!(count_events(&events), 0);
    }

    #[test]
    fn test_update_note_content_with_image_cleanup_no_panic() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, _events) = mock_publisher();
        let note = Note::new("标题".to_string(), "amber".to_string());
        let id = note.id.clone();
        note_repo.save(&note).unwrap();

        let result = update_note_content(
            &note_repo,
            &mock,
            &id,
            "![alt](img:abc.png) 文本".to_string(),
        );
        assert!(result.is_ok());
    }

    // ============ update_note_title 测试 ============

    #[test]
    fn test_update_note_title_persists_and_emits() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let note = Note::new("旧标题".to_string(), "amber".to_string());
        let id = note.id.clone();
        note_repo.save(&note).unwrap();

        update_note_title(&note_repo, &mock, &id, "新标题".to_string()).unwrap();

        let saved = note_repo.find_by_id(&id).unwrap().unwrap();
        assert_eq!(saved.title, "新标题");
        assert_eq!(count_events(&events), 1);
    }

    #[test]
    fn test_update_note_title_not_found_no_emit() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let result = update_note_title(&note_repo, &mock, "nonexistent", "x".to_string());
        assert!(result.is_err());
        assert_eq!(count_events(&events), 0);
    }

    // ============ update_note_window_state 测试 ============

    #[test]
    fn test_update_note_window_state_persists_and_emits() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let note = Note::new("标题".to_string(), "amber".to_string());
        let id = note.id.clone();
        note_repo.save(&note).unwrap();

        update_note_window_state(&note_repo, &mock, &id, 100, 200, 320, 280).unwrap();

        let saved = note_repo.find_by_id(&id).unwrap().unwrap();
        assert_eq!(saved.window_state.pos_x, 100);
        assert_eq!(saved.window_state.pos_y, 200);
        assert_eq!(saved.window_state.width, 320);
        assert_eq!(saved.window_state.height, 280);
        assert_eq!(count_events(&events), 1);
    }

    #[test]
    fn test_update_note_window_state_clamp_min_size() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, _events) = mock_publisher();
        let note = Note::new("标题".to_string(), "amber".to_string());
        let id = note.id.clone();
        note_repo.save(&note).unwrap();

        update_note_window_state(&note_repo, &mock, &id, 0, 0, 50, 50).unwrap();

        let saved = note_repo.find_by_id(&id).unwrap().unwrap();
        assert_eq!(saved.window_state.width, 200);
        assert_eq!(saved.window_state.height, 150);
    }

    #[test]
    fn test_update_note_window_state_not_found_no_emit() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let result = update_note_window_state(&note_repo, &mock, "nonexistent", 0, 0, 200, 150);
        assert!(result.is_err());
        assert_eq!(count_events(&events), 0);
    }

    // ============ update_note_tags 测试 ============

    #[test]
    fn test_update_note_tags_persists_and_emits() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let note = Note::new("标题".to_string(), "amber".to_string());
        let id = note.id.clone();
        note_repo.save(&note).unwrap();

        update_note_tags(
            &note_repo,
            &mock,
            &id,
            vec!["work".to_string(), "personal".to_string()],
        )
        .unwrap();

        let saved = note_repo.find_by_id(&id).unwrap().unwrap();
        assert_eq!(saved.tags.len(), 2);
        assert!(saved.tags.contains(&"work".to_string()));
        assert_eq!(count_events(&events), 1);
    }

    #[test]
    fn test_update_note_tags_dedup() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, _events) = mock_publisher();
        let note = Note::new("标题".to_string(), "amber".to_string());
        let id = note.id.clone();
        note_repo.save(&note).unwrap();

        update_note_tags(&note_repo, &mock, &id, vec!["work".to_string(), "work".to_string()]).unwrap();

        let saved = note_repo.find_by_id(&id).unwrap().unwrap();
        assert_eq!(saved.tags.len(), 1);
    }

    #[test]
    fn test_update_note_tags_not_found_no_emit() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let result = update_note_tags(&note_repo, &mock, "nonexistent", vec![]);
        assert!(result.is_err());
        assert_eq!(count_events(&events), 0);
    }

    // ============ archive_note / unarchive_note 测试 ============

    #[test]
    fn test_archive_note_sets_archived_and_emits() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let note = Note::new("标题".to_string(), "amber".to_string());
        let id = note.id.clone();
        note_repo.save(&note).unwrap();

        archive_note(&note_repo, &mock, &id).unwrap();

        let saved = note_repo.find_by_id(&id).unwrap().unwrap();
        assert!(saved.is_archived);
        assert!(!saved.is_reminder_eligible());
        assert_eq!(count_events(&events), 1);
    }

    #[test]
    fn test_unarchive_note_clears_archived_and_emits() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let mut note = Note::new("标题".to_string(), "amber".to_string());
        note.archive();
        let id = note.id.clone();
        note_repo.save(&note).unwrap();

        unarchive_note(&note_repo, &mock, &id).unwrap();

        let saved = note_repo.find_by_id(&id).unwrap().unwrap();
        assert!(!saved.is_archived);
        assert!(saved.is_reminder_eligible());
        assert_eq!(count_events(&events), 1);
    }

    #[test]
    fn test_archive_note_not_found_no_emit() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let result = archive_note(&note_repo, &mock, "nonexistent");
        assert!(result.is_err());
        assert_eq!(count_events(&events), 0);
    }

    // ============ batch_archive 测试 ============

    #[test]
    fn test_batch_archive_all_succeed_emits_per_item() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let n1 = Note::new("n1".to_string(), "amber".to_string());
        let n2 = Note::new("n2".to_string(), "amber".to_string());
        let id1 = n1.id.clone();
        let id2 = n2.id.clone();
        note_repo.save(&n1).unwrap();
        note_repo.save(&n2).unwrap();

        let succeeded = batch_archive(&note_repo, &mock, &[id1.clone(), id2.clone()]).unwrap();

        assert_eq!(succeeded.len(), 2);
        assert!(note_repo.find_by_id(&id1).unwrap().unwrap().is_archived);
        // 每个成功项 emit 一次
        assert_eq!(count_events(&events), 2);
    }

    #[test]
    fn test_batch_archive_partial_failure_emits_only_succeeded() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let n1 = Note::new("n1".to_string(), "amber".to_string());
        let id1 = n1.id.clone();
        note_repo.save(&n1).unwrap();

        let succeeded = batch_archive(
            &note_repo,
            &mock,
            &[id1.clone(), "nonexistent".to_string()],
        )
        .unwrap();

        assert_eq!(succeeded.len(), 1);
        assert_eq!(count_events(&events), 1);
    }

    #[test]
    fn test_batch_archive_empty_list_no_emit() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let succeeded = batch_archive(&note_repo, &mock, &[]).unwrap();
        assert!(succeeded.is_empty());
        assert_eq!(count_events(&events), 0);
    }

    // ============ batch_unarchive 测试 ============

    #[test]
    fn test_batch_unarchive_all_succeed_emits_per_item() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let mut n1 = Note::new("n1".to_string(), "amber".to_string());
        n1.archive();
        let id1 = n1.id.clone();
        note_repo.save(&n1).unwrap();

        let succeeded = batch_unarchive(&note_repo, &mock, &[id1.clone()]).unwrap();

        assert_eq!(succeeded.len(), 1);
        assert!(!note_repo.find_by_id(&id1).unwrap().unwrap().is_archived);
        assert_eq!(count_events(&events), 1);
    }

    // ============ batch_delete 测试 ============

    #[test]
    fn test_batch_delete_all_succeed_with_cascade_emits_per_item() {
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();
        let n1 = Note::new("n1".to_string(), "amber".to_string());
        let n2 = Note::new("n2".to_string(), "amber".to_string());
        let id1 = n1.id.clone();
        let id2 = n2.id.clone();
        note_repo.save(&n1).unwrap();
        note_repo.save(&n2).unwrap();
        let r = Reminder::new(
            id1.clone(),
            "t".to_string(),
            "2099-01-01T00:00:00Z".to_string(),
            "once".to_string(),
        );
        reminder_repo.save(&r).unwrap();

        let succeeded = batch_delete(&note_repo, &reminder_repo, &mock, &[id1.clone(), id2.clone()]).unwrap();

        assert_eq!(succeeded.len(), 2);
        assert!(note_repo.find_by_id(&id1).unwrap().is_none());
        assert!(reminder_repo.find_by_note_id(&id1).unwrap().is_empty());
        // 事件数 = 3：n1 级联删除 reminder emit ReminderWritten(Deleted) + n1 emit NoteWritten(Deleted) + n2 emit NoteWritten(Deleted)
        assert_eq!(count_events(&events), 3);
        // 验证事件类型分布
        let events_guard = events.lock().unwrap();
        let reminder_deleted_count = events_guard.iter().filter(|e| {
            matches!(e, DomainEvent::ReminderWritten { action: WriteAction::Deleted, .. })
        }).count();
        let note_deleted_count = events_guard.iter().filter(|e| {
            matches!(e, DomainEvent::NoteWritten { action: WriteAction::Deleted, .. })
        }).count();
        assert_eq!(reminder_deleted_count, 1, "n1 级联删除 1 个 reminder");
        assert_eq!(note_deleted_count, 2, "删除 2 张 note");
    }

    #[test]
    fn test_batch_delete_partial_failure_emits_only_succeeded() {
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();
        let n1 = Note::new("n1".to_string(), "amber".to_string());
        let id1 = n1.id.clone();
        note_repo.save(&n1).unwrap();

        let succeeded = batch_delete(
            &note_repo,
            &reminder_repo,
            &mock,
            &[id1.clone(), "nonexistent".to_string()],
        )
        .unwrap();

        assert_eq!(succeeded.len(), 1);
        assert_eq!(count_events(&events), 1);
    }

    #[test]
    fn test_batch_delete_empty_list_no_emit() {
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let (mock, events) = mock_publisher();
        let succeeded = batch_delete(&note_repo, &reminder_repo, &mock, &[]).unwrap();
        assert!(succeeded.is_empty());
        assert_eq!(count_events(&events), 0);
    }

    // ============ batch_update_color 测试 ============

    #[test]
    fn test_batch_update_color_all_succeed_emits_per_item() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let n1 = Note::new("n1".to_string(), "amber".to_string());
        let n2 = Note::new("n2".to_string(), "amber".to_string());
        let id1 = n1.id.clone();
        let id2 = n2.id.clone();
        note_repo.save(&n1).unwrap();
        note_repo.save(&n2).unwrap();

        let succeeded =
            batch_update_color(&note_repo, &mock, &[id1.clone(), id2.clone()], "blue".to_string()).unwrap();

        assert_eq!(succeeded.len(), 2);
        assert_eq!(note_repo.find_by_id(&id1).unwrap().unwrap().color, "blue");
        assert_eq!(count_events(&events), 2);
    }

    #[test]
    fn test_batch_update_color_partial_failure_emits_only_succeeded() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let n1 = Note::new("n1".to_string(), "amber".to_string());
        let id1 = n1.id.clone();
        note_repo.save(&n1).unwrap();

        let succeeded = batch_update_color(
            &note_repo,
            &mock,
            &[id1.clone(), "nonexistent".to_string()],
            "blue".to_string(),
        )
        .unwrap();

        assert_eq!(succeeded.len(), 1);
        assert_eq!(count_events(&events), 1);
    }

    #[test]
    fn test_batch_update_color_empty_list_no_emit() {
        let note_repo = InMemoryNoteRepository::new();
        let (mock, events) = mock_publisher();
        let succeeded = batch_update_color(&note_repo, &mock, &[], "blue".to_string()).unwrap();
        assert!(succeeded.is_empty());
        assert_eq!(count_events(&events), 0);
    }
}
