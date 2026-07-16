use chrono::Datelike;
use tyme4rs::tyme::solar::SolarDay;
use tyme4rs::tyme::lunar::LunarDay;
use tyme4rs::tyme::Tyme;

/// 计算农历月份+1后的公历 ISO 时间
///
/// 将公历 ISO 时间转为农历，农历月+1，转回公历。
/// 保持原有时分秒不变。若目标农历月没有该日（小月29天），取月末。
pub fn lunar_next_month(iso_time: &str) -> Option<String> {
    let dt = chrono::DateTime::parse_from_rfc3339(iso_time).ok()?;

    // 公历 → 农历日
    let solar = SolarDay::from_ymd(
        dt.year() as isize,
        dt.month() as usize,
        dt.day() as usize,
    );
    let lunar_day = solar.get_lunar_day();

    // 农历日号
    let day = lunar_day.get_day();

    // 下一个农历月（自动处理闰月）
    let next_month = lunar_day.get_lunar_month().next(1);
    let day_count = next_month.get_day_count();
    let actual_day = day.min(day_count);

    // 构造下月同一天的农历日
    let next_lunar_day = LunarDay::from_ymd(
        next_month.get_year(),
        next_month.get_month_with_leap(),
        actual_day,
    );

    // 转回公历
    let next_solar = next_lunar_day.get_solar_day();

    let next_date = chrono::NaiveDate::from_ymd_opt(
        next_solar.get_year() as i32,
        next_solar.get_month() as u32,
        next_solar.get_day() as u32,
    )?;

    let naive = chrono::NaiveDateTime::new(next_date, dt.naive_utc().time());
    Some(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc).to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lunar_next_month_basic() {
        // 2026-07-15 农历六月初二 → 下月农历七月初二 = 2026-08-14
        let result = lunar_next_month("2026-07-15T08:00:00Z");
        assert!(result.is_some());
        let next = result.unwrap();
        assert!(next.contains("2026-08-14"), "expected 2026-08-14, got {}", next);
    }

    #[test]
    fn test_lunar_next_month_keeps_time() {
        let result = lunar_next_month("2026-07-15T14:30:00Z");
        assert!(result.is_some());
        let next = result.unwrap();
        assert!(next.contains("T14:30:00"), "time should be preserved, got {}", next);
    }

    #[test]
    fn test_lunar_next_month_invalid_input() {
        assert!(lunar_next_month("not-a-date").is_none());
    }
}
