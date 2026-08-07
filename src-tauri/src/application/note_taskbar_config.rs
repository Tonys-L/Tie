//! 便签窗口任务栏显示配置（note_taskbar.json）。
//!
//! 设计意图：
//! - 便签默认不显示在任务栏（产品决策：常驻多窗口不污染任务栏）
//! - 用户可在 Hub 通用设置页开启，让便签显示在任务栏
//! - 复用 data_dir_path 定位配置文件
//! - 与 pin_desktop_config 同范式，单一职责，零业务依赖
//!
//! 调用方：
//! - `window_manager`：open_note_window_with_url 创建便签时读取配置决定初始 skip_taskbar
//! - `commands/system_commands`：get_note_taskbar / set_note_taskbar 命令

use serde::{Deserialize, Serialize};

/// 便签窗口任务栏显示配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteTaskbarConfig {
    /// 便签是否显示在任务栏（默认 false，不显示）
    pub show_in_taskbar: bool,
}

impl Default for NoteTaskbarConfig {
    fn default() -> Self {
        Self { show_in_taskbar: false }
    }
}

impl NoteTaskbarConfig {
    /// 配置文件名
    const FILENAME: &'static str = "note_taskbar.json";

    /// 获取配置文件路径
    pub fn config_path() -> Result<std::path::PathBuf, String> {
        let dir = super::paths::data_dir_path()?;
        Ok(dir.join(Self::FILENAME))
    }

    /// 从文件加载配置，文件不存在时返回默认值
    pub fn load() -> Result<Self, String> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("读取便签任务栏配置失败: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("解析便签任务栏配置失败: {}", e))
    }

    /// 保存配置到文件
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path()?;
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化便签任务栏配置失败: {}", e))?;
        std::fs::write(&path, content).map_err(|e| format!("写入便签任务栏配置失败: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 默认值_show_in_taskbar_为_false() {
        let config = NoteTaskbarConfig::default();
        assert!(!config.show_in_taskbar);
    }

    #[test]
    fn save_then_load_往返一致() {
        let tmp = std::env::temp_dir().join(format!(
            "note_taskbar_test_{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let original = NoteTaskbarConfig { show_in_taskbar: true };
        let content = serde_json::to_string_pretty(&original).unwrap();
        std::fs::write(&tmp, &content).unwrap();
        let loaded: NoteTaskbarConfig =
            serde_json::from_str(&std::fs::read_to_string(&tmp).unwrap()).unwrap();
        assert!(loaded.show_in_taskbar);
        let _ = std::fs::remove_file(&tmp);
    }
}
