use std::path::Path;

use serde::{Deserialize, Serialize};

/// Git 同步配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub repo_url: String,
    pub username: String,
    pub token: String,
    pub branch: String,
    pub auto_sync: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            repo_url: String::new(),
            username: String::new(),
            token: String::new(),
            branch: "main".to_string(),
            auto_sync: false,
        }
    }
}

impl SyncConfig {
    /// 从文件读取配置
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(SyncConfig::default());
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("读取配置失败: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("解析配置失败: {}", e))
    }

    /// 保存配置到文件
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化配置失败: {}", e))?;
        std::fs::write(path, content).map_err(|e| format!("写入配置失败: {}", e))
    }

    /// 获取带认证的 URL
    pub fn auth_url(&self) -> Result<String, String> {
        if self.repo_url.is_empty() {
            return Err("未配置仓库地址".to_string());
        }
        let url = &self.repo_url;
        if let Some(rest) = url.strip_prefix("https://") {
            Ok(format!("https://{}:{}@{}", self.username, self.token, rest))
        } else if let Some(rest) = url.strip_prefix("http://") {
            Ok(format!("http://{}:{}@{}", self.username, self.token, rest))
        } else {
            Err("仅支持 HTTPS 仓库地址".to_string())
        }
    }
}
