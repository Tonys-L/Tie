//! 模板 service：模板写操作编排 + emit 事件（ADR-007）
//!
//! 职责：
//! - 模板 CRUD 编排（save/delete）
//! - 从模板创建便签（查模板 + 建 Note + 打开窗口）
//! - 写操作完成后 emit `TemplateWritten` / `NoteWritten` 事件
//!
//! 调用方：
//! - `commands/template_commands.rs`：薄壳调用本 service
//!
//! 依赖：
//! - `domain::{Note, Template, NoteRepository, TemplateRepository}`
//! - `application::window_manager`（开窗）
//! - `application::event_bus::{EventPublisher, DomainEvent, WriteAction}`

use tauri::AppHandle;

use crate::application::event_bus::{DomainEvent, EventPublisher, WriteAction};
use crate::application::window_manager;
use crate::domain::{Note, NoteRepository, Template, TemplateRepository};

/// 保存模板（新增或更新），emit `TemplateWritten` 事件
pub fn save_template(
    template_repo: &dyn TemplateRepository,
    publisher: &dyn EventPublisher,
    template: &Template,
) -> Result<(), String> {
    let is_new = template_repo.find_by_id(&template.id)?.is_none();
    let action = if is_new {
        WriteAction::Created
    } else {
        WriteAction::Updated
    };
    template_repo.save(template)?;
    publisher.emit(DomainEvent::TemplateWritten {
        action,
        id: template.id.clone(),
    });
    Ok(())
}

/// 删除模板，emit `TemplateWritten(Deleted)` 事件
pub fn delete_template(
    template_repo: &dyn TemplateRepository,
    publisher: &dyn EventPublisher,
    id: &str,
) -> Result<(), String> {
    template_repo.delete(id)?;
    publisher.emit(DomainEvent::TemplateWritten {
        action: WriteAction::Deleted,
        id: id.to_string(),
    });
    Ok(())
}

/// 从模板创建便签：查模板 → 建 Note（写入模板内容）→ 保存 → 开窗 → emit `NoteWritten`
///
/// 返回新便签 id。
pub fn create_note_from_template(
    app: &AppHandle,
    note_repo: &dyn NoteRepository,
    template_repo: &dyn TemplateRepository,
    publisher: &dyn EventPublisher,
    template_id: &str,
) -> Result<String, String> {
    let template = template_repo
        .find_by_id(template_id)?
        .ok_or_else(|| format!("模板不存在: {}", template_id))?;
    let mut note = Note::new(template.name.clone(), "amber".to_string());
    note.update_content(template.content);
    note_repo.save(&note)?;
    window_manager::open_note_window(app, &note)?;
    publisher.emit(DomainEvent::NoteWritten {
        action: WriteAction::Created,
        id: note.id.clone(),
    });
    Ok(note.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::event_bus::MockEventPublisher;
    use crate::domain::mock_repo::InMemoryTemplateRepository;

    fn make_template() -> Template {
        Template::new("测试模板".to_string(), "内容".to_string())
    }

    #[test]
    fn test_save_template_new_emits_created() {
        let repo = InMemoryTemplateRepository::new();
        let mock = MockEventPublisher::new();
        let events = mock.events_clone();
        let template = make_template();

        save_template(&repo, &mock, &template).unwrap();

        let events = events.lock().unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            DomainEvent::TemplateWritten { action, id } => {
                assert_eq!(*action, WriteAction::Created);
                assert_eq!(id, &template.id);
            }
            _ => panic!("expected TemplateWritten"),
        }
    }

    #[test]
    fn test_save_template_existing_emits_updated() {
        let repo = InMemoryTemplateRepository::new();
        let mock = MockEventPublisher::new();
        let events = mock.events_clone();
        let template = make_template();

        save_template(&repo, &mock, &template).unwrap();
        save_template(&repo, &mock, &template).unwrap();

        let events = events.lock().unwrap();
        assert_eq!(events.len(), 2);
        match &events[1] {
            DomainEvent::TemplateWritten { action, .. } => {
                assert_eq!(*action, WriteAction::Updated);
            }
            _ => panic!("expected TemplateWritten"),
        }
    }

    #[test]
    fn test_delete_template_emits_deleted() {
        let repo = InMemoryTemplateRepository::new();
        let mock = MockEventPublisher::new();
        let events = mock.events_clone();
        let template = make_template();

        save_template(&repo, &mock, &template).unwrap();
        delete_template(&repo, &mock, &template.id).unwrap();

        let events = events.lock().unwrap();
        assert_eq!(events.len(), 2);
        match &events[1] {
            DomainEvent::TemplateWritten { action, id } => {
                assert_eq!(*action, WriteAction::Deleted);
                assert_eq!(id, &template.id);
            }
            _ => panic!("expected TemplateWritten"),
        }
    }

    #[test]
    fn test_save_template_propagates_repo_error() {
        // 验证 repo 失败时不 emit 事件
        struct FailingRepo;
        impl TemplateRepository for FailingRepo {
            fn save(&self, _: &Template) -> Result<(), String> {
                Err("repo error".to_string())
            }
            fn find_by_id(&self, _: &str) -> Result<Option<Template>, String> {
                Ok(None)
            }
            fn find_all(&self) -> Result<Vec<Template>, String> {
                Ok(vec![])
            }
            fn delete(&self, _: &str) -> Result<(), String> {
                Ok(())
            }
        }

        let repo = FailingRepo;
        let mock = MockEventPublisher::new();
        let events = mock.events_clone();
        let template = make_template();

        let result = save_template(&repo, &mock, &template);
        assert!(result.is_err());
        assert_eq!(events.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_mock_publisher_as_trait_object() {
        let mock = MockEventPublisher::new();
        let _publisher: &dyn EventPublisher = &mock;
    }
}
