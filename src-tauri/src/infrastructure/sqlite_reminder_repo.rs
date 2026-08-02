use std::sync::Arc;

use rusqlite::{params, OptionalExtension, Row};

use crate::domain::{Reminder, ReminderQuery, ReminderRepository, ReminderStatus, RepeatType};

use super::Database;

/// SQLite 实现的 Reminder 仓储
pub struct SqliteReminderRepository {
    db: Arc<Database>,
}

impl SqliteReminderRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

fn row_to_reminder(row: &Row) -> rusqlite::Result<Reminder> {
    Ok(Reminder {
        id: row.get("id")?,
        note_id: row.get("note_id")?,
        note_title: row.get("note_title")?,
        remind_at: row.get("remind_at")?,
        repeat_type: RepeatType::from_str(&row.get::<_, String>("repeat_type")?),
        status: ReminderStatus::from_str(&row.get::<_, String>("status")?),
        snoozed_until: row.get("snoozed_until")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        // INV-032 墓碑机制：从 DB 读取 deleted_at（NULL → None 即非墓碑）
        deleted_at: row.get("deleted_at")?,
    })
}

const SELECT_COLS: &str = "id, note_id, note_title, remind_at, repeat_type, status, snoozed_until, created_at, updated_at, deleted_at";

/// 有效触发时间的 SQL 表达式（INV-008）
///
/// 必须与 `domain::Reminder::effective_time()` 语义一致：贪睡中取 snoozed_until，否则取 remind_at。
/// 集中为此常量，避免 COALESCE 表达式散布导致规则漂移（LES-022）。
const EFFECTIVE_TIME_EXPR: &str = "COALESCE(snoozed_until, remind_at)";

