pub mod database;
pub mod sqlite_note_repo;
pub mod sqlite_reminder_repo;
pub mod sqlite_template_repo;

pub use database::Database;
pub use sqlite_note_repo::SqliteNoteRepository;
pub use sqlite_reminder_repo::SqliteReminderRepository;
pub use sqlite_template_repo::SqliteTemplateRepository;

/// mock/sqlite 仓储一致性测试
///
/// 锁定 InMemory 仓储（测试用）与 Sqlite 仓储（生产用）对相同操作返回等价结果，
/// 防止保真度缺口（如 delete 语义、排序差异）导致"测试通过但生产行为不同"的 bug。
/// 详见 LES-023（候选 1：mock/sqlite 保真度缺口）。
#[cfg(test)]
mod repo_consistency_tests {
    use super::*;
    use crate::domain::{
        mock_repo::{InMemoryNoteRepository, InMemoryReminderRepository, InMemoryTemplateRepository},
        Note, NoteRepository, Reminder, ReminderRepository, Template, TemplateRepository,
    };
    use std::sync::Arc;

    fn sqlite_note_repo() -> SqliteNoteRepository {
        SqliteNoteRepository::new(Arc::new(Database::new(":memory:").unwrap()))
    }

    /// 创建共享 Database 的 note + reminder 仓储，并插入父 note 满足外键约束。
    /// reminders 表有 `FOREIGN KEY (note_id) REFERENCES notes(id)`，
    /// 不先创建 note 会导致 sqlite 插入 reminder 时外键约束失败。
    fn sqlite_reminder_repo_with_note(note_id: &str) -> SqliteReminderRepository {
        let db = Arc::new(Database::new(":memory:").unwrap());
        let note_repo = SqliteNoteRepository::new(db.clone());
        let reminder_repo = SqliteReminderRepository::new(db);
        let mut note = Note::new("parent".into(), "amber".into());
        note.id = note_id.to_string();
        note_repo.save(&note).unwrap();
        reminder_repo
    }

    /// 创建 sqlite template 仓储并清空迁移时插入的默认模板（tpl-blank/tpl-meeting/tpl-todo）。
    /// Database::new 会在 templates 表为空时自动插入 3 个默认模板，干扰排序测试。
    fn sqlite_template_repo_empty() -> SqliteTemplateRepository {
        let db = Arc::new(Database::new(":memory:").unwrap());
        {
            let conn = db.lock().unwrap();
            conn.execute("DELETE FROM templates", []).unwrap();
        }
        SqliteTemplateRepository::new(db)
    }

    // ============ Note 一致性 ============

    #[test]
    fn note_delete_nonexistent_both_ok() {
        let mock = InMemoryNoteRepository::new();
        let sqlite = sqlite_note_repo();
        assert!(mock.delete("nonexistent").is_ok());
        assert!(sqlite.delete("nonexistent").is_ok());
    }

    #[test]
    fn note_find_all_sort_order_consistent() {
        let mock = InMemoryNoteRepository::new();
        let sqlite = sqlite_note_repo();
        // 3 条便签：n2 置顶(最早), n1 最新, n3 中间 → 期望顺序 n2, n1, n3
        let mut n1 = Note::new("最新".into(), "amber".into());
        n1.updated_at = "2026-07-03T00:00:00Z".into();
        let mut n2 = Note::new("置顶".into(), "amber".into());
        n2.is_pinned = true;
        n2.updated_at = "2026-07-01T00:00:00Z".into();
        let mut n3 = Note::new("中间".into(), "amber".into());
        n3.updated_at = "2026-07-02T00:00:00Z".into();
        for repo in [&mock as &dyn NoteRepository, &sqlite] {
            repo.save(&n1).unwrap();
            repo.save(&n2).unwrap();
            repo.save(&n3).unwrap();
        }
        let mock_list = mock.find_all().unwrap();
        let sqlite_list = sqlite.find_all().unwrap();
        assert_eq!(mock_list.len(), 3);
        assert_eq!(sqlite_list.len(), 3);
        for i in 0..3 {
            assert_eq!(mock_list[i].id, sqlite_list[i].id, "index {} 排序不一致", i);
        }
    }

    // ============ Reminder 一致性 ============

    #[test]
    fn reminder_delete_nonexistent_both_ok() {
        let mock = InMemoryReminderRepository::new();
        let sqlite = sqlite_reminder_repo_with_note("note-1");
        assert!(mock.delete("nonexistent").is_ok());
        assert!(sqlite.delete("nonexistent").is_ok());
    }

