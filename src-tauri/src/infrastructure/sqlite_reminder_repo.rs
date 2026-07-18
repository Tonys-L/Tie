use std::sync::Arc;

use rusqlite::{params, OptionalExtension, Row};

use crate::domain::{Reminder, ReminderRepository, ReminderStatus, RepeatType};

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
    })
}

const SELECT_COLS: &str = "id, note_id, note_title, remind_at, repeat_type, status, snoozed_until, created_at, updated_at";

impl ReminderRepository for SqliteReminderRepository {
    fn save(&self, reminder: &Reminder) -> Result<(), String> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT OR REPLACE INTO reminders
                (id, note_id, note_title, remind_at, repeat_type, status, snoozed_until, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
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
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn find_by_id(&self, id: &str) -> Result<Option<Reminder>, String> {
        let conn = self.db.lock()?;
        let sql = format!("SELECT {} FROM reminders WHERE id = ?1", SELECT_COLS);
        let reminder = conn
            .query_row(&sql, params![id], row_to_reminder)
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(reminder)
    }

    fn find_all(&self) -> Result<Vec<Reminder>, String> {
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

    fn find_due(&self, now: &str) -> Result<Vec<Reminder>, String> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                &format!("SELECT {} FROM reminders
                 WHERE status = 'pending'
                   AND (snoozed_until IS NULL AND remind_at <= ?1
                        OR snoozed_until IS NOT NULL AND snoozed_until <= ?1)
                 ORDER BY remind_at ASC", SELECT_COLS),
            )
            .map_err(|e| e.to_string())?;
        let reminders = stmt
            .query_map(params![now], row_to_reminder)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(reminders)
    }

    fn find_by_note_id(&self, note_id: &str) -> Result<Vec<Reminder>, String> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(&format!("SELECT {} FROM reminders WHERE note_id = ?1 ORDER BY remind_at ASC", SELECT_COLS))
            .map_err(|e| e.to_string())?;
        let reminders = stmt
            .query_map(params![note_id], row_to_reminder)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(reminders)
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

    fn find_next_due_time(&self) -> Result<Option<String>, String> {
        let conn = self.db.lock()?;
        let result = conn
            .query_row(
                "SELECT MIN(COALESCE(snoozed_until, remind_at)) AS next_time
                 FROM reminders
                 WHERE status = 'pending'",
                [],
                |row| row.get::<_, Option<String>>("next_time"),
            )
            .map_err(|e| e.to_string())?;
        Ok(result)
    }

    fn find_by_date_range(&self, start: &str, end: &str) -> Result<Vec<Reminder>, String> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                &format!("SELECT {} FROM reminders
                 WHERE COALESCE(snoozed_until, remind_at) >= ?1
                   AND COALESCE(snoozed_until, remind_at) < ?2
                 ORDER BY remind_at ASC", SELECT_COLS),
            )
            .map_err(|e| e.to_string())?;
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
}
