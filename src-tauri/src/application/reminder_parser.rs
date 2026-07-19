use serde::{Deserialize, Serialize};

use super::ai_config::AiConfig;
use super::ai_service::{AiError, AiService};
use super::prompts::reminder::build_reminder_messages;
use super::prompts::sniff::build_sniff_messages;

/// 自然语言解析后的提醒草稿（前端用于预填表单）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderDraft {
    pub title: String,
    pub start_time: String,
    pub repeat_type: String,
    pub repeat_day: Option<u32>,
}

/// 便签正文嗅探结果
///
/// 作为 reminder 类型的数据载体，序列化后放入 `Suggestion.data`。
/// `detected` 字段在旧版（单结果）格式中由 AI 返回；新版（建议列表）格式中由后端置为 true。
/// 加 `#[serde(default)]` 以便从新版 AI 返回的建议项（不含 `detected`）反序列化。
/// 派生 `Serialize` 以便放入 `Suggestion.data`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniffResult {
    #[serde(default)]
    pub detected: bool,
    #[serde(default)]
    pub time_text: String,
    #[serde(default)]
    pub start_time: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub repeat_type: String,
    #[serde(default)]
    pub repeat_day: Option<u32>,
}

/// 通用建议
///
/// AI 扫描便签后返回的建议项，通过 `type` 字段区分类型。
/// 支持 5 种类型：reminder / todo_split / tidy / style / tag_suggest。
/// `data` 为类型相关数据：
/// - reminder: { detected, time_text, start_time, title, repeat_type, repeat_day }
/// - todo_split: Vec<String>（todos 数组）
/// - tidy: String（tidy_text）
/// - style: { style_type, styled_text }
/// - tag_suggest: Vec<String>（tags 数组）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    /// 建议类型：reminder / todo_split / tidy / style / tag_suggest
    pub r#type: String,
    /// 简短标题（如"添加提醒"/"拆分为待办"/"规整文本"/"更正式"/"推荐标签"）
    pub title: String,
    /// 详细描述（如"检测到"明天上午9点"，可添加提醒"）
    pub description: String,
    /// 类型相关数据（结构因 type 而异，见结构体文档）
    pub data: serde_json::Value,
}

/// 解析自然语言文本为提醒草稿
///
/// 流程：build_reminder_messages → AiService::call → 解析 JSON → ReminderDraft
pub async fn parse_reminder_natural(text: &str, config: &AiConfig) -> Result<ReminderDraft, AiError> {
    let messages = build_reminder_messages(text);
    let service = AiService::new(config.clone());
    let content = service.call(messages).await?;
    parse_reminder_json(&content)
}

/// AI 嗅探响应外层结构
///
/// AI 返回 `{"suggestions": [...]}`，每个建议项含 `type` 及类型相关字段。
#[derive(Debug, Deserialize)]
struct SniffResponse {
    #[serde(default)]
    suggestions: Vec<serde_json::Value>,
}

