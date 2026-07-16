use chrono::Local;

use super::ChatMessage;

/// 嗅探系统提示模板
///
/// 要求 AI 分析便签正文并返回 JSON 建议列表。
/// 支持 5 种建议类型：reminder / todo_split / tidy / style / tag_suggest。
/// `{current_time}` 在构造消息时替换为本地当前时间，便于 AI 解析相对时间（如"明天"）。
/// AI 返回的每个建议项已包含 type 字段，后端按 type 包装为 Suggestion 结构。
const SNIFF_SYSTEM_PROMPT_TEMPLATE: &str = "\
你是便签应用的 AI 助手。分析以下便签正文，返回 JSON 建议列表。根据内容特点，可能返回以下类型的建议（可多条）：

1. reminder：当便签包含时间意图时必须返回。包括但不限于：
   - 明确提醒词：\"通知我\"\"提醒我\"\"叫我\"\"别忘了\"
   - 时间表达：\"五分钟后\"\"明天上午9点\"\"下周三\"\"每天早上8点\"\"腊月廿三\"
   - 示例输入\"五分钟后通知我\" → start_time 为当前时间+5分钟
   data: {\"type\":\"reminder\", \"time_text\":\"原文时间描述\", \"start_time\":\"YYYY-MM-DD HH:mm\", \"title\":\"提醒标题\", \"repeat_type\":\"once/daily/weekly/monthly/lunar_monthly\", \"repeat_day\":数字或null}

2. todo_split：检测到可拆分为待办清单的内容时返回。包括：
   - 顿号/逗号分隔的并列项：\"买牛奶、交水电费、回邮件\"
   - 换行分隔的多个动作：\"明天测试小程序，开发U盘播放\"
   - 包含多个\"动词+名词\"组合的短句：\"写报告、开会、review代码\"
   data: {\"todos\":[\"任务1\",\"任务2\",\"任务3\"]}

3. tidy：检测到口语化、冗余、不工整的文本时返回
   data: {\"tidy_text\":\"规整后的文本\"}

4. style：检测到可改善文风的内容时返回（仅在文本较正式场景才建议）
   data: {\"style_type\":\"formal\", \"styled_text\":\"更正式的文本\"}

5. tag_suggest：基于便签内容推荐最多3个标签
   data: {\"tags\":[\"标签1\",\"标签2\",\"标签3\"]}

重要规则：
- 只要便签包含\"通知我\"\"提醒我\"\"叫我\"等词，必须返回 reminder 建议
- 相对时间（\"五分钟后\"\"半小时后\"）根据当前时间计算绝对时间
- 便签包含多个动作或并列项时，必须返回 todo_split 建议
- 短文本（<5字）可能只有一条建议，正常
- 不要过度建议：文本已工整则不返回 tidy
- data 字段中不要包含 type，type 只在顶层

返回格式：{\"suggestions\": [建议1, 建议2, ...]}
如果便签内容确实无任何可优化项，返回：{\"suggestions\": []}
仅返回 JSON，不附加任何解释。
当前时间：{current_time}";

/// 构造嗅探消息列表（system + user）
///
/// system 提示注入当前本地时间，user 消息为便签正文。
pub fn build_sniff_messages(content: &str) -> Vec<ChatMessage> {
    let current_time = Local::now().format("%Y-%m-%d %H:%M").to_string();
    let system_prompt = SNIFF_SYSTEM_PROMPT_TEMPLATE.replace("{current_time}", &current_time);
    vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(content),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_sniff_messages_returns_system_and_user() {
        let messages = build_sniff_messages("明天上午9点开会");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[1].content, "明天上午9点开会");
    }

    #[test]
    fn test_system_prompt_contains_required_fields() {
        let messages = build_sniff_messages("test");
        let system_content = &messages[0].content;

        // 建议列表结构
        assert!(system_content.contains("suggestions"), "系统提示应包含 suggestions 字段说明");
        assert!(system_content.contains("type"), "系统提示应包含 type 字段说明");

        // 5 种建议类型说明
        assert!(system_content.contains("reminder"), "系统提示应包含 reminder 类型说明");
        assert!(system_content.contains("todo_split"), "系统提示应包含 todo_split 类型说明");
        assert!(system_content.contains("tidy"), "系统提示应包含 tidy 类型说明");
        assert!(system_content.contains("style"), "系统提示应包含 style 类型说明");
        assert!(system_content.contains("tag_suggest"), "系统提示应包含 tag_suggest 类型说明");

        // reminder 类型 JSON 字段说明
        assert!(system_content.contains("time_text"), "系统提示应包含 time_text 字段说明");
        assert!(system_content.contains("start_time"), "系统提示应包含 start_time 字段说明");
        assert!(system_content.contains("title"), "系统提示应包含 title 字段说明");
        assert!(system_content.contains("repeat_type"), "系统提示应包含 repeat_type 字段说明");
        assert!(system_content.contains("repeat_day"), "系统提示应包含 repeat_day 字段说明");

        // todo_split / tidy / style / tag_suggest 字段说明
        assert!(system_content.contains("todos"), "系统提示应包含 todos 字段说明");
        assert!(system_content.contains("tidy_text"), "系统提示应包含 tidy_text 字段说明");
        assert!(system_content.contains("style_type"), "系统提示应包含 style_type 字段说明");
        assert!(system_content.contains("styled_text"), "系统提示应包含 styled_text 字段说明");
        assert!(system_content.contains("tags"), "系统提示应包含 tags 字段说明");

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
    fn test_system_prompt_injects_current_time() {
        let messages = build_sniff_messages("test");
        let system_content = &messages[0].content;

        // 当前时间占位符应被替换为实际时间（YYYY-MM-DD HH:mm 格式）
        assert!(!system_content.contains("{current_time}"), "占位符应已被替换");
        assert!(
            system_content.contains("当前时间："),
            "系统提示应包含当前时间前缀"
        );

        // 验证时间格式：当前时间：YYYY-MM-DD HH:mm
        let time_prefix = "当前时间：";
        let idx = system_content
            .find(time_prefix)
            .expect("应包含当前时间前缀");
        let time_str = &system_content[idx + time_prefix.len()..];
        // 至少匹配 4 位年份开头
        let year: String = time_str.chars().take(4).collect();
        year.parse::<u32>()
            .expect("当前时间应以 4 位年份开头");
    }

    #[test]
    fn test_build_sniff_messages_preserves_user_input() {
        let inputs = [
            "明天上午9点开会",
            "下周三下午2点项目评审",
            "每天早上8点叫我起床",
            "",
        ];
        for input in inputs {
            let messages = build_sniff_messages(input);
            assert_eq!(messages[1].content, input);
        }
    }
}
