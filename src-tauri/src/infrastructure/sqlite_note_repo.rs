use std::sync::Arc;

use rusqlite::{params, OptionalExtension, Row};

use crate::domain::{Note, NoteRepository, WindowState};

use super::Database;

/// SQLite 实现的 Note 仓储
pub struct SqliteNoteRepository {
    db: Arc<Database>,
}

impl SqliteNoteRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

fn row_to_note(row: &Row) -> rusqlite::Result<Note> {
    let tags_json: String = row.get("tags")?;
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    Ok(Note {
        id: row.get("id")?,
        title: row.get("title")?,
        content: row.get("content")?,
        color: row.get("color")?,
        opacity: row.get("opacity")?,
        window_state: WindowState {
            pos_x: row.get("pos_x")?,
            pos_y: row.get("pos_y")?,
            width: row.get::<_, i64>("width")? as u32,
            height: row.get::<_, i64>("height")? as u32,
        },
        is_pinned: row.get::<_, i32>("is_pinned")? != 0,
        is_archived: row.get::<_, i32>("is_archived")? != 0,
        tags,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        highlight: None,
    })
}

/// 显式列名，避免 ALTER TABLE 添加的列顺序问题
const SELECT_COLS: &str = "id, title, content, color, opacity, pos_x, pos_y, width, height, is_pinned, is_archived, tags, created_at, updated_at";

