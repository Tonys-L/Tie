use rusqlite::Connection;
use std::sync::Mutex;

/// SQLite 数据库封装
///
/// 使用 Mutex 保护单个连接，桌面应用单用户场景下足够。
/// 若未来需要更高并发，可替换为连接池（如 r2d2）。
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// 打开或创建数据库文件，并执行建表迁移
    pub fn new(path: &str) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;

        // 性能优化：WAL 模式 + 减少同步频率
        // 默认 synchronous=FULL 每次写入都 fsync，桌面单用户场景不需要
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )
        .map_err(|e| e.to_string())?;

        Self::run_migrations(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 获取连接锁
    pub fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.conn
            .lock()
            .map_err(|_| "数据库锁中毒".to_string())
    }

    fn run_migrations(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS notes (
                id          TEXT PRIMARY KEY,
                title       TEXT NOT NULL DEFAULT '',
                content     TEXT NOT NULL DEFAULT '',
                color       TEXT NOT NULL DEFAULT 'amber',
                opacity     REAL NOT NULL DEFAULT 1.0,
                pos_x       INTEGER NOT NULL DEFAULT 100,
                pos_y       INTEGER NOT NULL DEFAULT 100,
                width       INTEGER NOT NULL DEFAULT 260,
                height      INTEGER NOT NULL DEFAULT 220,
                is_pinned   INTEGER NOT NULL DEFAULT 0,
                is_archived INTEGER NOT NULL DEFAULT 0,
                tags        TEXT NOT NULL DEFAULT '[]',
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS reminders (
                id            TEXT PRIMARY KEY,
                note_id       TEXT NOT NULL,
                note_title    TEXT NOT NULL DEFAULT '',
                remind_at     TEXT NOT NULL,
                repeat_type   TEXT NOT NULL DEFAULT 'once',
                status        TEXT NOT NULL DEFAULT 'pending',
                snoozed_until TEXT,
                created_at    TEXT NOT NULL,
                updated_at    TEXT NOT NULL,
                FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_reminders_status  ON reminders(status);
            CREATE INDEX IF NOT EXISTS idx_reminders_note_id ON reminders(note_id);

            -- 模板表（用户自定义便签模板）
            CREATE TABLE IF NOT EXISTS templates (
                id          TEXT PRIMARY KEY,
                name        TEXT NOT NULL,
                content     TEXT NOT NULL DEFAULT '',
                category    TEXT NOT NULL DEFAULT 'custom',
                sort_order  INTEGER NOT NULL DEFAULT 0,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );
            ",
        )
        .map_err(|e| e.to_string())?;

        // 已有数据库升级：检查 is_archived 列是否存在，不存在则添加
        // SQLite 不支持 ALTER TABLE ADD COLUMN IF NOT EXISTS，需手动检查
        let has_is_archived: bool = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(notes)")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| e.to_string())?;
            let mut found = false;
            for row in rows {
                if row.map_err(|e| e.to_string())? == "is_archived" {
                    found = true;
                    break;
                }
            }
            found
        };

        if !has_is_archived {
            conn.execute_batch("ALTER TABLE notes ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0;")
                .map_err(|e| e.to_string())?;
        }

        // 旧数据库升级：检查 reminders 表是否还有 repeat_config 列，有则删除
        let has_repeat_config: bool = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(reminders)")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| e.to_string())?;
            let mut found = false;
            for row in rows {
                if row.map_err(|e| e.to_string())? == "repeat_config" {
                    found = true;
                    break;
                }
            }
            found
        };

        if has_repeat_config {
            conn.execute_batch("ALTER TABLE reminders DROP COLUMN repeat_config;")
                .map_err(|e| e.to_string())?;
        }

        // 已有数据库升级：检查 notes 表是否有 tags 列，没有则添加
        let has_tags: bool = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(notes)")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| e.to_string())?;
            let mut found = false;
            for row in rows {
                if row.map_err(|e| e.to_string())? == "tags" {
                    found = true;
                    break;
                }
            }
            found
        };

        if !has_tags {
            conn.execute_batch("ALTER TABLE notes ADD COLUMN tags TEXT NOT NULL DEFAULT '[]';")
                .map_err(|e| e.to_string())?;
        }

        // FTS5 全文搜索虚拟表（外部内容模式，不复制数据）
        // 检查 FTS5 是否可用 + 虚拟表是否已存在
        let has_fts: bool = {
            let mut stmt = conn
                .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='notes_fts'")
                .map_err(|e| e.to_string())?;
            stmt.exists([]).map_err(|e| e.to_string())?
        };

        if !has_fts {
            // 创建 FTS5 虚拟表 + 触发器 + 全量同步已有数据
            conn.execute_batch(
                "CREATE VIRTUAL TABLE notes_fts USING fts5(
                    title, content, tags,
                    content=notes, content_rowid=rowid,
                    tokenize='trigram'
                );

                CREATE TRIGGER notes_ai AFTER INSERT ON notes BEGIN
                    INSERT INTO notes_fts(rowid, title, content, tags) VALUES (new.rowid, new.title, new.content, new.tags);
                END;
                CREATE TRIGGER notes_ad AFTER DELETE ON notes BEGIN
                    INSERT INTO notes_fts(notes_fts, rowid, title, content, tags) VALUES('delete', old.rowid, old.title, old.content, old.tags);
                END;
                CREATE TRIGGER notes_au AFTER UPDATE ON notes BEGIN
                    INSERT INTO notes_fts(notes_fts, rowid, title, content, tags) VALUES('delete', old.rowid, old.title, old.content, old.tags);
                    INSERT INTO notes_fts(rowid, title, content, tags) VALUES (new.rowid, new.title, new.content, new.tags);
                END;

                INSERT INTO notes_fts(rowid, title, content, tags) SELECT rowid, title, content, tags FROM notes;",
            )
            .map_err(|e| e.to_string())?;
        }

        // 默认模板：首次启动（templates 表为空）时插入 3 个预设模板
        let template_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM templates", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        if template_count == 0 {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute_batch(&format!(
                "INSERT INTO templates (id, name, content, category, sort_order, created_at, updated_at) VALUES
                ('tpl-blank', '空白', '', 'custom', 0, '{now}', '{now}'),
                ('tpl-meeting', '会议记录', '## 会议记录\n\n**日期**：\n**参会**：\n\n### 议题\n\n### 决议\n\n### 待办\n- [ ] ', 'custom', 1, '{now}', '{now}'),
                ('tpl-todo', '待办清单', '## 待办清单\n\n- [ ] \n- [ ] \n- [ ] ', 'custom', 2, '{now}', '{now}');",
            ))
            .map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_db() {
        let db = Database::new(":memory:").unwrap();
        // 验证 notes 表已创建
        let conn = db.lock().unwrap();
        let note_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))
            .unwrap();
        assert_eq!(note_count, 0);

        // 验证 reminders 表已创建
        let reminder_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM reminders", [], |row| row.get(0))
            .unwrap();
        assert_eq!(reminder_count, 0);

        // 验证索引已创建
        let index_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(index_count, 2);
    }

    #[test]
    fn test_wal_mode() {
        // WAL 模式仅对文件数据库生效，内存数据库始终使用 memory journal
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ai_notes_test_wal_{}.db", ts));
        let path_str = path.to_str().unwrap().to_string();

        {
            let db = Database::new(&path_str).unwrap();
            let conn = db.lock().unwrap();
            let mode: String = conn
                .query_row("PRAGMA journal_mode", [], |row| row.get(0))
                .unwrap();
            assert_eq!(mode, "wal");
        }

        // 清理临时文件（WAL 模式会产生 -wal 和 -shm 附属文件）
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}-wal", &path_str));
        let _ = std::fs::remove_file(format!("{}-shm", &path_str));
    }
}
