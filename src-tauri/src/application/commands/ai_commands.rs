//! AI 命令：配置管理、连接测试、自然语言解析、嗅探建议、报告生成、文本改写、待办排序。

use tauri::{AppHandle, Emitter, State};

use crate::AppState;
use super::super::ai_config::AiConfig;
use super::super::ai_service::{AiService, ChatMessage};

/// 统一 AI 调用链：加载配置 → 校验已配置 → 构造 AiService → 调用 call → 转字符串错误。
///
/// `test_ai_connection` / `ai_rewrite_text` / `ai_sort_todos` 三个命令共用此封装，
/// 避免每处内联 `load_default + is_configured + AiService::new + map_err` 链。
/// 各命令的解析逻辑（trim / JSON 数组提取）保留在调用方，因为解析语义属于命令的接口契约。
async fn ai_call_raw(messages: Vec<ChatMessage>) -> Result<String, String> {
    let config = AiConfig::load_default()?;
    if !config.is_configured() {
        return Err("AI 未配置".to_string());
    }
    let service = AiService::new(config);
    service.call(messages).await.map_err(|e| e.to_string())
}

// ============ AI 配置命令 ============

/// 获取 AI 配置（未配置时返回空值，前端用密码框显示 API Key）
#[tauri::command]
pub async fn get_ai_config() -> Result<AiConfig, String> {
    AiConfig::load_default()
}

/// 保存 AI 配置到本地用户目录（不随 Git 同步）
#[tauri::command]
pub async fn save_ai_config(app: AppHandle, base_url: String, api_key: String, model: String, sniff_enabled: bool) -> Result<(), String> {
    let path = AiConfig::default_path();
    let config = AiConfig {
        base_url,
        api_key,
        model,
        sniff_enabled,
    };
    config.save(&path)?;
    let _ = app.emit("ai-config-changed", ());
    Ok(())
}

/// 测试 AI 连接是否可用（发送 ping 请求）
#[tauri::command]
pub async fn test_ai_connection() -> Result<String, String> {
    ai_call_raw(vec![ChatMessage::user("ping")]).await
}

// ============ AI 业务命令 ============

/// 自然语言解析提醒（返回 ReminderDraft 供前端预填表单）
#[tauri::command]
pub async fn parse_reminder_natural(text: String) -> Result<super::super::reminder_parser::ReminderDraft, String> {
    let config = AiConfig::load_default()?;
    super::super::reminder_parser::parse_reminder_natural(&text, &config)
        .await
        .map_err(|e| e.to_string())
}

/// 嗅探便签正文，返回通用建议列表
///
/// 当前只识别 reminder 类型建议（检测到时间信息时返回"添加提醒"建议）。
/// 返回空 vec 表示无建议或未配置 AI/关闭嗅探（静默跳过）。
/// 架构支持未来扩展 todo_split / tidy 等类型。
#[tauri::command]
pub async fn sniff_suggestions(content: String) -> Result<Vec<super::super::reminder_parser::Suggestion>, String> {
    let config = AiConfig::load_default()?;
    super::super::reminder_parser::sniff_suggestions(&content, &config)
        .await
        .map_err(|e| e.to_string())
}

/// 生成周报/月报草稿
///
/// 基于便签列表调用 AI 生成 Markdown 报告。
/// - `period_type`：`"weekly"` 或 `"monthly"`
/// - `start_date` / `end_date`：ISO 格式 `YYYY-MM-DD`，用于过滤便签范围
///
/// 未配置 AI 时返回 `"AI 未配置"` 错误。
#[tauri::command]
pub async fn generate_report(
    state: State<'_, AppState>,
    period_type: String,
    start_date: String,
    end_date: String,
) -> Result<super::super::report_generator::ReportDraft, String> {
    let config = AiConfig::load_default()?;
    if !config.is_configured() {
        return Err("AI 未配置".to_string());
    }
    // period_type 解析下沉到 report_generator::parse_period
    let period = super::super::report_generator::parse_period(&period_type, &start_date, &end_date)?;
    let notes = state.note_repo.find_all()?;
    // 按 updated_at 日期部分过滤在 [start_date, end_date] 范围内（业务规则下沉到 report_generator）
    let filtered = super::super::report_generator::filter_notes_by_date(&notes, &start_date, &end_date);
    super::super::report_generator::generate_report(&filtered, period, &config)
        .await
        .map_err(|e| e.to_string())
}

/// AI 文本重写
///
/// 用户在便签中选中文本后右键调用，根据 `operation` 指定的风格重写文本。
/// - `operation`：`tidy` / `todo_split` / `style_formal` / `style_concise` / `style_mild`
/// - 文本长度限制 5~500 字符（按字符计数，避免 UTF-8 切片 panic）
///
/// 未配置 AI 时返回 `"AI 未配置"` 错误。
#[tauri::command]
pub async fn ai_rewrite_text(
    _state: State<'_, AppState>,
    text: String,
    operation: String,
) -> Result<String, String> {
    let op = super::super::prompts::rewrite::RewriteOperation::from_str(&operation)
        .ok_or_else(|| "无效的操作类型".to_string())?;
    // 文本长度校验下沉到 ai_validation::validate_rewrite_text
    super::super::ai_validation::validate_rewrite_text(&text)?;
    let messages = super::super::prompts::rewrite::build_rewrite_messages(&text, op);
    let result = ai_call_raw(messages).await?;
    Ok(result.trim().to_string())
}

/// AI 待办清单智能排序
///
/// 接收待办条目列表，调用 AI 按紧急程度排序后返回。
/// 条目数 ≤ 3 时拒绝排序（无必要）。
#[tauri::command]
pub async fn ai_sort_todos(
    _state: State<'_, AppState>,
    todos: Vec<String>,
) -> Result<Vec<String>, String> {
    // 待办条目数校验下沉到 ai_validation::validate_sort_todos
    super::super::ai_validation::validate_sort_todos(&todos)?;
    let messages = super::super::prompts::sort::build_sort_messages(&todos);
    let result = ai_call_raw(messages).await?;

    // 解析 JSON 数组
    let trimmed = result.trim();
    // 尝试提取 JSON 数组（兼容 AI 可能附加的额外文本）
    let json_str = extract_json_array(trimmed).ok_or_else(|| "排序结果解析失败".to_string())?;
    let arr: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
    let arr = arr
        .as_array()
        .ok_or_else(|| "排序结果不是数组".to_string())?;
    let sorted: Vec<String> = arr
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    if sorted.is_empty() {
        return Err("排序结果为空".to_string());
    }
    Ok(sorted)
}

/// 从 AI 返回文本中提取 JSON 数组片段
fn extract_json_array(text: &str) -> Option<String> {
    let start = text.find('[')?;
    let end = text.rfind(']')?;
    if end >= start {
        Some(text[start..=end].to_string())
    } else {
        None
    }
}
