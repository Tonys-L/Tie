use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::ai_config::AiConfig;

/// AI 调用错误类型
#[derive(Debug, PartialEq)]
pub enum AiError {
    NotConfigured,
    Timeout,
    Network(String),
    ParseError(String),
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::NotConfigured => write!(f, "AI 未配置：缺少 API Key"),
            AiError::Timeout => write!(f, "AI 调用超时"),
            AiError::Network(msg) => write!(f, "网络错误: {}", msg),
            AiError::ParseError(msg) => write!(f, "解析错误: {}", msg),
        }
    }
}

impl std::error::Error for AiError {}

/// 聊天消息（OpenAI 兼容格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }
}

/// OpenAI 兼容 chat/completions 请求体
#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    max_tokens: u32,
    temperature: f32,
}

/// OpenAI 兼容 chat/completions 响应体（仅解析需要的字段）
#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<String>,
    /// DeepSeek reasoning 模型的推理过程（当 content 为空时 fallback 使用）
    #[serde(default)]
    reasoning_content: Option<String>,
}

/// AI 调用服务（OpenAI 兼容 API）
pub struct AiService {
    config: AiConfig,
    client: reqwest::Client,
    timeout: Duration,
}

impl AiService {
    pub fn new(config: AiConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            timeout: Duration::from_secs(15),
        }
    }

    /// 自定义超时时间（用于测试）
    #[cfg(test)]
    pub fn with_timeout(config: AiConfig, timeout: Duration) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            timeout,
        }
    }

    /// 调用 chat/completions，返回 assistant 消息内容
    pub async fn call(&self, messages: Vec<ChatMessage>) -> Result<String, AiError> {
        if !self.config.is_configured() {
            return Err(AiError::NotConfigured);
        }

        // 兼容用户填写 base_url 时遗漏 /v1 后缀
        // 仅对非 localhost URL 自动追加（localhost 通常是测试 mock）
        let base = self.config.base_url.trim_end_matches('/');
        let base = if !base.ends_with("/v1")
            && !base.ends_with("/v1beta1")
            && !base.contains("127.0.0.1")
            && !base.contains("localhost")
        {
            format!("{}/v1", base)
        } else {
            base.to_string()
        };
        let url = format!("{}/chat/completions", base);
        let body = ChatRequest {
            model: &self.config.model,
            messages: &messages,
            max_tokens: 2000,
            temperature: 0.3,
        };

        // 打印 AI 请求详情
        log::debug!("=== AI 请求 ===");
        log::debug!("URL: {}", url);
        log::debug!("Model: {}", self.config.model);
        for (i, msg) in messages.iter().enumerate() {
            log::debug!("[消息{}] role={}, content={}", i, msg.role, msg.content);
        }

        let request = self
            .client
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .json(&body);

        // 超时包裹整个请求+响应解析过程（包括 body 读取）
        let chat_resp = tokio::time::timeout(self.timeout, async {
            let response = request
                .send()
                .await
                .map_err(|e| AiError::Network(e.to_string()))?;

            let status = response.status();
            let status_is_success = status.is_success();
            // 读取原始响应文本用于日志
            let raw_text = response.text().await.unwrap_or_default();
            let preview: String = raw_text.chars().take(1000).collect();
            log::debug!("=== AI 响应 ===");
            log::debug!("HTTP Status: {}", status);
            log::debug!("Raw Body (前1000字符): {}", preview);

            if !status_is_success {
                return Err(AiError::Network(format!("HTTP {}: {}", status, raw_text)));
            }

            serde_json::from_str::<ChatResponse>(&raw_text)
                .map_err(|e| {
                    let preview: String = raw_text.chars().take(500).collect();
                    AiError::ParseError(format!("JSON 解析失败: {}, 原始内容: {}", e, preview))
                })
        })
        .await
        .map_err(|_| AiError::Timeout)??;

        chat_resp
            .choices
            .into_iter()
            .next()
            .and_then(|c| {
                // 优先用 content；为空时 fallback 到 reasoning_content（DeepSeek reasoning 模型）
                let content = c.message.content.filter(|s| !s.trim().is_empty());
                content.or_else(|| {
                    c.message.reasoning_content.filter(|s| !s.trim().is_empty())
                })
            })
            .ok_or_else(|| AiError::ParseError("响应内容为空（content 和 reasoning_content 均为空）".to_string()))
    }

    /// 发送轻量请求验证配置可用性
    pub async fn test_connection(&self) -> Result<String, AiError> {
        let result = self
            .call(vec![ChatMessage::user("ping")])
            .await?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_base(base_url: &str) -> AiConfig {
        AiConfig {
            base_url: base_url.to_string(),
            api_key: "sk-test-key".to_string(),
            model: "gpt-4o-mini".to_string(),
            sniff_enabled: true,
        }
    }

    #[tokio::test]
    async fn test_call_returns_not_configured_when_no_api_key() {
        let config = AiConfig::default();
        let service = AiService::new(config);
        let result = service.call(vec![ChatMessage::user("hi")]).await;
        assert_eq!(result, Err(AiError::NotConfigured));
    }

    #[tokio::test]
    async fn test_call_returns_timeout_on_slow_response() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_chunked_body(|w| {
                std::thread::sleep(Duration::from_millis(300));
                w.write_all(
                    r#"{"choices":[{"message":{"role":"assistant","content":"x"}}]}"#.as_bytes(),
                )
            })
            .create_async()
            .await;

        let service = AiService::with_timeout(config_with_base(&server.url()), Duration::from_millis(50));
        let result = service.call(vec![ChatMessage::user("hi")]).await;
        assert_eq!(result, Err(AiError::Timeout));
    }

    #[tokio::test]
    async fn test_call_returns_network_on_http_error() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/chat/completions")
            .with_status(500)
            .with_body("internal server error")
            .create_async()
            .await;

        let service = AiService::new(config_with_base(&server.url()));
        let result = service.call(vec![ChatMessage::user("hi")]).await;
        match result {
            Err(AiError::Network(msg)) => assert!(msg.contains("500"), "msg={}", msg),
            other => panic!("期望 Network 错误，实际: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_call_returns_content_on_success() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"choices":[{"message":{"role":"assistant","content":"pong"}}]}"#)
            .create_async()
            .await;

        let service = AiService::new(config_with_base(&server.url()));
        let result = service.call(vec![ChatMessage::user("ping")]).await;
        assert_eq!(result, Ok("pong".to_string()));
    }

    #[tokio::test]
    async fn test_test_connection_returns_content() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"choices":[{"message":{"role":"assistant","content":"pong"}}]}"#)
            .create_async()
            .await;

        let service = AiService::new(config_with_base(&server.url()));
        let result = service.test_connection().await;
        assert_eq!(result, Ok("pong".to_string()));
    }
}
