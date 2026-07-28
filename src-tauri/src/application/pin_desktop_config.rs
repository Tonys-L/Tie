//! 置顶免疫显示桌面配置（pin_desktop.json）。
//!
//! 设计意图：
//! - 独立于 note 领域，属于应用层全局配置
//! - 复用 data_dir_path 定位配置文件
//! - 默认开启（true），因为"置顶便签免疫 Win+D"是用户期望的默认行为
//!
//! 调用方：
//! - `window_manager`：置顶/unpin 时检查是否附加 pin()
//! - `commands/system_commands`：get_pin_desktop / set_pin_desktop 命令

use serde::{Deserialize, Serialize};

/// 置顶免疫显示桌面配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinDesktopConfig {
    /// 是否在置顶时附加 pin()，使窗口免疫 Win+D（默认 true）
    pub enabled: bool,
}

impl Default for PinDesktopConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl PinDesktopConfig {
    /// 配置文件名
    const FILENAME: &'static str = "pin_desktop.json";

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
            .map_err(|e| format!("读取置顶免疫配置失败: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("解析置顶免疫配置失败: {}", e))
    }

    /// 保存配置到文件
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path()?;
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化置顶免疫配置失败: {}", e))?;
        std::fs::write(&path, content).map_err(|e| format!("写入置顶免疫配置失败: {}", e))
    }
}
