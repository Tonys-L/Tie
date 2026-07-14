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
    ///
    /// HTTPS/HTTP 仓库：嵌入用户名和 token。
    /// 本地路径（以 / 或盘符开头）：直接返回，无需认证。
    pub fn auth_url(&self) -> Result<String, String> {
        if self.repo_url.is_empty() {
            return Err("未配置仓库地址".to_string());
        }
        let url = &self.repo_url;
        if let Some(rest) = url.strip_prefix("https://") {
            Ok(format!("https://{}:{}@{}", self.username, self.token, rest))
        } else if let Some(rest) = url.strip_prefix("http://") {
            Ok(format!("http://{}:{}@{}", self.username, self.token, rest))
        } else if url.starts_with('/') || url.len() > 1 && url.as_bytes()[1] == b':' {
            // 本地路径（Unix /path 或 Windows C:\path），无需认证
            Ok(url.clone())
        } else {
            Err("仅支持 HTTPS 或本地仓库地址".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "tie_config_test_{}_{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn test_default_config() {
        let config = SyncConfig::default();
        assert_eq!(config.repo_url, "");
        assert_eq!(config.branch, "main");
        assert!(!config.auto_sync);
    }

    #[test]
    fn test_load_save_roundtrip() {
        let path = temp_path();
        let config = SyncConfig {
            repo_url: "https://github.com/user/repo.git".to_string(),
            username: "user".to_string(),
            token: "token123".to_string(),
            branch: "master".to_string(),
            auto_sync: true,
        };

        config.save(&path).unwrap();
        let loaded = SyncConfig::load(&path).unwrap();

        assert_eq!(loaded.repo_url, config.repo_url);
        assert_eq!(loaded.username, config.username);
        assert_eq!(loaded.token, config.token);
        assert_eq!(loaded.branch, config.branch);
        assert_eq!(loaded.auto_sync, config.auto_sync);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let path = std::env::temp_dir().join("nonexistent_config_file.json");
        let config = SyncConfig::load(&path).unwrap();
        assert_eq!(config.branch, "main");
        assert!(!config.auto_sync);
    }

    #[test]
    fn test_auth_url_https() {
        let config = SyncConfig {
            repo_url: "https://github.com/user/repo.git".to_string(),
            username: "user".to_string(),
            token: "token123".to_string(),
            branch: "main".to_string(),
            auto_sync: false,
        };
        let url = config.auth_url().unwrap();
        assert_eq!(url, "https://user:token123@github.com/user/repo.git");
    }

    #[test]
    fn test_auth_url_http() {
        let config = SyncConfig {
            repo_url: "http://git.example.com/repo.git".to_string(),
            username: "user".to_string(),
            token: "tok".to_string(),
            branch: "main".to_string(),
            auto_sync: false,
        };
        let url = config.auth_url().unwrap();
        assert_eq!(url, "http://user:tok@git.example.com/repo.git");
    }

    #[test]
    fn test_auth_url_empty_repo() {
        let config = SyncConfig::default();
        assert!(config.auth_url().is_err());
    }

    #[test]
    fn test_auth_url_unsupported_protocol() {
        let config = SyncConfig {
            repo_url: "git@github.com:user/repo.git".to_string(),
            ..Default::default()
        };
        assert!(config.auth_url().is_err());
    }
}