impl NoteRepository for SqliteNoteRepository {
    fn save(&self, note: &Note) -> Result<(), String> {
        let conn = self.db.lock()?;
        // 使用 INSERT ... ON CONFLICT DO UPDATE（UPSERT）而非 INSERT OR REPLACE
        // INSERT OR REPLACE 会先 DELETE 再 INSERT，触发 ON DELETE CASCADE 级联删除 reminders
        conn.execute(
            "INSERT INTO notes
                (id, title, content, color, opacity, pos_x, pos_y, width, height, is_pinned, is_archived, tags, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                content = excluded.content,
                color = excluded.color,
                opacity = excluded.opacity,
                pos_x = excluded.pos_x,
                pos_y = excluded.pos_y,
                width = excluded.width,
                height = excluded.height,
                is_pinned = excluded.is_pinned,
                is_archived = excluded.is_archived,
                tags = excluded.tags,
                updated_at = excluded.updated_at",
            params![
                note.id,
                note.title,
                note.content,
                &note.color,
                note.opacity,
                note.window_state.pos_x,
                note.window_state.pos_y,
                note.window_state.width as i64,
                note.window_state.height as i64,
                note.is_pinned as i32,
                note.is_archived as i32,
                serde_json::to_string(&note.tags).unwrap_or_else(|_| "[]".to_string()),
                note.created_at,
                note.updated_at,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn find_by_id(&self, id: &str) -> Result<Option<Note>, String> {
        let conn = self.db.lock()?;
        let sql = format!("SELECT {} FROM notes WHERE id = ?1", SELECT_COLS);
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| e.to_string())?;
        let note = stmt
            .query_row(params![id], row_to_note)
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(note)
    }

    fn find_all(&self) -> Result<Vec<Note>, String> {
        let conn = self.db.lock()?;
        let sql = format!("SELECT {} FROM notes WHERE is_archived = 0 ORDER BY is_pinned DESC, updated_at DESC", SELECT_COLS);
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| e.to_string())?;
        let notes = stmt
            .query_map([], row_to_note)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(notes)
    }

    fn delete(&self, id: &str) -> Result<(), String> {
        let conn = self.db.lock()?;
        conn.execute("DELETE FROM notes WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn find_archived(&self) -> Result<Vec<Note>, String> {
        let conn = self.db.lock()?;
        let sql = format!("SELECT {} FROM notes WHERE is_archived = 1 ORDER BY updated_at DESC", SELECT_COLS);
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| e.to_string())?;
        let notes = stmt
            .query_map([], row_to_note)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(notes)
    }

    fn search_notes(&self, query: &str) -> Result<Vec<Note>, String> {
        let conn = self.db.lock()?;
        let trimmed = query.trim();
        // trigram tokenizer 要求至少 3 个字符才能生成 trigram；
        // 短查询（<3 字符）回退到 LIKE 模糊匹配，保证用户体验
        if trimmed.chars().count() < 3 {
            let like = format!("%{}%", trimmed);
            let sql = format!(
                "SELECT {cols} FROM notes
                 WHERE title LIKE ?1 OR content LIKE ?1 OR tags LIKE ?1
                 ORDER BY is_pinned DESC, updated_at DESC",
                cols = SELECT_COLS
            );
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let notes = stmt
                .query_map(params![like], |row| row_to_note(row))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            return Ok(notes);
        }
        // FTS5 MATCH 查询：标题/内容/标签匹配（trigram tokenizer 支持中文子串）
        // 对 title(0)/content(1)/tags(2) 三列都生成 snippet，Rust 中选第一个含 <mark> 的返回。
        // 原因：固定查某列时，若该列无匹配词，snippet 返回该列开头纯文本（无 <mark>），用户看不到高亮。
        // 注意：JOIN notes_fts 和 notes 后存在同名列，必须用 n. 前缀限定所有列
        const SELECT_COLS_QUALIFIED: &str = "n.id, n.title, n.content, n.color, n.opacity, n.pos_x, n.pos_y, n.width, n.height, n.is_pinned, n.is_archived, n.tags, n.created_at, n.updated_at";
        let sql = format!(
            "SELECT {cols},
                    snippet(notes_fts, 0, '<mark>', '</mark>', '...', 24) as hl_title,
                    snippet(notes_fts, 1, '<mark>', '</mark>', '...', 24) as hl_content,
                    snippet(notes_fts, 2, '<mark>', '</mark>', '...', 24) as hl_tags
             FROM notes_fts f
             JOIN notes n ON f.rowid = n.rowid
             WHERE notes_fts MATCH ?1
             ORDER BY n.is_pinned DESC, rank",
            cols = SELECT_COLS_QUALIFIED
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let notes = stmt
            .query_map(params![trimmed], |row| {
                let mut note = row_to_note(row)?;
                let hl_title: String = row.get("hl_title")?;
                let hl_content: String = row.get("hl_content")?;
                let hl_tags: String = row.get("hl_tags")?;
                // 优先级：title > content > tags，选第一个含 <mark> 的片段
                note.highlight = Some(
                    [hl_title, hl_content, hl_tags]
                        .into_iter()
                        .find(|s| s.contains("<mark>"))
                        .unwrap_or_default(),
                );
                Ok(note)
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(notes)
    }

    fn find_activity_by_month(&self, year: i32, month: u32) -> Result<Vec<u32>, String> {
        let start = chrono::NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or("无效年月")?;
        let end = if month == 12 {
            chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .ok_or("无效年月")?;
        let start_str = start.and_hms_opt(0, 0, 0).unwrap().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let end_str = end.and_hms_opt(0, 0, 0).unwrap().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT CAST(strftime('%d', updated_at) AS INTEGER) AS day
                 FROM notes
                 WHERE updated_at >= ?1 AND updated_at < ?2
                 UNION
                 SELECT DISTINCT CAST(strftime('%d', created_at) AS INTEGER) AS day
                 FROM notes
                 WHERE created_at >= ?1 AND created_at < ?2",
            )
            .map_err(|e| e.to_string())?;
        let days = stmt
            .query_map(params![start_str, end_str], |row| row.get::<_, u32>("day"))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(days)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> SqliteNoteRepository {
        let db = Database::new(":memory:").unwrap();
        SqliteNoteRepository::new(Arc::new(db))
    }

    #[test]
    fn test_save_and_find_by_id() {
        let repo = setup();
        let note = Note::new("测试便签".to_string(), "amber".to_string());
        let id = note.id.clone();
        repo.save(&note).unwrap();

        let found = repo.find_by_id(&id).unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, note.id);
        assert_eq!(found.title, "测试便签");
        assert_eq!(found.color, "amber");
        assert_eq!(found.opacity, 1.0);
        assert!(!found.is_pinned);
    }

    #[test]
    fn test_find_all() {
        let repo = setup();
        let n1 = Note::new("n1".to_string(), "amber".to_string());
        let n2 = Note::new("n2".to_string(), "blue".to_string());
        let n3 = Note::new("n3".to_string(), "pink".to_string());
        repo.save(&n1).unwrap();
        repo.save(&n2).unwrap();
        repo.save(&n3).unwrap();

        let all = repo.find_all().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_find_all_ordering() {
        let repo = setup();
        let n1 = Note::new("普通便签".to_string(), "amber".to_string());
        let mut n2 = Note::new("置顶便签".to_string(), "blue".to_string());
        n2.toggle_pin();
        repo.save(&n1).unwrap();
        repo.save(&n2).unwrap();

        let all = repo.find_all().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, n2.id);
        assert!(all[0].is_pinned);
        assert_eq!(all[1].id, n1.id);
        assert!(!all[1].is_pinned);
    }

    #[test]
    fn test_save_updates_existing() {
        let repo = setup();
        let mut note = Note::new("标题".to_string(), "amber".to_string());
        let id = note.id.clone();
        repo.save(&note).unwrap();

        note.update_content("更新后的内容".to_string());
        repo.save(&note).unwrap();

        let found = repo.find_by_id(&id).unwrap().unwrap();
        assert_eq!(found.content, "更新后的内容");
        assert_eq!(found.title, "标题");
    }

    #[test]
    fn test_save_style_change() {
        let repo = setup();
        let mut note = Note::new("标题".to_string(), "amber".to_string());
        let id = note.id.clone();
        repo.save(&note).unwrap();

        note.set_color("blue".to_string());
        note.set_opacity(0.5);
        note.set_pinned(true);
        repo.save(&note).unwrap();

        let found = repo.find_by_id(&id).unwrap().unwrap();
        assert_eq!(found.color, "blue");
        assert_eq!(found.opacity, 0.5);
        assert!(found.is_pinned);
    }

    #[test]
    fn test_save_window_state_change() {
        let repo = setup();
        let mut note = Note::new("标题".to_string(), "amber".to_string());
        let id = note.id.clone();
        repo.save(&note).unwrap();

        note.update_window_state(200, 300, 400, 500);
        repo.save(&note).unwrap();

        let found = repo.find_by_id(&id).unwrap().unwrap();
        assert_eq!(found.window_state.pos_x, 200);
        assert_eq!(found.window_state.pos_y, 300);
        assert_eq!(found.window_state.width, 400);
        assert_eq!(found.window_state.height, 500);
    }

    #[test]
    fn test_save_does_not_cascade_delete_reminders() {
        // 回归测试：INSERT OR REPLACE 会触发 ON DELETE CASCADE 删除 reminders
        // 改用 ON CONFLICT DO UPDATE 后，save 不应删除关联的 reminders
        use crate::domain::{Reminder, ReminderRepository};
        use super::super::SqliteReminderRepository;

        let db = Database::new(":memory:").unwrap();
        let arc_db = std::sync::Arc::new(db);
        let note_repo = SqliteNoteRepository::new(arc_db.clone());
        let reminder_repo = SqliteReminderRepository::new(arc_db);

        let note = Note::new("测试便签".to_string(), "amber".to_string());
        let note_id = note.id.clone();
        note_repo.save(&note).unwrap();

        // 创建提醒
        let reminder = Reminder::new(
            note_id.clone(),
            "测试便签".to_string(),
            "2026-07-15T10:00:00.000Z".to_string(),
            "none".to_string(),
        );
        let reminder_id = reminder.id.clone();
        reminder_repo.save(&reminder).unwrap();

        // 模拟窗口 resize：多次 save note
        let mut note2 = note.clone();
        for i in 0..5 {
            note2.update_window_state(i * 100, i * 100, 300, 200);
            note_repo.save(&note2).unwrap();
        }

        // 验证提醒仍然存在
        let reminders = reminder_repo.find_by_note_id(&note_id).unwrap();
        assert_eq!(reminders.len(), 1, "save 后提醒不应被删除");
        assert_eq!(reminders[0].id, reminder_id);
    }

    #[test]
    fn test_delete() {
        let repo = setup();
        let note = Note::new("标题".to_string(), "amber".to_string());
        let id = note.id.clone();
        repo.save(&note).unwrap();
        assert!(repo.find_by_id(&id).unwrap().is_some());

        repo.delete(&id).unwrap();
        let found = repo.find_by_id(&id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_find_by_id_not_exist() {
        let repo = setup();
        let found = repo.find_by_id("non-existent-id").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_save_and_read_tags() {
        let repo = setup();
        let mut note = Note::new("测试标签".to_string(), "amber".to_string());
        note.set_tags(vec!["work".to_string(), "personal".to_string()]);
        let id = note.id.clone();
        repo.save(&note).unwrap();

        let found = repo.find_by_id(&id).unwrap().unwrap();
        assert_eq!(found.tags.len(), 2);
        assert!(found.tags.contains(&"work".to_string()));
        assert!(found.tags.contains(&"personal".to_string()));
    }

    #[test]
    fn test_save_empty_tags() {
        let repo = setup();
        let note = Note::new("无标签".to_string(), "amber".to_string());
        let id = note.id.clone();
        repo.save(&note).unwrap();

        let found = repo.find_by_id(&id).unwrap().unwrap();
        assert!(found.tags.is_empty());
    }

    #[test]
    fn test_search_by_title() {
        let repo = setup();
        let n1 = Note::new("Rust 学习笔记".to_string(), "amber".to_string());
        let n2 = Note::new("日常记录".to_string(), "blue".to_string());
        repo.save(&n1).unwrap();
        repo.save(&n2).unwrap();

        let results = repo.search_notes("Rust").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, n1.id);
    }

    #[test]
    fn test_search_by_content() {
        let repo = setup();
        let mut n1 = Note::new("笔记1".to_string(), "amber".to_string());
        n1.update_content("今天学习了 Rust 的所有权机制".to_string());
        let n2 = Note::new("笔记2".to_string(), "blue".to_string());
        repo.save(&n1).unwrap();
        repo.save(&n2).unwrap();

        let results = repo.search_notes("所有权").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, n1.id);
    }

    #[test]
    fn test_search_by_tags() {
        let repo = setup();
        let mut n1 = Note::new("笔记1".to_string(), "amber".to_string());
        n1.set_tags(vec!["work".to_string(), "meeting".to_string()]);
        let mut n2 = Note::new("笔记2".to_string(), "blue".to_string());
        n2.set_tags(vec!["personal".to_string()]);
        repo.save(&n1).unwrap();
        repo.save(&n2).unwrap();

        let results = repo.search_notes("meeting").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, n1.id);
    }

    #[test]
    fn test_search_no_match() {
        let repo = setup();
        let n1 = Note::new("笔记1".to_string(), "amber".to_string());
        repo.save(&n1).unwrap();

        let results = repo.search_notes("不存在的关键词").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_pinned_first() {
        let repo = setup();
        let n1 = Note::new("普通笔记".to_string(), "amber".to_string());
        let mut n2 = Note::new("置顶笔记".to_string(), "blue".to_string());
        n2.set_pinned(true);
        repo.save(&n1).unwrap();
        repo.save(&n2).unwrap();

        let results = repo.search_notes("笔记").unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].is_pinned);
    }
}
