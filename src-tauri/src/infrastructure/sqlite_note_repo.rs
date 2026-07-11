use std::sync::Arc;

use rusqlite::{params, OptionalExtension, Row};

use crate::domain::{Note, NoteColor, NoteRepository, WindowState};

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
    Ok(Note {
        id: row.get("id")?,
        title: row.get("title")?,
        content: row.get("content")?,
        color: NoteColor::from_str(&row.get::<_, String>("color")?),
        opacity: row.get("opacity")?,
        window_state: WindowState {
            pos_x: row.get("pos_x")?,
            pos_y: row.get("pos_y")?,
            width: row.get::<_, i64>("width")? as u32,
            height: row.get::<_, i64>("height")? as u32,
        },
        is_pinned: row.get::<_, i32>("is_pinned")? != 0,
        is_archived: row.get::<_, i32>("is_archived")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// 显式列名，避免 ALTER TABLE 添加的列顺序问题
const SELECT_COLS: &str = "id, title, content, color, opacity, pos_x, pos_y, width, height, is_pinned, is_archived, created_at, updated_at";

impl NoteRepository for SqliteNoteRepository {
    fn save(&self, note: &Note) -> Result<(), String> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT OR REPLACE INTO notes
                (id, title, content, color, opacity, pos_x, pos_y, width, height, is_pinned, is_archived, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                note.id,
                note.title,
                note.content,
                note.color.as_str(),
                note.opacity,
                note.window_state.pos_x,
                note.window_state.pos_y,
                note.window_state.width as i64,
                note.window_state.height as i64,
                note.is_pinned as i32,
                note.is_archived as i32,
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
        assert_eq!(found.color, NoteColor::Amber);
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
        assert_eq!(found.color, NoteColor::Blue);
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
}
