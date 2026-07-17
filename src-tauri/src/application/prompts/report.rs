use super::ChatMessage;

/// 周报/月报生成系统提示模板
///
/// 要求 AI 根据便签列表摘要生成周报/月报草稿，按四个板块输出 Markdown。
/// `{period}` 在构造消息时替换为周期描述（如 "2026-07-13 ~ 07-19" 或 "2026-07"）。
/// `{notes_summary}` 在构造消息时替换为便签列表摘要。
const REPORT_SYSTEM_PROMPT_TEMPLATE: &str = "\
你是便签应用的 AI 助手。根据以下便签列表生成周报/月报草稿，按四个板块输出 Markdown：

## 📌 重点
## ✅ 已完成
## ⏳ 进行中
## 💡 零散记录

规则：
- 根据便签内容判断每条属于哪个板块，没有内容的板块可留空或省略
- 不要添加额外解释，仅输出 Markdown 内容
- 保留便签中的关键信息，可适当提炼与整合
- 报告周期：{period}

便签列表摘要：
{notes_summary}";

/// 构造周报/月报生成的消息列表（system + user）
///
/// system 提示注入周期描述与便签列表摘要。
/// user 消息留空提示（全部上下文已注入 system），保持与已有 prompt 模板一致的 system+user 双消息结构。
pub fn build_report_messages(period: &str, notes_summary: &str) -> Vec<ChatMessage> {
    let system_prompt = REPORT_SYSTEM_PROMPT_TEMPLATE
        .replace("{period}", period)
        .replace("{notes_summary}", notes_summary);
    vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user("请根据以上便签摘要生成本期报告。"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_report_messages_returns_system_and_user() {
        let messages = build_report_messages("2026-07-13 ~ 07-19", "[2026-07-15] 周报: 本周完成功能开发");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
    }

    #[test]
    fn test_build_report_messages_preserves_period_and_notes() {
        let period = "2026-07";
        let notes_summary = "[2026-07-01] 月度规划: 完成 v1.0 上线";
        let messages = build_report_messages(period, notes_summary);
        let system_content = &messages[0].content;

        // 周期描述应被注入
        assert!(
            system_content.contains(period),
            "系统提示应包含周期描述，实际: {}",
            system_content
        );
        // 占位符应已被替换
        assert!(
            !system_content.contains("{period}"),
            "period 占位符应已被替换"
        );
        assert!(
            !system_content.contains("{notes_summary}"),
            "notes_summary 占位符应已被替换"
        );
        // 便签摘要应被注入
        assert!(
            system_content.contains(notes_summary),
            "系统提示应包含便签摘要，实际: {}",
            system_content
        );

        // 四个板块标题应存在
        assert!(system_content.contains("## 📌 重点"), "应包含'重点'板块");
        assert!(system_content.contains("## ✅ 已完成"), "应包含'已完成'板块");
        assert!(system_content.contains("## ⏳ 进行中"), "应包含'进行中'板块");
        assert!(system_content.contains("## 💡 零散记录"), "应包含'零散记录'板块");
    }
}
