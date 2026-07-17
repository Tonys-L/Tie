use serde::{Deserialize, Serialize};

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
    fn test_window_state_default() {
        let ws = WindowState::default();
        assert_eq!(ws.width, 260);
        assert_eq!(ws.height, 220);
    }
}
