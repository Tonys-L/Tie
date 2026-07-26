//! 月份边界计算单一所有者（LES-024 候选1）
//!
//! 职责：
//! - 将 year/month 转换为 ISO 字符串边界（用于 SQL/Repo 查询）
//! - 计算 month 内天数（用于日历视图遍历）
//!
//! 调用方：
//! - `commands/reminder_commands.rs` get_reminders_by_month（find_by_date_range 边界）
//! - `commands/reminder_commands.rs` get_lunar_dates（days_in_month）
//! - `infrastructure/sqlite_note_repo.rs` find_activity_by_month（SQL WHERE 边界）
//!
//! 依赖：chrono

use chrono::{Datelike, NaiveDate};

const ISO_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

/// 校验 month 在 1..=12 范围内
fn validate_month(month: u32) -> Result<(), String> {
    if month == 0 || month > 12 {
        Err("无效的年月".to_string())
    } else {
        Ok(())
    }
}

/// 返回下个月 1 号的 NaiveDate（用于计算 end 边界和 days_in_month）
///
/// 12 月特判：year + 1, month = 1
fn next_month_first(year: i32, month: u32) -> Result<NaiveDate, String> {
    validate_month(month)?;
    let (y, m) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(y, m, 1).ok_or("无效的年月".to_string())
}

/// 月份起始 ISO 字符串（1 号 00:00:00.000Z）
pub fn month_start_iso(year: i32, month: u32) -> Result<String, String> {
    validate_month(month)?;
    let start = NaiveDate::from_ymd_opt(year, month, 1).ok_or("无效的年月")?;
    Ok(start
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .format(ISO_FORMAT)
        .to_string())
}

/// 月份结束 ISO 字符串（下月 1 号 00:00:00.000Z，半开区间）
pub fn month_end_iso(year: i32, month: u32) -> Result<String, String> {
    let end = next_month_first(year, month)?;
    Ok(end
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .format(ISO_FORMAT)
        .to_string())
}

/// 月份天数（28/29/30/31）
pub fn days_in_month(year: i32, month: u32) -> Result<u32, String> {
    let next = next_month_first(year, month)?;
    let last_day = next.pred_opt().ok_or("无效的年月")?;
    Ok(last_day.day())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_month_start_iso_normal() {
        let s = month_start_iso(2026, 7).unwrap();
        assert_eq!(s, "2026-07-01T00:00:00.000Z");
    }

    #[test]
    fn test_month_end_iso_normal() {
        let s = month_end_iso(2026, 7).unwrap();
        assert_eq!(s, "2026-08-01T00:00:00.000Z");
    }

    #[test]
    fn test_month_end_iso_december_rollover() {
        let s = month_end_iso(2026, 12).unwrap();
        assert_eq!(s, "2027-01-01T00:00:00.000Z");
    }

    #[test]
    fn test_month_start_iso_january() {
        let s = month_start_iso(2026, 1).unwrap();
        assert_eq!(s, "2026-01-01T00:00:00.000Z");
    }

    #[test]
    fn test_days_in_month_february_leap() {
        assert_eq!(days_in_month(2024, 2).unwrap(), 29);
    }

    #[test]
    fn test_days_in_month_february_common() {
        assert_eq!(days_in_month(2026, 2).unwrap(), 28);
    }

    #[test]
    fn test_days_in_month_december() {
        assert_eq!(days_in_month(2026, 12).unwrap(), 31);
    }

    #[test]
    fn test_days_in_month_april() {
        assert_eq!(days_in_month(2026, 4).unwrap(), 30);
    }

    #[test]
    fn test_invalid_month_returns_err() {
        assert!(month_start_iso(2026, 13).is_err());
        assert!(month_end_iso(2026, 0).is_err());
        assert!(days_in_month(2026, 13).is_err());
    }
}
