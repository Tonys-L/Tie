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
        // INV-032 墓碑机制：读取 deleted_at 列（NULL 表示活跃，非 NULL 表示墓碑）
        deleted_at: row.get("deleted_at")?,
    })
}

impl TemplateRepository for SqliteTemplateRepository {
    fn save(&self, template: &Template) -> Result<(), String> {
        let conn = self.db.lock()?;
        // INV-032 墓碑机制：deleted_at 列参与 INSERT/UPDATE。
        // ON CONFLICT 显式 SET deleted_at = excluded.deleted_at 是关键：
        // 复活场景（墓碑被非墓碑覆盖 save）必须用 None 覆盖旧墓碑值，否则墓碑状态会残留。
        conn.execute(
            "INSERT INTO templates (id, name, content, category, sort_order, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                content = excluded.content,
                category = excluded.category,
                sort_order = excluded.sort_order,
                updated_at = excluded.updated_at,
                deleted_at = excluded.deleted_at",
            params![
                template.id,
                template.name,
                template.content,
                template.category,
                template.sort_order,
                template.created_at,
                template.updated_at,
                template.deleted_at,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn find_all(&self) -> Result<Vec<Template>, String> {
        let conn = self.db.lock()?;
        // INV-032 墓碑机制：find_all 默认过滤墓碑（deleted_at IS NULL），业务查询用
        let mut stmt = conn
            .prepare("SELECT id, name, content, category, sort_order, created_at, updated_at, deleted_at FROM templates WHERE deleted_at IS NULL ORDER BY sort_order ASC, created_at ASC")
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
        // INV-032 墓碑机制：find_by_id 默认过滤墓碑（deleted_at IS NULL），业务查询用
        let template = conn
            .query_row(
                "SELECT id, name, content, category, sort_order, created_at, updated_at, deleted_at FROM templates WHERE id = ?1 AND deleted_at IS NULL",
                params![id],
                row_to_template,
            )
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(template)
    }

    /// INV-032 墓碑机制：含墓碑查询，供 sync import 仲裁用。
    /// 与 find_by_id 的区别：墓碑（deleted_at IS NOT NULL）也会返回。
    fn find_by_id_including_deleted(&self, id: &str) -> Result<Option<Template>, String> {
        let conn = self.db.lock()?;
        let template = conn
            .query_row(
                "SELECT id, name, content, category, sort_order, created_at, updated_at, deleted_at FROM templates WHERE id = ?1",
                params![id],
                row_to_template,
            )
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(template)
    }

    /// INV-032 墓碑机制：含墓碑查询，供 sync export 写出墓碑 JSON 用。
    /// 与 find_all 的区别：墓碑也会返回，让 export 把墓碑 JSON 写到 sync 目录传播删除。
    fn find_all_including_deleted(&self) -> Result<Vec<Template>, String> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare("SELECT id, name, content, category, sort_order, created_at, updated_at, deleted_at FROM templates ORDER BY sort_order ASC, created_at ASC")
            .map_err(|e| e.to_string())?;
        let templates = stmt
            .query_map([], row_to_template)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(templates)
    }

    /// INV-032 墓碑机制：物理删除（DELETE FROM），仅供墓碑清理使用。
    /// 正常业务删除走 domain Template::delete() + save 软删除路径。
    fn physical_delete(&self, id: &str) -> Result<(), String> {
        let conn = self.db.lock()?;
        conn.execute("DELETE FROM templates WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
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

    // ============ INV-032 墓碑机制测试 ============

    /// 创建空仓储（清空默认模板种子），用于墓碑测试避免种子数据干扰
    fn setup_empty() -> SqliteTemplateRepository {
        let repo = setup();
        {
            let conn = repo.db.lock().unwrap();
            conn.execute("DELETE FROM templates", []).unwrap();
        }
        repo
    }

    #[test]
    fn find_all_excludes_tombstones() {
        // find_all 默认过滤墓碑：2 条活跃 + 1 条墓碑 → 返回 2 条
        let repo = setup_empty();
        let t1 = Template::new("活跃1".to_string(), "a".to_string());
        let t2 = Template::new("活跃2".to_string(), "b".to_string());
        let mut t3 = Template::new("墓碑".to_string(), "c".to_string());
        t3.delete();
        repo.save(&t1).unwrap();
        repo.save(&t2).unwrap();
        repo.save(&t3).unwrap();

        let all = repo.find_all().unwrap();
        assert_eq!(all.len(), 2, "find_all 应过滤墓碑");
    }

    #[test]
    fn find_all_including_deleted_returns_tombstones() {
        // find_all_including_deleted 含墓碑：2 条活跃 + 1 条墓碑 → 返回 3 条
        let repo = setup_empty();
        let t1 = Template::new("活跃1".to_string(), "a".to_string());
        let t2 = Template::new("活跃2".to_string(), "b".to_string());
        let mut t3 = Template::new("墓碑".to_string(), "c".to_string());
        t3.delete();
        repo.save(&t1).unwrap();
        repo.save(&t2).unwrap();
        repo.save(&t3).unwrap();

        let all = repo.find_all_including_deleted().unwrap();
        assert_eq!(all.len(), 3, "find_all_including_deleted 应含墓碑");
    }

    #[test]
    fn find_by_id_excludes_tombstone() {
        // 墓碑：find_by_id 返回 None，find_by_id_including_deleted 返回 Some
        let repo = setup_empty();
        let mut tpl = Template::new("墓碑".to_string(), "x".to_string());
        let id = tpl.id.clone();
        tpl.delete();
        repo.save(&tpl).unwrap();

        assert!(
            repo.find_by_id(&id).unwrap().is_none(),
            "find_by_id 应过滤墓碑"
        );
        assert!(
            repo.find_by_id_including_deleted(&id)
                .unwrap()
                .is_some(),
            "find_by_id_including_deleted 应返回墓碑"
        );
    }

    #[test]
    fn save_tombstone_persists_deleted_at() {
        // template.delete() 后 save，重新读出 deleted_at 应为 Some
        let repo = setup_empty();
        let mut tpl = Template::new("墓碑".to_string(), "x".to_string());
        tpl.delete();
        let id = tpl.id.clone();
        repo.save(&tpl).unwrap();

        let found = repo.find_by_id_including_deleted(&id).unwrap().unwrap();
        assert!(
            found.deleted_at.is_some(),
            "墓碑的 deleted_at 应持久化"
        );
        assert!(found.is_deleted());
    }

    #[test]
    fn save_non_tombstone_clears_deleted_at() {
        // 复活场景：先保存墓碑，再用非墓碑覆盖 save，deleted_at 应被 None 清除
        let repo = setup_empty();
        let mut tpl = Template::new("将复活".to_string(), "x".to_string());
        let id = tpl.id.clone();
        tpl.delete();
        repo.save(&tpl).unwrap();

        // 复活：用非墓碑（deleted_at = None）覆盖 save
        let mut revived = tpl.clone();
        revived.deleted_at = None;
        revived.update_content("复活".to_string(), "y".to_string());
        repo.save(&revived).unwrap();

        let found = repo.find_by_id_including_deleted(&id).unwrap().unwrap();
        assert!(
            found.deleted_at.is_none(),
            "复活后 deleted_at 应被 None 覆盖清除"
        );
        assert!(!found.is_deleted());
    }

    #[test]
    fn physical_delete_removes_record() {
        // physical_delete 后，find_by_id_including_deleted 也返回 None（记录彻底消失）
        let repo = setup_empty();
        let tpl = Template::new("待物理删除".to_string(), "x".to_string());
        let id = tpl.id.clone();
        repo.save(&tpl).unwrap();

        repo.physical_delete(&id).unwrap();
        assert!(
            repo.find_by_id_including_deleted(&id)
                .unwrap()
                .is_none(),
            "physical_delete 后记录应彻底消失"
        );
    }
}