    #[test]
    fn reminder_find_all_sort_order_consistent() {
        let mock = InMemoryReminderRepository::new();
        let sqlite = sqlite_reminder_repo_with_note("note-1");
        let note_id = "note-1".to_string();
        // 3 条提醒：remind_at 顺序为 r2 < r3 < r1 → 期望 ASC 排序 r2, r3, r1
        let r1 = Reminder::new(note_id.clone(), "晚".into(), "2026-07-13T12:00:00Z".into(), "once".into());
        let r2 = Reminder::new(note_id.clone(), "早".into(), "2026-07-13T08:00:00Z".into(), "once".into());
        let r3 = Reminder::new(note_id.clone(), "中".into(), "2026-07-13T10:00:00Z".into(), "once".into());
        for repo in [&mock as &dyn ReminderRepository, &sqlite] {
            repo.save(&r1).unwrap();
            repo.save(&r2).unwrap();
            repo.save(&r3).unwrap();
        }
        let mock_list = mock.find_all().unwrap();
        let sqlite_list = sqlite.find_all().unwrap();
        assert_eq!(mock_list.len(), 3);
        assert_eq!(sqlite_list.len(), 3);
        for i in 0..3 {
            assert_eq!(mock_list[i].id, sqlite_list[i].id, "index {} 排序不一致", i);
        }
    }

    #[test]
    fn reminder_find_by_note_id_sort_order_consistent() {
        let mock = InMemoryReminderRepository::new();
        // sqlite 需要两个 note：一个共享 note + 一个 other note
        let db = Arc::new(Database::new(":memory:").unwrap());
        let note_repo = SqliteNoteRepository::new(db.clone());
        let sqlite = SqliteReminderRepository::new(db);
        let mut note_shared = Note::new("shared".into(), "amber".into());
        note_shared.id = "note-shared".into();
        let mut note_other = Note::new("other".into(), "amber".into());
        note_other.id = "note-other".into();
        note_repo.save(&note_shared).unwrap();
        note_repo.save(&note_other).unwrap();

        let note_id = "note-shared".to_string();
        let r1 = Reminder::new(note_id.clone(), "晚".into(), "2026-07-13T12:00:00Z".into(), "once".into());
        let r2 = Reminder::new(note_id.clone(), "早".into(), "2026-07-13T08:00:00Z".into(), "once".into());
        let r_other = Reminder::new("note-other".into(), "其他".into(), "2026-07-13T09:00:00Z".into(), "once".into());
        for repo in [&mock as &dyn ReminderRepository, &sqlite] {
            repo.save(&r1).unwrap();
            repo.save(&r2).unwrap();
            repo.save(&r_other).unwrap();
        }
        let mock_list = mock.find_by_note_id(&note_id).unwrap();
        let sqlite_list = sqlite.find_by_note_id(&note_id).unwrap();
        assert_eq!(mock_list.len(), 2);
        assert_eq!(sqlite_list.len(), 2);
        for i in 0..2 {
            assert_eq!(mock_list[i].id, sqlite_list[i].id, "index {} 排序不一致", i);
        }
    }

    // ============ Template 一致性 ============

    #[test]
    fn template_delete_nonexistent_both_ok() {
        let mock = InMemoryTemplateRepository::new();
        let sqlite = sqlite_template_repo_empty();
        assert!(mock.delete("nonexistent").is_ok());
        assert!(sqlite.delete("nonexistent").is_ok());
    }

    #[test]
    fn template_find_all_sort_order_consistent() {
        let mock = InMemoryTemplateRepository::new();
        let sqlite = sqlite_template_repo_empty();
        // 3 条模板：sort_order 顺序为 t2 < t1 < t3 → 期望 ASC 排序 t2, t1, t3
        let mut t1 = Template::new("中".into(), "c1".into());
        t1.sort_order = 2;
        t1.created_at = "2026-07-02T00:00:00Z".into();
        let mut t2 = Template::new("早".into(), "c2".into());
        t2.sort_order = 1;
        t2.created_at = "2026-07-01T00:00:00Z".into();
        let mut t3 = Template::new("晚".into(), "c3".into());
        t3.sort_order = 3;
        t3.created_at = "2026-07-03T00:00:00Z".into();
        for repo in [&mock as &dyn TemplateRepository, &sqlite] {
            repo.save(&t1).unwrap();
            repo.save(&t2).unwrap();
            repo.save(&t3).unwrap();
        }
        let mock_list = mock.find_all().unwrap();
        let sqlite_list = sqlite.find_all().unwrap();
        assert_eq!(mock_list.len(), 3);
        assert_eq!(sqlite_list.len(), 3);
        for i in 0..3 {
            assert_eq!(mock_list[i].id, sqlite_list[i].id, "index {} 排序不一致", i);
        }
    }
}
