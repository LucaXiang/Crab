//! 时间工具函数 — 业务时区转换
//!
//! 所有日期→时间戳转换统一在 API handler 层完成，
//! repository 层只接收 `i64` Unix millis。

use chrono::{NaiveDate, NaiveTime};
use chrono_tz::Tz;

use super::{AppError, AppResult};

/// 解析日期字符串 (YYYY-MM-DD)
pub fn parse_date(date: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| AppError::validation(format!("Invalid date format: {}", date)))
}

/// 验证日期不在未来 (业务时区)
pub fn validate_not_future(date: NaiveDate, tz: Tz) -> AppResult<()> {
    let today = chrono::Utc::now().with_timezone(&tz).date_naive();
    if date > today {
        return Err(AppError::validation(format!(
            "Date {} is in the future (today is {})",
            date, today
        )));
    }
    Ok(())
}

/// 日期 + 时分秒 → Unix millis (业务时区)
///
/// DST gap fallback: 如果本地时间不存在 (夏令时跳跃)，fallback 到 UTC。
pub fn date_hms_to_millis(date: NaiveDate, hour: u32, min: u32, sec: u32, tz: Tz) -> i64 {
    let naive = date.and_hms_opt(hour, min, sec).unwrap();
    naive
        .and_local_timezone(tz)
        .latest()
        .map(|dt| dt.timestamp_millis())
        .unwrap_or_else(|| naive.and_utc().timestamp_millis())
}

/// 日期开始 (00:00:00) → Unix millis (业务时区)
pub fn day_start_millis(date: NaiveDate, tz: Tz) -> i64 {
    date_hms_to_millis(date, 0, 0, 0, tz)
}

/// 日期结束 → 次日 00:00:00 的 Unix millis (业务时区)
///
/// 返回次日零点时间戳，调用方使用 `< end` (不含) 语义。
pub fn day_end_millis(date: NaiveDate, tz: Tz) -> i64 {
    let next_day = date.succ_opt().unwrap_or(date);
    date_hms_to_millis(next_day, 0, 0, 0, tz)
}

/// 日期 + cutoff 时间 → Unix millis (业务时区)
///
/// 用于营业日边界计算 (business_day_cutoff)。
pub fn date_cutoff_millis(date: NaiveDate, cutoff: NaiveTime, tz: Tz) -> i64 {
    let naive = date.and_time(cutoff);
    naive
        .and_local_timezone(tz)
        .latest()
        .map(|dt| dt.timestamp_millis())
        .unwrap_or_else(|| naive.and_utc().timestamp_millis())
}

/// 解析 cutoff 时间字符串 (HH:MM)，失败返回 00:00
pub fn parse_cutoff(cutoff: &str) -> NaiveTime {
    NaiveTime::parse_from_str(cutoff, "%H:%M").unwrap_or_else(|e| {
        tracing::warn!(
            "Failed to parse business_day_cutoff '{}': {}, falling back to 00:00",
            cutoff,
            e
        );
        NaiveTime::MIN
    })
}

/// 计算当前营业日起始日期 (业务时区)
///
/// 当前时间 < cutoff → 还在"昨天"的营业日
/// 当前时间 >= cutoff → 当前营业日 = 今天
pub fn current_business_date(cutoff: NaiveTime, tz: Tz) -> NaiveDate {
    let now = chrono::Utc::now().with_timezone(&tz);
    if now.time() < cutoff {
        (now - chrono::Duration::days(1)).date_naive()
    } else {
        now.date_naive()
    }
}
