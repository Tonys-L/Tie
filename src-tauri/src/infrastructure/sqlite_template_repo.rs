use std::sync::Arc;

use rusqlite::{params, OptionalExtension, Row};

use crate::domain::{Template, TemplateRepository};

use super::Database;

/// SQLite 实现的 Template 仓储
pub struct SqliteTemplateRepository {
    db: Arc<Database>,
}

impl SqliteTemplateRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

fn row_to_template(row: &Row) -> rusqlite::Result<Template> {
    Ok(Template {
        id: row.get("id")?,
        name: row.get("name")?,
        content: row.get("content")?,
        category: row.get("category")?,
        sort_order: row.get("sort_order")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

impl TemplateRepository for SqliteTemplateRepository {
    fn save(&self, template: &Template) -> Result<(), String> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO templates (id, name, content, category, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                content = excluded.content,
                category = excluded.category,
                sort_order = excluded.sort_order,
                updated_at = excluded.updated_at",
            params![
                template.id,
                template.name,
                template.content,
                template.category,
                template.sort_order,
                template.created_at,
                template.updated_at,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn find_all(&self) -> Result<Vec<Template>, String> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare("SELECT id, name, content, category, sort_order, created_at, updated_at FROM templates ORDER BY sort_order ASC, created_at ASC")
            .map_err(|e| e.to_string())?;
        let templates = stmt
            .query_map([], row_to_template)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(templates)
    }

    fn find_by_id(&self, id: &str) -> Result<Option<Template>, String> {
        let conn = self.db.lock()?;
        let template = conn
            .query_row(
                "SELECT id, name, content, category, sort_order, created_at, updated_at FROM templates WHERE id = ?1",
                params![id],
                row_to_template,
            )
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(template)
    }

    fn delete(&self, id: &str) -> Result<(), String> {
        let conn = self.db.lock()?;
        conn.execute("DELETE FROM templates WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> SqliteTemplateRepository {
        let db = Database::new(":memory:").unwrap();
        SqliteTemplateRepository::new(Arc::new(db))
    }

    #[test]
    fn test_save_and_find_by_id() {
        let repo = setup();
        let tpl = Template::new("测试模板".to_string(), "内容".to_string());
        let id = tpl.id.clone();
        repo.save(&tpl).unwrap();

        let found = repo.find_by_id(&id).unwrap().unwrap();
        assert_eq!(found.name, "测试模板");
        assert_eq!(found.content, "内容");
        assert_eq!(found.category, "custom");
    }

    #[test]
    fn test_find_all_ordered() {
        let repo = setup();
        // 清空默认模板
        {
            let conn = repo.db.lock().unwrap();
            conn.execute("DELETE FROM templates", []).unwrap();
        }
        let mut t1 = Template::new("模板1".to_string(), "a".to_string());
        t1.sort_order = 2;
        let mut t2 = Template::new("模板2".to_string(), "b".to_string());
        t2.sort_order = 1;
        repo.save(&t1).unwrap();
        repo.save(&t2).unwrap();

        let all = repo.find_all().unwrap();
        assert_eq!(all.len(), 2);
        // sort_order 升序：模板2 在前
        assert_eq!(all[0].name, "模板2");
        assert_eq!(all[1].name, "模板1");
    }

    #[test]
    fn test_save_updates_existing() {
        let repo = setup();
        let mut tpl = Template::new("原名".to_string(), "原内容".to_string());
        repo.save(&tpl).unwrap();
        tpl.update_content("新名".to_string(), "新内容".to_string());
        repo.save(&tpl).unwrap();

        let found = repo.find_by_id(&tpl.id).unwrap().unwrap();
        assert_eq!(found.name, "新名");
        assert_eq!(found.content, "新内容");
    }

    #[test]
    fn test_delete() {
        let repo = setup();
        let tpl = Template::new("待删除".to_string(), "x".to_string());
        let id = tpl.id.clone();
        repo.save(&tpl).unwrap();
        assert!(repo.find_by_id(&id).unwrap().is_some());
        repo.delete(&id).unwrap();
        assert!(repo.find_by_id(&id).unwrap().is_none());
    }

    #[test]
    fn test_default_templates_seeded() {
        // 首次创建数据库时应自动插入 3 个默认模板
        let db = Database::new(":memory:").unwrap();
        let repo = SqliteTemplateRepository::new(Arc::new(db));
        let all = repo.find_all().unwrap();
        assert_eq!(all.len(), 3, "应有 3 个默认模板");
        assert!(all.iter().any(|t| t.name == "空白"));
        assert!(all.iter().any(|t| t.name == "会议记录"));
        assert!(all.iter().any(|t| t.name == "待办清单"));
    }
}
