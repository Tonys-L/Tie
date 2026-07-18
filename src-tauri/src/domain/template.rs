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
        }
    }

    pub fn update_content(&mut self, name: String, content: String) {
        self.name = name;
        self.content = content;
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}
