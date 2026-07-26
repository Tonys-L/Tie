//! 从 AI 返回文本中提取 JSON 片段的通用工具。
//!
//! 职责：
//! - `extract_object`：提取第一个完整 JSON 对象片段（`{` 开头）
//! - `extract_array`：提取第一个完整 JSON 数组片段（`[` 开头）
//!
//! 调用方：
//! - `commands/ai_commands.rs`：`ai_sort_todos` 解析 AI 返回的待办数组
//! - `reminder_parser.rs`：`parse_reminder_json` / `sniff_suggestions` 解析 AI 返回的对象
//!
//! 设计要点：
//! - 用 `serde_json::Deserializer` 流式解析第一个完整 JSON 值，通过 `byte_offset()` 精确取得边界
//! - 相比手写 `find('{') + rfind('}')`，可正确处理 JSON 字符串值内含 `{`/`}`/`[`/`]` 字符的情况
//! - 兼容 AI 返回 markdown 代码块或附带解释文字的场景

use serde::de::IgnoredAny;
use serde_json::Deserializer;

/// 从文本中提取第一个完整 JSON 对象片段（`{` 开头）。
///
/// 找不到 `{` 或解析失败时返回 `None`。
///
/// 用 `Deserializer::into_iter::<IgnoredAny>` 流式解析，通过 `byte_offset()` 精确取得边界。
/// 可正确处理 JSON 字符串值内含 `{`/`}` 字符的情况（如 `content` 字段值含大括号）。
pub fn extract_object(text: &str) -> Option<&str> {
    extract_first(text, '{')
}

/// 从文本中提取第一个完整 JSON 数组片段（`[` 开头）。
///
/// 找不到 `[` 或解析失败时返回 `None`。
///
/// 用 `Deserializer::into_iter::<IgnoredAny>` 流式解析，通过 `byte_offset()` 精确取得边界。
/// 可正确处理 JSON 字符串值内含 `[`/`]` 字符的情况（如待办条目含方括号）。
pub fn extract_array(text: &str) -> Option<&str> {
    extract_first(text, '[')
}

/// 提取以 `start_char` 开始的第一个完整 JSON 值片段。
///
/// 找不到 `start_char` 或解析失败时返回 `None`。
fn extract_first(text: &str, start_char: char) -> Option<&str> {
    let start = text.find(start_char)?;
    let slice = &text[start..];
    let mut stream = Deserializer::from_str(slice).into_iter::<IgnoredAny>();
    match stream.next() {
        Some(Ok(_)) => {
            let end = start + stream.byte_offset();
            Some(&text[start..end])
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============ extract_array 测试（迁移自 ai_commands） ============

    #[test]
    fn test_extract_array_pure_array() {
        let text = r#"["a","b","c"]"#;
        let extracted = extract_array(text).unwrap();
        assert_eq!(extracted, r#"["a","b","c"]"#);
    }

    #[test]
    fn test_extract_array_with_surrounding_text() {
        let text = r#"排序结果：["开会","写文档"] 完成"#;
        let extracted = extract_array(text).unwrap();
        assert_eq!(extracted, r#"["开会","写文档"]"#);
    }

    /// 待办条目内含 `[`/`]` 字符时，旧 `rfind(']')` 会取字符串值内的 `]` 误当作数组结束。
    #[test]
    fn test_extract_array_with_bracket_in_value() {
        let text = r#"结果 ["任务 [重要]","普通"]"#;
        let extracted = extract_array(text).unwrap();
        assert_eq!(extracted, r#"["任务 [重要]","普通"]"#);
    }

    #[test]
    fn test_extract_array_no_bracket_returns_none() {
        let text = "没有数组的文本";
        assert!(extract_array(text).is_none());
    }

    // ============ extract_object 测试（迁移自 reminder_parser） ============

    #[test]
    fn test_extract_object_strips_markdown_code_block() {
        let content = "```json\n{\"title\":\"x\"}\n```";
        let extracted = extract_object(content).unwrap();
        assert_eq!(extracted, "{\"title\":\"x\"}");
    }

    /// JSON 字符串值内含 `}` 字符时，旧 `rfind('}')` 会把字符串值内的 `}` 误当作对象结束，
    /// 导致截断范围错误。新实现用 `Deserializer::byte_offset` 精确取边界。
    #[test]
    fn test_extract_object_with_brace_in_string_value() {
        // content 字段值含 `}` 字符（如用户写了一段 JSON 代码在便签里被 AI 解析）
        let content = r#"前缀文字 {"title":"x","content":"}"} 后缀"#;
        let extracted = extract_object(content).unwrap();
        assert_eq!(extracted, r#"{"title":"x","content":"}"}"#);
    }

    /// 嵌套 JSON 对象：旧 `rfind('}')` 取最外层 `}` 是对的，但若字符串值内含 `}` 仍会出错。
    /// 新实现通过流式解析正确识别嵌套边界。
    #[test]
    fn test_extract_object_nested_object() {
        let content = r#"AI 返回：{"title":"x","meta":{"nested":true}} 完成"#;
        let extracted = extract_object(content).unwrap();
        assert_eq!(extracted, r#"{"title":"x","meta":{"nested":true}}"#);
    }

    #[test]
    fn test_extract_object_no_brace_returns_none() {
        let content = "没有对象的文本";
        assert!(extract_object(content).is_none());
    }
}
