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

/// 删除模板（软删除，墓碑机制 INV-032），emit `TemplateWritten(Deleted)` 事件
///
/// 走 domain `Template::delete()` + `save` 软删除路径：设 `deleted_at` 和 `updated_at`，
/// 保留墓碑供跨设备同步传播删除（LES-028）。
///
/// 存在性守卫：模板不存在时幂等返回 `Ok(())`，不 emit 事件（INV-013 保真度缺口修复）。
pub fn delete_template(
    template_repo: &dyn TemplateRepository,
    publisher: &dyn EventPublisher,
    id: &str,
) -> Result<(), String> {
    let mut template = match template_repo.find_by_id(id)? {
        Some(t) => t,
        None => return Ok(()),
    };
    template.delete(); // 软删除（INV-032）
    template_repo.save(&template)?;
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
    fn delete_template_is_soft_delete() {
        // 软删除（INV-032）：delete 后 find_all 不含该模板，
        // find_all_including_deleted 含墓碑且 is_deleted() 为 true
        let repo = InMemoryTemplateRepository::new();
        let mock = MockEventPublisher::new();
        let template = make_template();
        repo.save(&template).unwrap();

        delete_template(&repo, &mock, &template.id).unwrap();

        // find_all 不含已删除模板（墓碑被过滤）
        let active = repo.find_all().unwrap();
        assert!(
            active.iter().all(|t| t.id != template.id),
            "find_all 不应返回已软删除的模板"
        );

        // find_all_including_deleted 含墓碑，且 is_deleted 为 true
        let all = repo.find_all_including_deleted().unwrap();
        let tombstone = all
            .iter()
            .find(|t| t.id == template.id)
            .expect("墓碑应保留在 find_all_including_deleted 中");
        assert!(tombstone.is_deleted());
    }

    #[test]
    fn delete_template_emits_event() {
        // delete_template 必须 emit TemplateWritten { action: Deleted, id } 事件
        let repo = InMemoryTemplateRepository::new();
        let mock = MockEventPublisher::new();
        let events = mock.events_clone();
        let template = make_template();
        repo.save(&template).unwrap();

        delete_template(&repo, &mock, &template.id).unwrap();

        let events = events.lock().unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            DomainEvent::TemplateWritten { action, id } => {
                assert_eq!(*action, WriteAction::Deleted);
                assert_eq!(id, &template.id);
            }
            _ => panic!("expected TemplateWritten"),
        }
    }

    #[test]
    fn delete_template_not_found_is_idempotent() {
        // 不存在的 id 幂等返回 Ok(())，不 emit 事件（INV-013 存在性守卫）
        let repo = InMemoryTemplateRepository::new();
        let mock = MockEventPublisher::new();
        let events = mock.events_clone();

        let result = delete_template(&repo, &mock, "nonexistent-id");

        assert!(result.is_ok());
        assert_eq!(events.lock().unwrap().len(), 0);
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
            fn find_all_including_deleted(&self) -> Result<Vec<Template>, String> {
                Ok(vec![])
            }
            fn find_by_id_including_deleted(&self, _: &str) -> Result<Option<Template>, String> {
                Ok(None)
            }
            fn physical_delete(&self, _: &str) -> Result<(), String> {
                Ok(())
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
