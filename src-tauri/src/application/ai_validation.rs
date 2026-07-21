//! AI 命令业务校验（命令层业务规则下沉）
//!
//! 职责：
//! - 承载 AI 命令的输入校验规则（文本长度、待办条目数等）
//! - 与命令层解耦，可在不启动 Tauri 的情况下单测
//!
//! 调用方：
//! - `commands/ai_commands.rs`：薄壳调用本模块校验后再调用 AI service
//!
//! 依赖：无（纯函数校验）

/// AI 文本重写长度限制
const REWRITE_TEXT_MIN_LEN: usize = 5;
const REWRITE_TEXT_MAX_LEN: usize = 500;

/// AI 待办排序触发阈值（条目数 ≤ 此值时拒绝）
const SORT_TODOS_MIN_COUNT: usize = 3;

/// 校验 AI 文本重写输入
///
/// 文本长度限制 5~500 字符（按字符计数，避免 UTF-8 切片 panic）。
/// 错误时返回中文提示供前端展示。
pub fn validate_rewrite_text(text: &str) -> Result<(), String> {
    let char_count = text.chars().count();
    if char_count < REWRITE_TEXT_MIN_LEN || char_count > REWRITE_TEXT_MAX_LEN {
        return Err(format!(
            "请选中文本长度在 {}~{} 字符之间",
            REWRITE_TEXT_MIN_LEN, REWRITE_TEXT_MAX_LEN
        ));
    }
    Ok(())
}

/// 校验 AI 待办排序输入
///
/// 待办条目数 ≤ 3 时拒绝排序（无必要调用 AI）。
pub fn validate_sort_todos(todos: &[String]) -> Result<(), String> {
    if todos.len() <= SORT_TODOS_MIN_COUNT {
        return Err(format!(
            "待办条目数 ≤ {}，无需 AI 排序",
            SORT_TODOS_MIN_COUNT
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- validate_rewrite_text 测试 ----

    #[test]
    fn test_validate_rewrite_text_valid() {
        assert!(validate_rewrite_text("这是一段足够长的文本").is_ok());
    }

    #[test]
    fn test_validate_rewrite_text_too_short() {
        let result = validate_rewrite_text("短");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("5"));
    }

    #[test]
    fn test_validate_rewrite_text_too_long() {
        let text: String = "a".repeat(501);
        let result = validate_rewrite_text(&text);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("500"));
    }

    #[test]
    fn test_validate_rewrite_text_boundary_min() {
        // 恰好 5 字符应通过
        assert!(validate_rewrite_text("abcde").is_ok());
    }

    #[test]
    fn test_validate_rewrite_text_boundary_max() {
        // 恰好 500 字符应通过
        let text: String = "a".repeat(500);
        assert!(validate_rewrite_text(&text).is_ok());
    }

    #[test]
    fn test_validate_rewrite_text_utf8_count() {
        // 中文字符按字符计数而非字节，5 个中文字符应通过
        assert!(validate_rewrite_text("一二三四五").is_ok());
    }

    // ---- validate_sort_todos 测试 ----

    #[test]
    fn test_validate_sort_todos_valid() {
        let todos = vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()];
        assert!(validate_sort_todos(&todos).is_ok());
    }

    #[test]
    fn test_validate_sort_todos_too_few() {
        let todos = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let result = validate_sort_todos(&todos);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("≤ 3"));
    }

    #[test]
    fn test_validate_sort_todos_empty() {
        let todos: Vec<String> = vec![];
        let result = validate_sort_todos(&todos);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_sort_todos_boundary() {
        // 恰好 4 条应通过（> 3）
        let todos = vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()];
        assert!(validate_sort_todos(&todos).is_ok());
    }
}
