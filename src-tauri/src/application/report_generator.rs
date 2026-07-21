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

/// 按报告周期过滤便签（基于 updated_at 日期部分，YYYY-MM-DD 字符串比较）。
///
/// 此为报告数据拾取的业务规则（boundaries.md "周报/月报业务规则"），
/// 从命令层下沉至此以便单测覆盖各种边界（跨年、空范围、updated_at 格式异常等）。
pub fn filter_notes_by_date(notes: &[Note], start_date: &str, end_date: &str) -> Vec<Note> {
    notes
        .iter()
        .filter(|note| {
            let date_part: String = note.updated_at.chars().take(10).collect();
            date_part.as_str() >= start_date && date_part.as_str() <= end_date
        })
        .cloned()
        .collect()
}

/// 解析报告周期参数（命令层业务规则下沉）
///
/// 将 `period_type` 字符串 + `start_date`/`end_date` 解析为 `ReportPeriod` 枚举。
/// - `"weekly"` → `ReportPeriod::Weekly { start, end }`
/// - `"monthly"` → 从 `start_date` 解析 year/month → `ReportPeriod::Monthly { year, month }`
///
/// 错误场景：
/// - 未知 period_type → `Err("无效的 period_type: xxx，应为 weekly 或 monthly")`
/// - monthly 模式 start_date 格式异常 → `Err("无效的年份"/"无效的月份")`
pub fn parse_period(period_type: &str, start_date: &str, end_date: &str) -> Result<ReportPeriod, String> {
    match period_type {
        "weekly" => Ok(ReportPeriod::Weekly {
            start: start_date.to_string(),
            end: end_date.to_string(),
        }),
        "monthly" => {
            let year: u32 = start_date
                .chars()
                .take(4)
                .collect::<String>()
                .parse()
                .map_err(|_| "无效的年份".to_string())?;
            let month: u32 = start_date
                .chars()
                .skip(5)
                .take(2)
                .collect::<String>()
                .parse()
                .map_err(|_| "无效的月份".to_string())?;
            Ok(ReportPeriod::Monthly { year, month })
        }
        _ => Err(format!(
            "无效的 period_type: {}，应为 weekly 或 monthly",
            period_type
        )),
    }
}

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

    // ---- filter_notes_by_date 单测 ----

    #[test]
    fn test_filter_notes_by_date_empty_input() {
        let result = filter_notes_by_date(&[], "2026-07-01", "2026-07-31");
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_notes_by_date_all_in_range() {
        let notes = vec![
            make_note("a", "x", "2026-07-10T08:00:00+00:00"),
            make_note("b", "x", "2026-07-20T08:00:00+00:00"),
        ];
        let result = filter_notes_by_date(&notes, "2026-07-01", "2026-07-31");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_filter_notes_by_date_boundary_inclusive() {
        // 起始日和结束日的便签都应包含（闭区间）
        let notes = vec![
            make_note("start", "x", "2026-07-01T00:00:00+00:00"),
            make_note("end", "x", "2026-07-31T23:59:59+00:00"),
            make_note("before", "x", "2026-06-30T23:59:59+00:00"),
            make_note("after", "x", "2026-08-01T00:00:00+00:00"),
        ];
        let result = filter_notes_by_date(&notes, "2026-07-01", "2026-07-31");
        let titles: Vec<&str> = result.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(titles, vec!["start", "end"]);
    }

    #[test]
    fn test_filter_notes_by_date_cross_year() {
        let notes = vec![
            make_note("last_year", "x", "2025-12-31T10:00:00+00:00"),
            make_note("new_year", "x", "2026-01-01T10:00:00+00:00"),
            make_note("mid", "x", "2025-12-15T10:00:00+00:00"),
        ];
        let result = filter_notes_by_date(&notes, "2025-12-10", "2026-01-05");
        let titles: Vec<&str> = result.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(titles, vec!["last_year", "new_year", "mid"]);
    }

    #[test]
    fn test_filter_notes_by_date_no_match() {
        let notes = vec![
            make_note("a", "x", "2026-06-10T08:00:00+00:00"),
            make_note("b", "x", "2026-08-20T08:00:00+00:00"),
        ];
        let result = filter_notes_by_date(&notes, "2026-07-01", "2026-07-31");
        assert!(result.is_empty());
    }

    // ---- parse_period 单测 ----

    #[test]
    fn test_parse_period_weekly() {
        let period = parse_period("weekly", "2026-07-13", "2026-07-19").unwrap();
        assert_eq!(period, ReportPeriod::Weekly {
            start: "2026-07-13".to_string(),
            end: "2026-07-19".to_string(),
        });
    }

    #[test]
    fn test_parse_period_monthly() {
        let period = parse_period("monthly", "2026-07-01", "2026-07-31").unwrap();
        assert_eq!(period, ReportPeriod::Monthly { year: 2026, month: 7 });
    }

    #[test]
    fn test_parse_period_invalid_type() {
        let result = parse_period("daily", "2026-07-13", "2026-07-19");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("无效的 period_type"));
    }

    #[test]
    fn test_parse_period_monthly_invalid_year() {
        let result = parse_period("monthly", "abcd-07-01", "2026-07-31");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("无效的年份"));
    }

    #[test]
    fn test_parse_period_monthly_invalid_month() {
        let result = parse_period("monthly", "2026-xx-01", "2026-07-31");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("无效的月份"));
    }
}
