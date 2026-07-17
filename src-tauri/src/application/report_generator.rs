use serde::{Deserialize, Serialize};

use super::ai_config::AiConfig;
use super::ai_service::{AiError, AiService};
use super::prompts::report::build_report_messages;
use crate::domain::Note;

/// 报告周期类型
///
/// 用于 `generate_report` 区分周报/月报，并决定标题与 period 占位符格式。
#[derive(Debug, Clone, PartialEq)]
pub enum ReportPeriod {
    /// 周报：start/end 为 ISO 格式 "YYYY-MM-DD"
    Weekly { start: String, end: String },
    /// 月报：year/month 标识月份
    Monthly { year: u32, month: u32 },
}

/// 报告草稿（前端用于预览/编辑）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportDraft {
    pub title: String,
    pub content: String,
}

/// 便签摘要条数上限
const MAX_NOTES_FOR_REPORT: usize = 20;
/// 单条便签内容摘要字符数上限
const NOTE_CONTENT_PREVIEW_LEN: usize = 200;

/// 生成周报/月报草稿
///
/// 流程：build_notes_summary → build_report_messages → AiService::call → ReportDraft
///
/// 数据拾取规则：
/// - 按 updated_at 倒序
/// - 上限 20 条
/// - 每条取 content 前 200 字符
/// - 格式化为 `[YYYY-MM-DD] 标题: 内容摘要`
///
/// 未配置 AI（api_key 为空）时返回 `AiError::NotConfigured`。
pub async fn generate_report(
    notes: &[Note],
    period_type: ReportPeriod,
    config: &AiConfig,
) -> Result<ReportDraft, AiError> {
    if !config.is_configured() {
        return Err(AiError::NotConfigured);
    }
    let notes_summary = build_notes_summary(notes);
    let period_str = build_period_str(&period_type);
    let messages = build_report_messages(&period_str, &notes_summary);
    let service = AiService::new(config.clone());
    let content = service.call(messages).await?;
    Ok(ReportDraft {
        title: build_report_title(&period_type),
        content,
    })
}