/// 嗅探便签正文，返回通用建议列表
///
/// 流程：
/// 1. 未配置 AI（api_key 为空）→ 返回 `Ok(vec![])`，静默跳过
/// 2. 用户关闭嗅探（`sniff_enabled=false`）→ 返回 `Ok(vec![])`，静默跳过
/// 3. 调用 AI：build_sniff_messages → AiService::call → 解析 JSON → SniffResponse
/// 4. AI 返回 `{"suggestions": []}` → 返回 `Ok(vec![])`
/// 5. AI 返回含建议 → 遍历建议项，按 `type` 包装为 `Suggestion` 返回
///
/// 支持 5 种建议类型：
/// - `reminder`：解析为 `SniffResult`（`detected` 置 true），data = SniffResult 序列化
/// - `todo_split`：data = `Vec<String>`（todos 数组）
/// - `tidy`：data = `String`（tidy_text）
/// - `style`：data = `{"style_type": String, "styled_text": String}`
/// - `tag_suggest`：data = `Vec<String>`（tags 数组）
///
/// 未知类型跳过。各类型在数据为空时跳过（不 push 建议）。
pub async fn sniff_suggestions(content: &str, config: &AiConfig) -> Result<Vec<Suggestion>, AiError> {
    // 未配置：静默跳过
    if !config.is_configured() {
        log::info!("AI分析跳过：AI 未配置（api_key 为空）");
        return Ok(vec![]);
    }
    // sniff_enabled 仅控制保存时自动分析，手动触发（灯泡）不受此开关限制
    // 调用 AI
    let messages = build_sniff_messages(content);
    let service = AiService::new(config.clone());
    let resp = service.call(messages).await?;
    // AI 返回空内容则无建议
    if resp.trim().is_empty() {
        log::debug!("嗅探：AI 返回空内容");
        return Ok(vec![]);
    }
    let resp_preview: String = resp.chars().take(1000).collect();
    log::debug!("=== 嗅探解析 ===");
    log::debug!("AI 返回内容: {}", resp_preview);
    // 4. 解析 JSON（兼容 markdown 代码块或附带解释文字）
    let json_str = extract_json(&resp);
    log::debug!("extract_json 提取后: {}", json_str);
    if json_str.trim().is_empty() {
        let preview: String = resp.chars().take(200).collect();
        log::warn!("嗅探 AI 返回内容中未找到 JSON: {}", preview);
        return Ok(vec![]);
    }
    let response: SniffResponse = serde_json::from_str(json_str)
        .map_err(|e| {
            let preview: String = resp.chars().take(200).collect();
            log::error!("嗅探 JSON 解析失败: {}, json_str: {}", e, json_str);
            AiError::ParseError(format!("嗅探结果 JSON 解析失败: {}，原始内容: {}", e, preview))
        })?;
    log::debug!("解析到 {} 条建议", response.suggestions.len());
    // 5. 遍历建议项，按 type 包装为 Suggestion
    let mut suggestions = Vec::new();
    for item in response.suggestions {
        let item_type = item
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        // 兼容两种格式：字段在顶层 或 嵌套在 data 里
        let data_obj = item.get("data").unwrap_or(&item);
        match item_type.as_str() {
            "reminder" => {
                let mut sniff_result: SniffResult = serde_json::from_value(data_obj.clone())
                    .map_err(|e| AiError::ParseError(format!("reminder 建议解析失败: {}", e)))?;
                sniff_result.detected = true;
                suggestions.push(Suggestion {
                    r#type: "reminder".to_string(),
                    title: "添加提醒".to_string(),
                    description: format!("检测到\"{}\"，可添加提醒", sniff_result.time_text),
                    data: serde_json::to_value(&sniff_result).unwrap_or(serde_json::Value::Null),
                });
            }
            "todo_split" => {
                let todos: Vec<String> = data_obj
                    .get("todos")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                if !todos.is_empty() {
                    suggestions.push(Suggestion {
                        r#type: "todo_split".to_string(),
                        title: "拆分为待办".to_string(),
                        description: format!("检测到{}项可拆分任务", todos.len()),
                        data: serde_json::to_value(&todos).unwrap_or(serde_json::Value::Null),
                    });
                }
            }
            "tidy" => {
                let tidy_text = data_obj
                    .get("tidy_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !tidy_text.is_empty() {
                    suggestions.push(Suggestion {
                        r#type: "tidy".to_string(),
                        title: "规整文本".to_string(),
                        description: "口语化文本可规整为书面表达".to_string(),
                        data: serde_json::Value::String(tidy_text),
                    });
                }
            }
            "style" => {
                let style_type = data_obj
                    .get("style_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let styled_text = data_obj
                    .get("styled_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !styled_text.is_empty() {
                    let style_label = match style_type.as_str() {
                        "formal" => "更正式",
                        "concise" => "更精简",
                        "gentle" => "更温和",
                        _ => "文风优化",
                    };
                    suggestions.push(Suggestion {
                        r#type: "style".to_string(),
                        title: style_label.to_string(),
                        description: "调整文风以适应正式场景".to_string(),
                        data: serde_json::json!({
                            "style_type": style_type,
                            "styled_text": styled_text,
                        }),
                    });
                }
            }
            "tag_suggest" => {
                let tags: Vec<String> = data_obj
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                if !tags.is_empty() {
                    suggestions.push(Suggestion {
                        r#type: "tag_suggest".to_string(),
                        title: "推荐标签".to_string(),
                        description: format!("推荐：{}", tags.join("、")),
                        data: serde_json::to_value(&tags).unwrap_or(serde_json::Value::Null),
                    });
                }
            }
            _ => {
                // 未知类型，跳过
            }
        }
    }
    Ok(suggestions)
}

/// 从 AI 返回内容中解析 ReminderDraft
///
/// 自动提取 JSON 片段（兼容 AI 返回 markdown 代码块或附带解释文字的情况）。
pub fn parse_reminder_json(content: &str) -> Result<ReminderDraft, AiError> {
    let json_str = extract_json(content);
    let draft: ReminderDraft = serde_json::from_str(json_str)
        .map_err(|e| AiError::ParseError(format!("JSON 解析失败: {}", e)))?;

    // 字段校验
    if draft.title.is_empty() {
        return Err(AiError::ParseError("缺少 title 字段".to_string()));
    }
    if draft.start_time.is_empty() {
        return Err(AiError::ParseError("缺少 start_time 字段".to_string()));
    }
    if draft.repeat_type.is_empty() {
        return Err(AiError::ParseError("缺少 repeat_type 字段".to_string()));
    }

    Ok(draft)
}

/// 从文本中提取最外层 JSON 对象片段
fn extract_json(content: &str) -> &str {
    let start = match content.find('{') {
        Some(idx) => idx,
        None => return content,
    };
    let end = match content.rfind('}') {
        Some(idx) => idx + 1,
        None => return content,
    };
    if end <= start {
        return content;
    }
    &content[start..end]
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

    fn ai_response(content: &str) -> String {
        format!(
            r#"{{"choices":[{{"message":{{"role":"assistant","content":{}}}}}]}}"#,
            serde_json::Value::String(content.to_string())
        )
    }

    #[tokio::test]
    async fn test_parse_reminder_returns_draft_on_valid_json() {
        let mut server = mockito::Server::new_async().await;
        let inner = r#"{"title":"开会","start_time":"2026-07-17 15:00","repeat_type":"once","repeat_day":null}"#;
        let _m = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(ai_response(inner))
            .create_async()
            .await;

        let config = config_with_base(&server.url());
        let draft = parse_reminder_natural("明天下午 3 点提醒我开会", &config).await.unwrap();

        assert_eq!(draft.title, "开会");
        assert_eq!(draft.start_time, "2026-07-17 15:00");
        assert_eq!(draft.repeat_type, "once");
        assert_eq!(draft.repeat_day, None);
    }

    #[tokio::test]
    async fn test_parse_reminder_returns_parse_error_on_invalid_json() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(ai_response("这不是 JSON 内容"))
            .create_async()
            .await;

        let config = config_with_base(&server.url());
        let result = parse_reminder_natural("随便说点", &config).await;
        match result {
            Err(AiError::ParseError(_)) => {}
            other => panic!("期望 ParseError，实际: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_parse_reminder_returns_parse_error_on_missing_field() {
        let mut server = mockito::Server::new_async().await;
        // 缺少 start_time 字段
        let inner = r#"{"title":"开会","repeat_type":"once","repeat_day":null}"#;
        let _m = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(ai_response(inner))
            .create_async()
            .await;

        let config = config_with_base(&server.url());
        let result = parse_reminder_natural("提醒我开会", &config).await;
        match result {
            Err(AiError::ParseError(msg)) => assert!(msg.contains("start_time"), "msg={}", msg),
            other => panic!("期望 ParseError（缺少 start_time），实际: {:?}", other),
        }
    }

    #[test]
    fn test_extract_json_strips_markdown_code_block() {
        let content = "```json\n{\"title\":\"x\"}\n```";
        let extracted = extract_json(content);
        assert_eq!(extracted, "{\"title\":\"x\"}");
    }

    #[test]
    fn test_parse_reminder_json_with_monthly_repeat_day() {
        let content = r#"{"title":"月度报表","start_time":"2026-08-01 09:00","repeat_type":"monthly","repeat_day":1}"#;
        let draft = parse_reminder_json(content).unwrap();
        assert_eq!(draft.repeat_type, "monthly");
        assert_eq!(draft.repeat_day, Some(1));
    }

    #[test]
    fn test_parse_reminder_json_missing_title_returns_error() {
        let content = r#"{"start_time":"2026-08-01 09:00","repeat_type":"once","repeat_day":null}"#;
        let result = parse_reminder_json(content);
        match result {
            Err(AiError::ParseError(msg)) => assert!(msg.contains("title")),
            other => panic!("期望 ParseError（缺少 title），实际: {:?}", other),
        }
    }

    // ============ sniff_suggestions 测试 ============

    #[tokio::test]
    async fn test_sniff_returns_empty_when_not_configured() {
        // api_key 为空 → 未配置，静默跳过
        let config = AiConfig::default();
        let result = sniff_suggestions("明天上午9点开会", &config).await;
        assert!(result.is_ok(), "未配置应返回 Ok(vec![]) 而非错误");
        assert!(result.unwrap().is_empty(), "未配置应返回空 vec");
    }

    #[tokio::test]
    async fn test_sniff_ignores_sniff_enabled_flag() {
        // sniff_enabled=false 不再阻止后端 sniff_suggestions 调用
        // 该开关仅由前端在自动触发（非 force）时检查
        // 后端 sniff_suggestions 只检查 is_configured
        let config = AiConfig {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-test-key".to_string(),
            model: "gpt-4o-mini".to_string(),
            sniff_enabled: false,
        };
        // sniff_enabled=false 不会导致静默返回空，会继续调用 AI
        // 但因为没有 mock 服务器，调用会失败，验证的是不会提前返回 Ok([])
        let result = sniff_suggestions("明天上午9点开会", &config).await;
        // 应该是 AI 调用失败（网络错误），而不是静默返回空
        assert!(result.is_err(), "sniff_enabled=false 不应阻止后端调用，AI 请求应因网络失败");
    }

    #[tokio::test]
    async fn test_sniff_returns_empty_when_ai_says_no_time() {
        let mut server = mockito::Server::new_async().await;
        // AI 返回空建议列表（便签正文无时间信息）
        let inner = r#"{"suggestions": []}"#;
        let _m = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(ai_response(inner))
            .create_async()
            .await;

        let config = config_with_base(&server.url());
        let result = sniff_suggestions("只是一段普通文字，没有时间", &config).await;
        assert!(result.is_ok(), "AI 返回空建议列表应返回 Ok(vec![])");
        assert!(result.unwrap().is_empty(), "空建议列表应返回空 vec");
    }

    #[tokio::test]
    async fn test_sniff_returns_suggestion_when_ai_detects_time() {
        let mut server = mockito::Server::new_async().await;
        // AI 返回含一条 reminder 建议（新版 suggestions 格式）
        let inner = r#"{"suggestions": [{"type": "reminder", "time_text": "明天上午9点", "start_time": "2026-07-17 09:00", "title": "开会", "repeat_type": "once", "repeat_day": null}]}"#;
        let _m = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(ai_response(inner))
            .create_async()
            .await;

        let config = config_with_base(&server.url());
        let result = sniff_suggestions("明天上午9点开会", &config).await.unwrap();
        assert_eq!(result.len(), 1, "AI 检测到时间应返回 1 条建议");
        let s = &result[0];
        assert_eq!(s.r#type, "reminder", "建议类型应为 reminder");
        assert_eq!(s.title, "添加提醒", "建议标题应为'添加提醒'");
        assert!(
            s.description.contains("明天上午9点"),
            "描述应包含时间文本，实际: {}",
            s.description
        );
        // data 字段为 SniffResult 序列化
        assert_eq!(s.data["detected"], true, "data.detected 应为 true");
        assert_eq!(s.data["time_text"], "明天上午9点");
        assert_eq!(s.data["start_time"], "2026-07-17 09:00");
        assert_eq!(s.data["title"], "开会");
        assert_eq!(s.data["repeat_type"], "once");
        assert_eq!(s.data["repeat_day"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn test_sniff_returns_suggestion_with_monthly_repeat_day() {
        let mut server = mockito::Server::new_async().await;
        // AI 返回 monthly 重复 + repeat_day=15
        let inner = r#"{"suggestions": [{"type": "reminder", "time_text": "每月15号", "start_time": "2026-08-15 09:00", "title": "月度报表", "repeat_type": "monthly", "repeat_day": 15}]}"#;
        let _m = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(ai_response(inner))
            .create_async()
            .await;

        let config = config_with_base(&server.url());
        let result = sniff_suggestions("每月15号提交月度报表", &config).await.unwrap();
        assert_eq!(result.len(), 1, "应返回 1 条建议");
        let s = &result[0];
        assert_eq!(s.r#type, "reminder");
        assert_eq!(s.data["repeat_type"], "monthly");
        assert_eq!(s.data["repeat_day"], 15);
    }

    // ============ 新增 4 种建议类型测试 ============

    #[tokio::test]
    async fn test_sniff_returns_todo_split_suggestion() {
        let mut server = mockito::Server::new_async().await;
        // AI 返回 todo_split 建议
        let inner = r#"{"suggestions": [{"type": "todo_split", "todos": ["买牛奶", "交水电费", "取快递"]}]}"#;
        let _m = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(ai_response(inner))
            .create_async()
            .await;

        let config = config_with_base(&server.url());
        let result = sniff_suggestions("今天要买牛奶、交水电费、取快递", &config).await.unwrap();
        assert_eq!(result.len(), 1, "应返回 1 条 todo_split 建议");
        let s = &result[0];
        assert_eq!(s.r#type, "todo_split", "建议类型应为 todo_split");
        assert_eq!(s.title, "拆分为待办", "标题应为'拆分为待办'");
        assert!(
            s.description.contains("3"),
            "描述应包含任务数量，实际: {}",
            s.description
        );
        // data 为 todos 数组
        let todos = s.data.as_array().expect("data 应为数组");
        assert_eq!(todos.len(), 3, "应有 3 个待办项");
        assert_eq!(todos[0], "买牛奶");
        assert_eq!(todos[1], "交水电费");
        assert_eq!(todos[2], "取快递");
    }

    #[tokio::test]
    async fn test_sniff_returns_tidy_suggestion() {
        let mut server = mockito::Server::new_async().await;
        // AI 返回 tidy 建议
        let inner = r#"{"suggestions": [{"type": "tidy", "tidy_text": "明天上午九点召开部门会议"}]}"#;
        let _m = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(ai_response(inner))
            .create_async()
            .await;

        let config = config_with_base(&server.url());
        let result = sniff_suggestions("明天上午九点开部门会议哈", &config).await.unwrap();
        assert_eq!(result.len(), 1, "应返回 1 条 tidy 建议");
        let s = &result[0];
        assert_eq!(s.r#type, "tidy", "建议类型应为 tidy");
        assert_eq!(s.title, "规整文本", "标题应为'规整文本'");
        // data 为 tidy_text 字符串
        assert_eq!(s.data, "明天上午九点召开部门会议", "data 应为规整后的文本");
    }

    #[tokio::test]
    async fn test_sniff_returns_style_suggestion() {
        let mut server = mockito::Server::new_async().await;
        // AI 返回 style 建议（formal 正式文风）
        let inner = r#"{"suggestions": [{"type": "style", "style_type": "formal", "styled_text": "请于周一前提交项目报告"}]}"#;
        let _m = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(ai_response(inner))
            .create_async()
            .await;

        let config = config_with_base(&server.url());
        let result = sniff_suggestions("周一前把项目报告交了", &config).await.unwrap();
        assert_eq!(result.len(), 1, "应返回 1 条 style 建议");
        let s = &result[0];
        assert_eq!(s.r#type, "style", "建议类型应为 style");
        assert_eq!(s.title, "更正式", "formal 类型标题应为'更正式'");
        // data 含 style_type 和 styled_text
        assert_eq!(s.data["style_type"], "formal");
        assert_eq!(s.data["styled_text"], "请于周一前提交项目报告");
    }

    #[tokio::test]
    async fn test_sniff_returns_tag_suggest_suggestion() {
        let mut server = mockito::Server::new_async().await;
        // AI 返回 tag_suggest 建议
        let inner = r#"{"suggestions": [{"type": "tag_suggest", "tags": ["工作", "项目", "报告"]}]}"#;
        let _m = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(ai_response(inner))
            .create_async()
            .await;

        let config = config_with_base(&server.url());
        let result = sniff_suggestions("本月项目周报汇总", &config).await.unwrap();
        assert_eq!(result.len(), 1, "应返回 1 条 tag_suggest 建议");
        let s = &result[0];
        assert_eq!(s.r#type, "tag_suggest", "建议类型应为 tag_suggest");
        assert_eq!(s.title, "推荐标签", "标题应为'推荐标签'");
        assert!(
            s.description.contains("工作") && s.description.contains("项目"),
            "描述应包含标签，实际: {}",
            s.description
        );
        // data 为 tags 数组
        let tags = s.data.as_array().expect("data 应为数组");
        assert_eq!(tags.len(), 3, "应推荐 3 个标签");
        assert_eq!(tags[0], "工作");
        assert_eq!(tags[1], "项目");
        assert_eq!(tags[2], "报告");
    }

    #[tokio::test]
    async fn test_sniff_returns_multiple_suggestions() {
        let mut server = mockito::Server::new_async().await;
        // AI 同时返回 reminder + todo_split + tag_suggest 三条建议
        let inner = r#"{"suggestions": [{"type": "reminder", "time_text": "明天上午9点", "start_time": "2026-07-17 09:00", "title": "开会", "repeat_type": "once", "repeat_day": null}, {"type": "todo_split", "todos": ["准备议程", "通知参会人员"]}, {"type": "tag_suggest", "tags": ["会议", "工作"]}]}"#;
        let _m = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(ai_response(inner))
            .create_async()
            .await;

        let config = config_with_base(&server.url());
        let result = sniff_suggestions("明天上午9点开会，要准备议程、通知参会人员", &config).await.unwrap();
        assert_eq!(result.len(), 3, "应返回 3 条建议");
        // 验证三条建议的类型（按 AI 返回顺序）
        assert_eq!(result[0].r#type, "reminder", "第 1 条应为 reminder");
        assert_eq!(result[1].r#type, "todo_split", "第 2 条应为 todo_split");
        assert_eq!(result[2].r#type, "tag_suggest", "第 3 条应为 tag_suggest");
        // 验证 reminder 数据
        assert_eq!(result[0].data["start_time"], "2026-07-17 09:00");
        // 验证 todo_split 数据
        let todos = result[1].data.as_array().expect("todo_split data 应为数组");
        assert_eq!(todos.len(), 2);
        // 验证 tag_suggest 数据
        let tags = result[2].data.as_array().expect("tag_suggest data 应为数组");
        assert_eq!(tags.len(), 2);
    }

    #[tokio::test]
    async fn test_sniff_skips_unknown_type() {
        let mut server = mockito::Server::new_async().await;
        // AI 返回未知类型 + 一个合法 reminder，未知类型应被跳过
        let inner = r#"{"suggestions": [{"type": "unknown_future_type", "foo": "bar"}, {"type": "reminder", "time_text": "明天上午9点", "start_time": "2026-07-17 09:00", "title": "开会", "repeat_type": "once", "repeat_day": null}]}"#;
        let _m = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(ai_response(inner))
            .create_async()
            .await;

        let config = config_with_base(&server.url());
        let result = sniff_suggestions("明天上午9点开会", &config).await.unwrap();
        // 未知类型应被跳过，只保留 1 条 reminder
        assert_eq!(result.len(), 1, "未知类型应被跳过，只保留 1 条 reminder");
        assert_eq!(result[0].r#type, "reminder", "应只保留 reminder 建议");
    }
}