impl ReminderRepository for SqliteReminderRepository {
    fn save(&self, reminder: &Reminder) -> Result<(), String> {
        let conn = self.db.lock()?;
        // INV-032 墓碑机制：用 ON CONFLICT DO UPDATE 替代 INSERT OR REPLACE
        // 原因：(1) INSERT OR REPLACE 会先 DELETE 再 INSERT，可能触发外键级联问题；
        //       (2) 必须显式 SET deleted_at = excluded.deleted_at 才能正确处理复活场景
        //       （墓碑被非墓碑覆盖时需清空 deleted_at）。
        // created_at 不在 UPDATE SET 中（保持创建时间不变）。
        conn.execute(
            "INSERT INTO reminders
                (id, note_id, note_title, remind_at, repeat_type, status, snoozed_until, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                note_id = excluded.note_id,
                note_title = excluded.note_title,
                remind_at = excluded.remind_at,
                repeat_type = excluded.repeat_type,
                status = excluded.status,
                snoozed_until = excluded.snoozed_until,
                updated_at = excluded.updated_at,
                deleted_at = excluded.deleted_at",
            params![
                reminder.id,
                reminder.note_id,
                reminder.note_title,
                reminder.remind_at,
                reminder.repeat_type.as_str(),
                reminder.status.as_str(),
                reminder.snoozed_until,
                reminder.created_at,
                reminder.updated_at,
                reminder.deleted_at,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn find_by_id(&self, id: &str) -> Result<Option<Reminder>, String> {
        // INV-032 墓碑机制：默认过滤墓碑
        let conn = self.db.lock()?;
        let sql = format!("SELECT {} FROM reminders WHERE id = ?1 AND deleted_at IS NULL", SELECT_COLS);
        let reminder = conn
            .query_row(&sql, params![id], row_to_reminder)
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(reminder)
    }

    fn find_by_id_including_deleted(&self, id: &str) -> Result<Option<Reminder>, String> {
        // INV-032 墓碑机制：含墓碑查询，供 sync import 仲裁用
        let conn = self.db.lock()?;
        let sql = format!("SELECT {} FROM reminders WHERE id = ?1", SELECT_COLS);
        let reminder = conn
            .query_row(&sql, params![id], row_to_reminder)
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(reminder)
    }

    fn find_all(&self) -> Result<Vec<Reminder>, String> {
        // INV-032 墓碑机制：默认过滤墓碑
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(&format!("SELECT {} FROM reminders WHERE deleted_at IS NULL ORDER BY remind_at ASC", SELECT_COLS))
            .map_err(|e| e.to_string())?;
        let reminders = stmt
            .query_map([], row_to_reminder)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(reminders)
    }

    fn find_all_including_deleted(&self) -> Result<Vec<Reminder>, String> {
        // INV-032 墓碑机制：含墓碑查询，供 sync export 写出墓碑 JSON 用
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(&format!("SELECT {} FROM reminders ORDER BY remind_at ASC", SELECT_COLS))
            .map_err(|e| e.to_string())?;
        let reminders = stmt
            .query_map([], row_to_reminder)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(reminders)
    }

    fn find_by_note_id(&self, note_id: &str) -> Result<Vec<Reminder>, String> {
        // INV-032 墓碑机制：默认过滤墓碑
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(&format!("SELECT {} FROM reminders WHERE note_id = ?1 AND deleted_at IS NULL ORDER BY remind_at ASC", SELECT_COLS))
            .map_err(|e| e.to_string())?;
        let reminders = stmt
            .query_map(params![note_id], row_to_reminder)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(reminders)
    }

    fn physical_delete(&self, id: &str) -> Result<(), String> {
        // INV-032 墓碑机制：物理删除，仅供墓碑清理使用
        let conn = self.db.lock()?;
        conn.execute("DELETE FROM reminders WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<(), String> {
        let conn = self.db.lock()?;
        conn.execute("DELETE FROM reminders WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn delete_by_note_id(&self, note_id: &str) -> Result<(), String> {
        let conn = self.db.lock()?;
        conn.execute("DELETE FROM reminders WHERE note_id = ?1", params![note_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Reminder 读投影实现（CQRS 风味拆分，ADR-010）
impl ReminderQuery for SqliteReminderRepository {
    fn find_due(&self, now: &str) -> Result<Vec<Reminder>, String> {
        // 只用 SQL 筛 status='pending'，到期判断委托 `Reminder::is_due`（INV-008 单一真相源）。
        // 避免 SQL 重新实现 is_due 的 snoozed/remind_at 比较逻辑导致规则漂移。
        // INV-032 墓碑机制：过滤墓碑，墓碑提醒即使到期也不触发。
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {} FROM reminders WHERE status = 'pending' AND deleted_at IS NULL ORDER BY remind_at ASC",
                SELECT_COLS
            ))
            .map_err(|e| e.to_string())?;
        let reminders: Vec<Reminder> = stmt
            .query_map([], row_to_reminder)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(reminders.into_iter().filter(|r| r.is_due(now)).collect())
    }

    fn find_next_due_time(&self) -> Result<Option<String>, String> {
        // INV-032 墓碑机制：过滤墓碑，只有墓碑处于 pending 时返回 None
        let conn = self.db.lock()?;
        let sql = format!(
            "SELECT MIN({}) AS next_time FROM reminders WHERE status = 'pending' AND deleted_at IS NULL",
            EFFECTIVE_TIME_EXPR
        );
        let result = conn
            .query_row(&sql, [], |row| row.get::<_, Option<String>>("next_time"))
            .map_err(|e| e.to_string())?;
        Ok(result)
    }

    fn find_by_date_range(&self, start: &str, end: &str) -> Result<Vec<Reminder>, String> {
        // INV-032 墓碑机制：过滤墓碑，日历视图不显示墓碑提醒
        let conn = self.db.lock()?;
        let sql = format!(
            "SELECT {} FROM reminders
             WHERE {} >= ?1 AND {} < ?2 AND deleted_at IS NULL
             ORDER BY remind_at ASC",
            SELECT_COLS, EFFECTIVE_TIME_EXPR, EFFECTIVE_TIME_EXPR
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let reminders = stmt
            .query_map(params![start, end], row_to_reminder)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(reminders)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建内存数据库，插入一条便签以满足外键约束，并返回 ReminderRepository
    fn setup() -> SqliteReminderRepository {
        let db = Arc::new(Database::new(":memory:").unwrap());
        // 插入一条便签，满足 reminders 表的外键约束
        {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO notes (id, title, content, color, opacity, pos_x, pos_y, width, height, is_pinned, created_at, updated_at)
                 VALUES ('note-1', '测试便签', '', 'amber', 1.0, 100, 100, 320, 280, 0, '2026-07-03T00:00:00Z', '2026-07-03T00:00:00Z')",
                (),
            )
            .unwrap();
        }
        SqliteReminderRepository::new(db)
    }

    /// 创建一个关联到 note-1 的提醒
    fn make_reminder(remind_at: &str, repeat_type: &str) -> Reminder {
        Reminder::new(
            "note-1".to_string(),
            "测试便签".to_string(),
            remind_at.to_string(),
            repeat_type.to_string(),
        )
    }

    #[test]
    fn test_save_and_find_by_note_id() {
        let repo = setup();
        let reminder = make_reminder("2026-07-03T10:00:00Z", "once");
        let reminder_id = reminder.id.clone();
        repo.save(&reminder).unwrap();

        let found = repo.find_by_note_id("note-1").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, reminder_id);
        assert_eq!(found[0].note_id, "note-1");
        assert_eq!(found[0].note_title, "测试便签");
    }

    #[test]
    fn test_find_due() {
        let repo = setup();
        // 创建一个已到期的提醒（过去时间）
        let reminder = make_reminder("2026-01-01T00:00:00Z", "once");
        repo.save(&reminder).unwrap();

        let due = repo.find_due("2026-07-03T00:00:00Z").unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, reminder.id);
    }

    #[test]
    fn test_find_due_not_due() {
        let repo = setup();
        // 创建一个未到期的提醒（未来时间）
        let reminder = make_reminder("2026-12-31T00:00:00Z", "once");
        repo.save(&reminder).unwrap();

        let due = repo.find_due("2026-07-03T00:00:00Z").unwrap();
        assert_eq!(due.len(), 0);
    }

    #[test]
    fn test_find_due_snoozed_due() {
        let repo = setup();
        // remind_at 已过，但 snoozed_until 也已过 → 到期
        let mut reminder = make_reminder("2026-01-01T00:00:00Z", "once");
        reminder.snoozed_until = Some("2026-01-01T00:30:00Z".to_string());
        repo.save(&reminder).unwrap();

        let due = repo.find_due("2026-07-03T00:00:00Z").unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, reminder.id);
    }

    #[test]
    fn test_find_due_snoozed_not_due() {
        let repo = setup();
        // remind_at 已过，但 snoozed_until 在未来 → 未到期（is_due 看 snoozed_until）
        let mut reminder = make_reminder("2026-01-01T00:00:00Z", "once");
        reminder.snoozed_until = Some("2026-12-31T00:00:00Z".to_string());
        repo.save(&reminder).unwrap();

        let due = repo.find_due("2026-07-03T00:00:00Z").unwrap();
        assert_eq!(due.len(), 0);
    }

    #[test]
    fn test_delete() {
        let repo = setup();
        let reminder = make_reminder("2026-07-03T10:00:00Z", "once");
        let reminder_id = reminder.id.clone();
        repo.save(&reminder).unwrap();
        assert_eq!(repo.find_by_note_id("note-1").unwrap().len(), 1);

        repo.delete(&reminder_id).unwrap();
        let found = repo.find_by_note_id("note-1").unwrap();
        assert_eq!(found.len(), 0);
    }

    // ============ 墓碑机制测试（INV-032）============
    // 覆盖软删除过滤、含墓碑查询、持久化、复活场景、物理删除

    #[test]
    fn find_all_excludes_tombstones() {
        // 2 条活跃 + 1 条墓碑，find_all() 返回 2 条
        let repo = setup();
        let r1 = make_reminder("2026-07-03T10:00:00Z", "once");
        let r2 = make_reminder("2026-07-04T10:00:00Z", "once");
        let mut r3 = make_reminder("2026-07-05T10:00:00Z", "once");
        r3.delete();
        repo.save(&r1).unwrap();
        repo.save(&r2).unwrap();
        repo.save(&r3).unwrap();

        let found = repo.find_all().unwrap();
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn find_all_including_deleted_returns_tombstones() {
        // 同上数据，find_all_including_deleted() 返回 3 条
        let repo = setup();
        let r1 = make_reminder("2026-07-03T10:00:00Z", "once");
        let r2 = make_reminder("2026-07-04T10:00:00Z", "once");
        let mut r3 = make_reminder("2026-07-05T10:00:00Z", "once");
        r3.delete();
        repo.save(&r1).unwrap();
        repo.save(&r2).unwrap();
        repo.save(&r3).unwrap();

        let found = repo.find_all_including_deleted().unwrap();
        assert_eq!(found.len(), 3);
    }

    #[test]
    fn find_by_id_excludes_tombstone() {
        // 墓碑 reminder find_by_id() 返回 None，find_by_id_including_deleted() 返回 Some
        let repo = setup();
        let mut r = make_reminder("2026-07-03T10:00:00Z", "once");
        r.delete();
        repo.save(&r).unwrap();

        assert!(repo.find_by_id(&r.id).unwrap().is_none());
        assert!(repo.find_by_id_including_deleted(&r.id).unwrap().is_some());
    }

    #[test]
    fn find_by_note_id_excludes_tombstones() {
        // 同一 note_id 下 1 条活跃 + 1 条墓碑，find_by_note_id() 返回 1 条
        let repo = setup();
        let r1 = make_reminder("2026-07-03T10:00:00Z", "once");
        let mut r2 = make_reminder("2026-07-04T10:00:00Z", "once");
        r2.delete();
        repo.save(&r1).unwrap();
        repo.save(&r2).unwrap();

        let found = repo.find_by_note_id("note-1").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, r1.id);
    }

    #[test]
    fn find_due_excludes_tombstones() {
        // 墓碑 reminder 即使到期也不在 find_due() 结果中
        let repo = setup();
        let r1 = make_reminder("2026-01-01T00:00:00Z", "once");
        let mut r2 = make_reminder("2026-02-01T00:00:00Z", "once");
        r2.delete();
        repo.save(&r1).unwrap();
        repo.save(&r2).unwrap();

        let due = repo.find_due("2026-07-03T00:00:00Z").unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, r1.id);
    }

    #[test]
    fn find_by_date_range_excludes_tombstones() {
        // 墓碑 reminder 不在 find_by_date_range() 结果中
        let repo = setup();
        let r1 = make_reminder("2026-07-03T10:00:00Z", "once");
        let mut r2 = make_reminder("2026-07-04T10:00:00Z", "once");
        r2.delete();
        repo.save(&r1).unwrap();
        repo.save(&r2).unwrap();

        let found = repo
            .find_by_date_range("2026-07-01T00:00:00Z", "2026-08-01T00:00:00Z")
            .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, r1.id);
    }

    #[test]
    fn find_next_due_time_excludes_tombstones() {
        // 只有墓碑 reminder 处于 pending 时，find_next_due_time() 返回 None
        let repo = setup();
        let mut r = make_reminder("2026-07-03T10:00:00Z", "once");
        r.delete();
        repo.save(&r).unwrap();

        let next = repo.find_next_due_time().unwrap();
        assert!(next.is_none());
    }

    #[test]
    fn save_tombstone_persists_deleted_at() {
        // reminder.delete() 后 save，重新 find_by_id_including_deleted 读出，deleted_at.is_some()
        let repo = setup();
        let mut r = make_reminder("2026-07-03T10:00:00Z", "once");
        r.delete();
        repo.save(&r).unwrap();

        let found = repo.find_by_id_including_deleted(&r.id).unwrap().unwrap();
        assert!(found.deleted_at.is_some());
        assert!(found.is_deleted());
    }

    #[test]
    fn save_non_tombstone_clears_deleted_at() {
        // 复活场景：先保存墓碑，再用非墓碑覆盖 save，读出后 deleted_at.is_none()
        let repo = setup();
        let mut r = make_reminder("2026-07-03T10:00:00Z", "once");
        let id = r.id.clone();
        // 第一步：保存为墓碑
        r.delete();
        repo.save(&r).unwrap();
        assert!(repo
            .find_by_id_including_deleted(&id)
            .unwrap()
            .unwrap()
            .deleted_at
            .is_some());

        // 第二步：用非墓碑覆盖 save（复活）
        let revived = make_reminder("2026-07-03T10:00:00Z", "once");
        let mut revived = revived;
        revived.id = id;
        repo.save(&revived).unwrap();

        let found = repo.find_by_id_including_deleted(&revived.id).unwrap().unwrap();
        assert!(found.deleted_at.is_none());
        assert!(!found.is_deleted());
    }

    #[test]
    fn physical_delete_removes_record() {
        // 保存 reminder 后 physical_delete，find_by_id_including_deleted 返回 None
        let repo = setup();
        let r = make_reminder("2026-07-03T10:00:00Z", "once");
        let id = r.id.clone();
        repo.save(&r).unwrap();
        assert!(repo.find_by_id_including_deleted(&id).unwrap().is_some());

        repo.physical_delete(&id).unwrap();
        assert!(repo.find_by_id_including_deleted(&id).unwrap().is_none());
    }
}
