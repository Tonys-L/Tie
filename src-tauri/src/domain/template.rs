use serde::{Deserialize, Serialize};

/// 便签模板（用户自定义）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub content: String,
    pub category: String,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
    /// 软删除时间戳：None 表示未删除，Some(ts) 表示墓碑（INV-032）
    ///
    /// delete() 时同时设 deleted_at 和 updated_at 为 now，确保 last-write-wins 仲裁
    /// 用 updated_at 比较即可正确传播删除。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
}

impl Template {
    pub fn new(name: String, content: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: format!("tpl-{}", uuid::Uuid::new_v4()),
            name,
            content,
            category: "custom".to_string(),
            sort_order: 0,
            created_at: now.clone(),
            updated_at: now,
            deleted_at: None,
        }
    }

    pub fn update_content(&mut self, name: String, content: String) {
        self.name = name;
        self.content = content;
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// 软删除：设 deleted_at = now，同时更新 updated_at = now（INV-032）
    ///
    /// 关键不变量：delete() 后 updated_at == deleted_at，
    /// 确保 last-write-wins 仲裁只需比较 updated_at 即可正确传播删除。
    pub fn delete(&mut self) {
        let now = chrono::Utc::now().to_rfc3339();
        self.deleted_at = Some(now.clone());
        self.updated_at = now;
    }

    /// 判断是否为墓碑（已软删除）
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_sets_deleted_at_and_updated_at() {
        // delete() 后 deleted_at 为 Some，且 updated_at == deleted_at
        let mut tpl = Template::new("测试模板".to_string(), "内容".to_string());
        assert!(tpl.deleted_at.is_none());
        assert!(!tpl.is_deleted());

        tpl.delete();

        assert!(tpl.deleted_at.is_some());
        assert_eq!(&tpl.updated_at, tpl.deleted_at.as_ref().unwrap());
        assert!(tpl.is_deleted());
    }

    #[test]
    fn is_deleted_reflects_deleted_at() {
        // is_deleted 在 delete 前后返回值正确
        let mut tpl = Template::new("测试模板".to_string(), "内容".to_string());
        assert!(!tpl.is_deleted());

        tpl.delete();
        assert!(tpl.is_deleted());
    }

    #[test]
    fn delete_updates_updated_at_to_now() {
        // delete 后 updated_at 应被刷新为当前时间（一定大于 2026-01-01）
        let mut tpl = Template::new("测试模板".to_string(), "内容".to_string());
        tpl.updated_at = "2026-01-01T10:00:00+00:00".to_string();
        tpl.delete();
        assert!(tpl.updated_at.as_str() > "2026-01-01T10:00:00+00:00");
        assert_eq!(&tpl.updated_at, tpl.deleted_at.as_ref().unwrap());
    }
}
