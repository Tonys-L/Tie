use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

use super::value_objects::{NoteColor, WindowState};

/// Note 聚合根 — 便签的核心领域模型
///
/// 每张便签是一个独立实体，拥有自己的内容、外观和窗口状态。
/// 业务规则集中在此结构中，不依赖任何技术实现。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub color: NoteColor,
    pub opacity: f64,
    pub window_state: WindowState,
    pub is_pinned: bool,
    pub is_archived: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl Note {
    /// 创建新便签
    pub fn new(title: String, color: String) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            content: String::new(),
            color: NoteColor::from_str(&color),
            opacity: 1.0,
            window_state: WindowState::default(),
            is_pinned: false,
            is_archived: false,
            created_at: now.clone(),
            updated_at: now,
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
        self.color = NoteColor::from_str(&color);
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

    /// 更新窗口位置和尺寸
    pub fn update_window_state(&mut self, x: i32, y: i32, width: u32, height: u32) {
        self.window_state.pos_x = x;
        self.window_state.pos_y = y;
        self.window_state.width = width;
        self.window_state.height = height;
        self.touch();
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
        assert_eq!(note.color, NoteColor::Amber);
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
        assert_eq!(note.color, NoteColor::Amber);
        note.set_color("blue".to_string());
        assert_eq!(note.color, NoteColor::Blue);
        note.set_color("pink".to_string());
        assert_eq!(note.color, NoteColor::Pink);
        note.set_color("green".to_string());
        assert_eq!(note.color, NoteColor::Green);
        note.set_color("white".to_string());
        assert_eq!(note.color, NoteColor::White);
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
}
