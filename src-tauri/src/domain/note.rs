use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

use super::value_objects::WindowState;

/// Note 聚合根 — 便签的核心领域模型
///
/// 每张便签是一个独立实体，拥有自己的内容、外观和窗口状态。
/// 业务规则集中在此结构中，不依赖任何技术实现。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub color: String,
    pub opacity: f64,
    pub window_state: WindowState,
    pub is_pinned: bool,
    pub is_archived: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    /// 搜索高亮片段（仅搜索结果填充，FTS5 snippet 生成，格式：<mark>关键词</mark>）
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub highlight: Option<String>,
    /// 软删除时间戳：None 表示未删除，Some(ts) 表示墓碑（INV-032）
    ///
    /// delete() 时同时设 deleted_at 和 updated_at 为 now，确保 last-write-wins 仲裁
    /// 用 updated_at 比较即可正确传播删除（无需 import 改仲裁核心逻辑）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
}

/// 标签业务规则常量
const MAX_TAGS: usize = 10;
const MAX_TAG_LEN: usize = 20;

impl Note {
    /// 创建新便签
    pub fn new(title: String, color: String) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            content: String::new(),
            color,
            opacity: 1.0,
            window_state: WindowState::default(),
            is_pinned: false,
            is_archived: false,
            tags: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
            highlight: None,
            deleted_at: None,
        }
    }

    /// 更新内容
    pub fn update_content(&mut self, content: String) {
        self.content = content;
        self.touch();
    }

    /// 更新标题
    pub fn update_title(&mut self, title: String) {
        self.title = title;
        self.touch();
    }

    /// 切换颜色
    pub fn set_color(&mut self, color: String) {
        self.color = color;
        self.touch();
    }

    /// 设置透明度 (0.3 ~ 1.0)
    pub fn set_opacity(&mut self, opacity: f64) {
        let clamped = opacity.clamp(0.3, 1.0);
        self.opacity = clamped;
        self.touch();
    }

    /// 切换置顶
    pub fn toggle_pin(&mut self) {
        self.is_pinned = !self.is_pinned;
        self.touch();
    }

    /// 设置置顶状态
    pub fn set_pinned(&mut self, pinned: bool) {
        self.is_pinned = pinned;
        self.touch();
    }

    /// 归档便签
    pub fn archive(&mut self) {
        self.is_archived = true;
        self.touch();
    }

    /// 取消归档
    pub fn unarchive(&mut self) {
        self.is_archived = false;
        self.touch();
    }

    /// 判断便签是否为空（title + content 均空）
    ///
    /// 业务规则 INV-003：空便签在窗口关闭时从 DB 删除。
    /// 集中在 Note 聚合根，避免判断逻辑散布到 application 层多个模块。
    pub fn is_empty(&self) -> bool {
        self.title.is_empty() && self.content.is_empty()
    }

    /// 判断便签是否可触发提醒（未归档）
    ///
    /// 业务规则：归档便签不触发提醒。
    /// 集中在 Note 聚合根，让 reminder_scheduler 无需直接访问 is_archived 字段。
    pub fn is_reminder_eligible(&self) -> bool {
        !self.is_archived
    }

    /// 软删除：设 deleted_at = now，同时更新 updated_at = now（INV-032）
    ///
    /// 关键不变量：delete() 后 updated_at == deleted_at，
    /// 确保 last-write-wins 仲裁只需比较 updated_at 即可正确传播删除。
    /// save 时需正确写入 deleted_at 字段（None 或 Some(ts)）。
    pub fn delete(&mut self) {
        let now = Utc::now().to_rfc3339();
        self.deleted_at = Some(now.clone());
        self.updated_at = now;
    }

    /// 判断是否为墓碑（已软删除）
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// 更新窗口位置和尺寸（宽高低于最小值时自动修正）
    pub fn update_window_state(&mut self, x: i32, y: i32, width: u32, height: u32) {
        self.window_state.pos_x = x;
        self.window_state.pos_y = y;
        self.window_state.width = width.max(WindowState::MIN_WIDTH);
        self.window_state.height = height.max(WindowState::MIN_HEIGHT);
        self.touch();
    }

    /// 全量设置标签（自动 trim、去重、限制数量和长度）
    pub fn set_tags(&mut self, tags: Vec<String>) {
        let mut seen = std::collections::HashSet::new();
        self.tags = tags
            .into_iter()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty() && t.len() <= MAX_TAG_LEN)
            .filter(|t| seen.insert(t.clone()))
            .take(MAX_TAGS)
            .collect();
        self.touch();
    }

    /// 添加单个标签（自动 trim、去重）
    pub fn add_tag(&mut self, tag: String) {
        let tag = tag.trim().to_string();
        if tag.is_empty() || tag.len() > MAX_TAG_LEN {
            return;
        }
        if !self.tags.contains(&tag) && self.tags.len() < MAX_TAGS {
            self.tags.push(tag);
            self.touch();
        }
    }

    /// 删除指定标签
    pub fn remove_tag(&mut self, tag: &str) {
        let before = self.tags.len();
        self.tags.retain(|t| t != tag);
        if self.tags.len() != before {
            self.touch();
        }
    }

    /// 更新时间戳
    fn touch(&mut self) {
        self.updated_at = Utc::now().to_rfc3339();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_note() {
        let note = Note::new("测试".to_string(), "amber".to_string());
        assert!(!note.id.is_empty());
        assert_eq!(note.title, "测试");
        assert_eq!(note.color, "amber");
        assert_eq!(note.opacity, 1.0);
        assert!(!note.is_pinned);
    }

    #[test]
    fn test_update_content() {
        let mut note = Note::new("".to_string(), "blue".to_string());
        let original_time = note.updated_at.clone();
        note.update_content("新内容".to_string());
        assert_eq!(note.content, "新内容");
        assert_ne!(note.updated_at, original_time);
    }

    #[test]
    fn test_opacity_clamp() {
        let mut note = Note::new("".to_string(), "white".to_string());
        note.set_opacity(0.1);
        assert_eq!(note.opacity, 0.3);
        note.set_opacity(1.5);
        assert_eq!(note.opacity, 1.0);
    }

    #[test]
    fn test_toggle_pin() {
        let mut note = Note::new("".to_string(), "green".to_string());
        assert!(!note.is_pinned);
        note.toggle_pin();
        assert!(note.is_pinned);
        note.toggle_pin();
        assert!(!note.is_pinned);
    }

    #[test]
    fn test_set_pinned() {
        let mut note = Note::new("".to_string(), "amber".to_string());
        assert!(!note.is_pinned);
        note.set_pinned(true);
        assert!(note.is_pinned);
        note.set_pinned(true);
        assert!(note.is_pinned);
        note.set_pinned(false);
        assert!(!note.is_pinned);
    }

    #[test]
    fn test_set_color() {
        let mut note = Note::new("".to_string(), "amber".to_string());
        assert_eq!(note.color, "amber");
        note.set_color("blue".to_string());
        assert_eq!(note.color, "blue");
        note.set_color("pink".to_string());
        assert_eq!(note.color, "pink");
        note.set_color("green".to_string());
        assert_eq!(note.color, "green");
        note.set_color("white".to_string());
        assert_eq!(note.color, "white");
    }

    #[test]
    fn test_set_opacity_boundary() {
        let mut note = Note::new("".to_string(), "white".to_string());
        // 下边界 0.3
        note.set_opacity(0.3);
        assert_eq!(note.opacity, 0.3);
        // 上边界 1.0
        note.set_opacity(1.0);
        assert_eq!(note.opacity, 1.0);
    }

    #[test]
    fn test_update_window_state() {
        let mut note = Note::new("".to_string(), "white".to_string());
        let original_time = note.updated_at.clone();
        note.update_window_state(200, 300, 400, 500);
        assert_eq!(note.window_state.pos_x, 200);
        assert_eq!(note.window_state.pos_y, 300);
        assert_eq!(note.window_state.width, 400);
        assert_eq!(note.window_state.height, 500);
        assert_ne!(note.updated_at, original_time);
    }

    #[test]
    fn test_update_title() {
        let mut note = Note::new("旧标题".to_string(), "white".to_string());
        let original_time = note.updated_at.clone();
        note.update_title("新标题".to_string());
        assert_eq!(note.title, "新标题");
        assert_ne!(note.updated_at, original_time);
    }

    #[test]
    fn test_new_note_has_empty_tags() {
        let note = Note::new("测试".to_string(), "amber".to_string());
        assert!(note.tags.is_empty());
    }

    #[test]
    fn test_set_tags() {
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        note.set_tags(vec!["work".to_string(), "personal".to_string()]);
        assert_eq!(note.tags.len(), 2);
        assert!(note.tags.contains(&"work".to_string()));
        assert!(note.tags.contains(&"personal".to_string()));
    }

    #[test]
    fn test_set_tags_dedup() {
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        note.set_tags(vec!["work".to_string(), "work".to_string(), "personal".to_string()]);
        assert_eq!(note.tags.len(), 2);
    }

    #[test]
    fn test_set_tags_trim() {
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        note.set_tags(vec!["  work  ".to_string(), "personal".to_string()]);
        assert_eq!(note.tags[0], "work");
    }

    #[test]
    fn test_set_tags_empty_filtered() {
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        note.set_tags(vec!["".to_string(), "  ".to_string(), "valid".to_string()]);
        assert_eq!(note.tags.len(), 1);
        assert_eq!(note.tags[0], "valid");
    }

    #[test]
    fn test_set_tags_max_limit() {
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        let tags: Vec<String> = (0..15).map(|i| format!("tag{}", i)).collect();
        note.set_tags(tags);
        assert_eq!(note.tags.len(), 10);
    }

    #[test]
    fn test_set_tags_max_length() {
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        let long_tag = "a".repeat(21);
        note.set_tags(vec![long_tag, "valid".to_string()]);
        assert_eq!(note.tags.len(), 1);
        assert_eq!(note.tags[0], "valid");
    }

    #[test]
    fn test_add_tag() {
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        note.add_tag("work".to_string());
        assert_eq!(note.tags.len(), 1);
        assert_eq!(note.tags[0], "work");
    }

    #[test]
    fn test_add_tag_dedup() {
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        note.add_tag("work".to_string());
        note.add_tag("work".to_string());
        assert_eq!(note.tags.len(), 1);
    }

    #[test]
    fn test_add_tag_trim() {
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        note.add_tag("  work  ".to_string());
        assert_eq!(note.tags[0], "work");
    }

    #[test]
    fn test_add_tag_max_limit() {
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        for i in 0..10 {
            note.add_tag(format!("tag{}", i));
        }
        note.add_tag("overflow".to_string());
        assert_eq!(note.tags.len(), 10);
        assert!(!note.tags.contains(&"overflow".to_string()));
    }

    #[test]
    fn test_remove_tag() {
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        note.set_tags(vec!["work".to_string(), "personal".to_string()]);
        note.remove_tag("work");
        assert_eq!(note.tags.len(), 1);
        assert_eq!(note.tags[0], "personal");
    }

    #[test]
    fn test_remove_tag_not_exist() {
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        note.set_tags(vec!["work".to_string()]);
        let original_time = note.updated_at.clone();
        note.remove_tag("nonexistent");
        assert_eq!(note.tags.len(), 1);
        assert_eq!(note.updated_at, original_time);
    }

    // ============ is_empty 测试 (INV-003) ============

    #[test]
    fn test_is_empty_both_empty() {
        // title + content 均空 → is_empty 返回 true
        let mut note = Note::new(String::new(), "amber".to_string());
        note.title = String::new();
        note.content = String::new();
        assert!(note.is_empty());
    }

    #[test]
    fn test_is_empty_title_only() {
        // 仅 title 非空 → is_empty 返回 false
        let mut note = Note::new(String::new(), "amber".to_string());
        note.title = "有标题".to_string();
        note.content = String::new();
        assert!(!note.is_empty());
    }

    #[test]
    fn test_is_empty_content_only() {
        // 仅 content 非空 → is_empty 返回 false
        let mut note = Note::new(String::new(), "amber".to_string());
        note.title = String::new();
        note.content = "有内容".to_string();
        assert!(!note.is_empty());
    }

    #[test]
    fn test_is_empty_both_filled() {
        // title + content 均非空 → is_empty 返回 false
        let mut note = Note::new(String::new(), "amber".to_string());
        note.title = "标题".to_string();
        note.content = "内容".to_string();
        assert!(!note.is_empty());
    }

    // ============ is_reminder_eligible 测试 ============

    #[test]
    fn test_is_reminder_eligible_not_archived() {
        // 未归档便签可触发提醒
        let note = Note::new("测试".to_string(), "amber".to_string());
        assert!(!note.is_archived);
        assert!(note.is_reminder_eligible());
    }

    #[test]
    fn test_is_reminder_eligible_archived() {
        // 归档便签不可触发提醒
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        note.archive();
        assert!(note.is_archived);
        assert!(!note.is_reminder_eligible());
    }

    // ============ delete / is_deleted 测试 (INV-032) ============

    #[test]
    fn delete_sets_deleted_at_and_updated_at() {
        // delete() 后 deleted_at 为 Some，且 updated_at == deleted_at
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        assert!(note.deleted_at.is_none());
        assert!(!note.is_deleted());

        note.delete();

        assert!(note.deleted_at.is_some());
        assert_eq!(&note.updated_at, note.deleted_at.as_ref().unwrap());
        assert!(note.is_deleted());
    }

    #[test]
    fn is_deleted_reflects_deleted_at() {
        // is_deleted 在 delete 前后返回值正确
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        assert!(!note.is_deleted());

        note.delete();
        assert!(note.is_deleted());
    }

    #[test]
    fn delete_updates_updated_at_to_now() {
        // 10:00 创建 → 10:05 编辑 → 10:10 delete，updated_at 应为 10:10
        // 这里用字符串比较验证 delete 后 updated_at 不小于原值
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        note.updated_at = "2026-01-01T10:00:00+00:00".to_string();
        note.delete();
        // delete 后 updated_at 应被刷新为当前时间（一定大于 2026-01-01）
        assert!(note.updated_at.as_str() > "2026-01-01T10:00:00+00:00");
        assert_eq!(&note.updated_at, note.deleted_at.as_ref().unwrap());
    }
}
