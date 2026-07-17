use super::ChatMessage;

/// 构造待办排序消息列表（system + user）
///
/// system 提示要求 AI 根据紧急程度对待办事项排序，返回 JSON 数组。
/// user 消息为待办清单（每行一条）。
pub fn build_sort_messages(todos: &[String]) -> Vec<ChatMessage> {
    let system_prompt = "你是待办事项排序助手。根据紧急程度对以下待办事项重新排序，紧急的排前面。
排序权重（从高到低）：
1. 含\"紧急\"\"deadline\"\"立刻\"\"马上\"\"必须\"等词
2. 含\"今天\"\"明天\"\"今晚\"等近期时间
3. 含\"本周\"\"下周\"\"这周\"等中期时间
4. 含\"下个月\"\"以后\"\"有空\"等远期时间
5. 无明确时间的一般事项

返回格式：JSON 数组，如 [\"任务1\",\"任务2\",\"任务3\"]
仅返回 JSON，不附加任何解释。保持每项的原文不变，只调整顺序。";

    let user_content = todos
        .iter()
        .map(|t| format!("- {}", t))
        .collect::<Vec<_>>()
        .join("\n");

    vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(user_content),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_sort_messages_returns_system_and_user() {
        let todos = vec![
            "买牛奶".to_string(),
            "紧急：完成季度报告".to_string(),
            "明天交水电费".to_string(),
        ];
        let messages = build_sort_messages(&todos);

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
    }

    #[test]
    fn test_system_prompt_contains_sorting_rules() {
        let messages = build_sort_messages(&["任务".to_string()]);
        let system_content = &messages[0].content;

        assert!(system_content.contains("紧急程度"), "应包含排序依据");
        assert!(system_content.contains("JSON"), "应要求返回 JSON");
        assert!(system_content.contains("仅返回"), "应要求仅返回结果");
    }

    #[test]
    fn test_user_content_formats_todos_with_dash() {
        let todos = vec!["任务A".to_string(), "任务B".to_string()];
        let messages = build_sort_messages(&todos);
        let user_content = &messages[1].content;

        assert!(user_content.contains("- 任务A"), "每项应以 - 开头");
        assert!(user_content.contains("- 任务B"));
    }

    #[test]
    fn test_empty_todos_returns_valid_messages() {
        let messages = build_sort_messages(&[]);
        assert_eq!(messages.len(), 2);
        // 空列表时 user 内容为空字符串
        assert_eq!(messages[1].content, "");
    }
}