/// 构造便签列表摘要
///
/// 按 updated_at 倒序，上限 20 条，每条取 content 前 200 字符。
/// 格式：`[YYYY-MM-DD] 标题: 内容摘要`，每条一行。
fn build_notes_summary(notes: &[Note]) -> String {
    let mut sorted: Vec<&Note> = notes.iter().collect();
    sorted.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    sorted
        .into_iter()
        .take(MAX_NOTES_FOR_REPORT)
        .map(|note| {
            let date: String = note.updated_at.chars().take(10).collect();
            let content_preview: String = note.content.chars().take(NOTE_CONTENT_PREVIEW_LEN).collect();
            format!("[{}] {}: {}", date, note.title, content_preview)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 构造周期描述字符串（注入 prompt 的 `{period}` 占位符）
///
/// - Weekly：`2026-07-13 ~ 07-19`
/// - Monthly：`2026-07`
fn build_period_str(period: &ReportPeriod) -> String {
    match period {
        ReportPeriod::Weekly { start, end } => {
            let end_short: String = end.chars().skip(5).collect();
            format!("{} ~ {}", start, end_short)
        }
        ReportPeriod::Monthly { year, month } => format!("{:04}-{:02}", year, month),
    }
}

/// 构造报告标题
///
/// - Weekly：`2026-07-13 ~ 07-19 周报`
/// - Monthly：`2026-07 月报`
fn build_report_title(period: &ReportPeriod) -> String {
    match period {
        ReportPeriod::Weekly { start, end } => {
            let end_short: String = end.chars().skip(5).collect();
            format!("{} ~ {} 周报", start, end_short)
        }
        ReportPeriod::Monthly { year, month } => format!("{:04}-{:02} 月报", year, month),
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

    fn ai_response(content: &str) -> String {
        format!(
            r#"{{"choices":[{{"message":{{"role":"assistant","content":{}}}}}]}}"#,
            serde_json::Value::String(content.to_string())
        )
    }

    fn make_note(title: &str, content: &str, updated_at: &str) -> Note {
        let mut note = Note::new(title.to_string(), "amber".to_string());
        note.content = content.to_string();
        note.updated_at = updated_at.to_string();
        note
    }

    #[tokio::test]
    async fn test_generate_report_returns_draft_on_success() {
        let mut server = mockito::Server::new_async().await;
        let markdown = "## 📌 重点\n- 完成功能 A\n## ✅ 已完成\n- 修复 Bug B";
        let _m = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(ai_response(markdown))
            .create_async()
            .await;

        let config = config_with_base(&server.url());
        let notes = vec![
            make_note("开发功能", "本周完成功能 A 的开发", "2026-07-15T10:00:00+00:00"),
            make_note("修复 Bug", "修复了 Bug B", "2026-07-16T11:00:00+00:00"),
        ];
        let period = ReportPeriod::Weekly {
            start: "2026-07-13".to_string(),
            end: "2026-07-19".to_string(),
        };

        let draft = generate_report(&notes, period, &config).await.unwrap();
        assert_eq!(draft.title, "2026-07-13 ~ 07-19 周报");
        assert_eq!(draft.content, markdown);
    }

    #[tokio::test]
    async fn test_generate_report_returns_error_when_not_configured() {
        let config = AiConfig::default();
        let notes = vec![make_note("测试", "内容", "2026-07-15T10:00:00+00:00")];
        let period = ReportPeriod::Monthly { year: 2026, month: 7 };

        let result = generate_report(&notes, period, &config).await;
        match result {
            Err(AiError::NotConfigured) => {}
            other => panic!("期望 NotConfigured，实际: {:?}", other),
        }
    }

    #[test]
    fn test_build_notes_summary_truncates_to_200_chars() {
        let long_content: String = "a".repeat(300);
        let note = make_note("长内容", &long_content, "2026-07-15T10:00:00+00:00");
        let summary = build_notes_summary(&[note]);
        // content 部分应被截断为 200 字符（300 个 a 中只保留前 200 个）
        let a_count = summary.chars().filter(|c| *c == 'a').count();
        assert_eq!(
            a_count, 200,
            "content 部分应被截断为 200 字符，实际: {}",
            a_count
        );
        // 摘要应包含日期和标题
        assert!(summary.contains("[2026-07-15]"), "摘要应包含日期");
        assert!(summary.contains("长内容"), "摘要应包含标题");
    }

    #[test]
    fn test_build_notes_summary_limits_to_20_notes() {
        let notes: Vec<Note> = (0..25)
            .map(|i| make_note(&format!("笔记{}", i), "内容", "2026-07-15T10:00:00+00:00"))
            .collect();
        let summary = build_notes_summary(&notes);
        let line_count = summary.lines().count();
        assert_eq!(line_count, 20, "摘要应限制为 20 条，实际: {}", line_count);
    }

    #[test]
    fn test_generate_report_weekly_title_format() {
        let period = ReportPeriod::Weekly {
            start: "2026-07-13".to_string(),
            end: "2026-07-19".to_string(),
        };
        let title = build_report_title(&period);
        assert_eq!(title, "2026-07-13 ~ 07-19 周报");
    }

    #[test]
    fn test_generate_report_monthly_title_format() {
        let period = ReportPeriod::Monthly { year: 2026, month: 7 };
        let title = build_report_title(&period);
        assert_eq!(title, "2026-07 月报");
    }

    #[test]
    fn test_build_notes_summary_orders_by_updated_at_desc() {
        let notes = vec![
            make_note("旧", "内容1", "2026-07-10T10:00:00+00:00"),
            make_note("新", "内容2", "2026-07-20T10:00:00+00:00"),
            make_note("中", "内容3", "2026-07-15T10:00:00+00:00"),
        ];
        let summary = build_notes_summary(&notes);
        let lines: Vec<&str> = summary.lines().collect();
        // 倒序：新 → 中 → 旧
        assert!(lines[0].contains("新"), "第一行应为最新的便签");
        assert!(lines[1].contains("中"), "第二行应为中间的便签");
        assert!(lines[2].contains("旧"), "第三行应为最旧的便签");
    }
}
