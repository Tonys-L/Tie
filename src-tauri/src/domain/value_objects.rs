use serde::{Deserialize, Serialize};

/// 便签颜色枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NoteColor {
    Amber,
    Blue,
    Pink,
    Green,
    White,
}

impl NoteColor {
    pub fn from_str(s: &str) -> Self {
        match s {
            "blue" => NoteColor::Blue,
            "pink" => NoteColor::Pink,
            "green" => NoteColor::Green,
            "white" => NoteColor::White,
            _ => NoteColor::Amber,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            NoteColor::Amber => "amber",
            NoteColor::Blue => "blue",
            NoteColor::Pink => "pink",
            NoteColor::Green => "green",
            NoteColor::White => "white",
        }
    }
}

/// 窗口状态值对象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub pos_x: i32,
    pub pos_y: i32,
    pub width: u32,
    pub height: u32,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            pos_x: 100,
            pos_y: 100,
            width: 260,
            height: 220,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_from_str() {
        assert_eq!(NoteColor::from_str("amber"), NoteColor::Amber);
        assert_eq!(NoteColor::from_str("blue"), NoteColor::Blue);
        assert_eq!(NoteColor::from_str("invalid"), NoteColor::Amber);
    }

    #[test]
    fn test_window_state_default() {
        let ws = WindowState::default();
        assert_eq!(ws.width, 260);
        assert_eq!(ws.height, 220);
    }
}
