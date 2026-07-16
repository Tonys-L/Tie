use super::ChatMessage;

/// 提醒解析系统提示
///
/// 要求 AI 将自然语言解析为 JSON，字段：title / start_time / repeat_type / repeat_day。
/// 仅返回 JSON，不附加解释。
const REMINDER_SYSTEM_PROMPT: &str = "\
你是一个提醒解析助手。请将用户输入的自然语言解析为 JSON 格式的提醒信息。

仅返回 JSON，不要附加任何解释或 markdown 代码块。JSON 字段如下：

- title: 提醒标题（字符串）
- start_time: 开始时间，格式 YYYY-MM-DD HH:mm（24 小时制）
- repeat_type: 重复类型，取值之一：once（一次性）、daily（每天）、weekly（每周）、monthly（每月）、lunar_monthly（农历每月）
- repeat_day: 重复日期（数字 1-31），仅 monthly 和 lunar_monthly 类型需要；其他类型为 null

规则：
1. repeat_type 只能是 once/daily/weekly/monthly/lunar_monthly 之一
2. repeat_day 仅在 repeat_type 为 monthly 或 lunar_monthly 时提供数字，其他类型必须为 null
3. 仅返回 JSON，不要有任何额外文字或代码块标记";

/// 构建提醒解析的消息列表（system + user）
pub fn build_reminder_messages(user_input: &str) -> Vec<ChatMessage> {
    vec![
        ChatMessage::system(REMINDER_SYSTEM_PROMPT),
        ChatMessage::user(user_input),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_reminder_messages_returns_system_and_user() {
        let messages = build_reminder_messages("明天下午 3 点提醒我开会");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[1].content, "明天下午 3 点提醒我开会");
    }

    #[test]
    fn test_system_prompt_contains_required_fields() {
        let messages = build_reminder_messages("test");
        let system_content = &messages[0].content;

        assert!(system_content.contains("title"), "系统提示应包含 title 字段说明");
        assert!(system_content.contains("start_time"), "系统提示应包含 start_time 字段说明");
        assert!(system_content.contains("repeat_type"), "系统提示应包含 repeat_type 字段说明");
        assert!(system_content.contains("repeat_day"), "系统提示应包含 repeat_day 字段说明");

        // 重复类型取值
        assert!(system_content.contains("once"));
        assert!(system_content.contains("daily"));
        assert!(system_content.contains("weekly"));
        assert!(system_content.contains("monthly"));
        assert!(system_content.contains("lunar_monthly"));

        // 时间格式
        assert!(system_content.contains("YYYY-MM-DD HH:mm"));

        // 要求仅返回 JSON
        assert!(system_content.contains("JSON"));
    }

    #[test]
    fn test_build_reminder_messages_preserves_user_input() {
        let inputs = [
            "每天早上 8 点叫我起床",
            "每周一上午 9 点开周会",
            "2026 年 1 月 1 日 00:00 新年快乐",
            "",
        ];
        for input in inputs {
            let messages = build_reminder_messages(input);
            assert_eq!(messages[1].content, input);
        }
    }
}
