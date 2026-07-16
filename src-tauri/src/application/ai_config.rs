use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// AI 服务配置（OpenAI 兼容）
///
/// 存储在用户本地配置目录，不随 Git 同步。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    /// 是否启用便签正文时间嗅探（默认 true）
    ///
    /// 旧版配置文件缺失该字段时通过 `serde(default)` 回退为 true。
    #[serde(default = "default_sniff_enabled")]
    pub sniff_enabled: bool,
}

fn default_sniff_enabled() -> bool {
    true
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            api_key: String::new(),
            model: String::new(),
            sniff_enabled: true,
        }
    }
}

impl AiConfig {
    /// 从指定路径读取配置，文件不存在时返回默认空值
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(AiConfig::default());
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("读取 AI 配置失败: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("解析 AI 配置失败: {}", e))
    }

    /// 保存配置到指定路径（自动创建父目录）
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("创建 AI 配置目录失败: {}", e))?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化 AI 配置失败: {}", e))?;
        std::fs::write(path, content).map_err(|e| format!("写入 AI 配置失败: {}", e))
    }

    /// 默认配置文件路径：用户配置目录下 `tie/ai_config.json`
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tie")
            .join("ai_config.json")
    }

    /// 是否已配置（api_key 非空视为已配置）
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "tie_ai_config_test_{}_{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn test_load_returns_default_when_not_configured() {
        let path = std::env::temp_dir().join("nonexistent_ai_config_file.json");
        let config = AiConfig::load(&path).unwrap();
        assert_eq!(config.base_url, "");
        assert_eq!(config.api_key, "");
        assert_eq!(config.model, "");
    }

    #[test]
    fn test_save_then_load_roundtrip() {
        let path = temp_path();
        let config = AiConfig {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-test-key".to_string(),
            model: "gpt-4o-mini".to_string(),
            sniff_enabled: true,
        };

        config.save(&path).unwrap();
        let loaded = AiConfig::load(&path).unwrap();

        assert_eq!(loaded.base_url, config.base_url);
        assert_eq!(loaded.api_key, config.api_key);
        assert_eq!(loaded.model, config.model);
        assert_eq!(loaded.sniff_enabled, config.sniff_enabled);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_is_configured_checks_api_key() {
        let unconfigured = AiConfig::default();
        assert!(!unconfigured.is_configured());

        let configured = AiConfig {
            base_url: String::new(),
            api_key: "sk-key".to_string(),
            model: String::new(),
            sniff_enabled: true,
        };
        assert!(configured.is_configured());
    }

    #[test]
    fn test_default_sniff_enabled_is_true() {
        let config = AiConfig::default();
        assert!(config.sniff_enabled, "默认 sniff_enabled 应为 true");
    }

    #[test]
    fn test_save_then_load_preserves_disabled_sniff() {
        let path = temp_path();
        let config = AiConfig {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-test-key".to_string(),
            model: "gpt-4o-mini".to_string(),
            sniff_enabled: false,
        };
        config.save(&path).unwrap();
        let loaded = AiConfig::load(&path).unwrap();
        assert!(!loaded.sniff_enabled, "保存为 false 后应能读回 false");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_old_config_without_sniff_enabled_defaults_to_true() {
        // 模拟旧版配置文件（不含 sniff_enabled 字段）
        let path = temp_path();
        let old_json = r#"{"base_url":"https://api.openai.com/v1","api_key":"sk-key","model":"gpt-4o-mini"}"#;
        std::fs::write(&path, old_json).unwrap();

        let loaded = AiConfig::load(&path).unwrap();
        assert_eq!(loaded.base_url, "https://api.openai.com/v1");
        assert_eq!(loaded.api_key, "sk-key");
        assert_eq!(loaded.model, "gpt-4o-mini");
        assert!(loaded.sniff_enabled, "旧配置文件缺失 sniff_enabled 应回退为 true");

        std::fs::remove_file(&path).ok();
    }
}
