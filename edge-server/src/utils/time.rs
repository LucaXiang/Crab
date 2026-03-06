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
    let Some(naive) = date.and_hms_opt(hour, min, sec) else {
        // Invalid h/m/s — fallback to day start in UTC
        // SAFETY: (0,0,0) is always valid for NaiveDate::and_hms_opt
        return date.and_time(NaiveTime::MIN).and_utc().timestamp_millis();
    };
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

/// 从分钟偏移量构造 NaiveTime (e.g. 210 → 03:30, 480 → 08:00)
pub fn cutoff_to_time(minutes: i32) -> NaiveTime {
    let clamped = minutes.clamp(0, 480);
    let h = (clamped / 60) as u32;
    let m = (clamped % 60) as u32;
    // SAFETY: h ∈ [0,8], m ∈ [0,59], always valid
    NaiveTime::from_hms_opt(h, m, 0).unwrap_or(NaiveTime::MIN)
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

/// 计算距离下一次 cutoff 时间点的 Duration
///
/// 用于定时调度器 (班次检测、日报自动生成等)。
pub fn duration_until_next_cutoff(cutoff_time: NaiveTime, tz: Tz) -> std::time::Duration {
    let now = chrono::Utc::now().with_timezone(&tz);
    let today = now.date_naive();

    let target_date = if now.time() >= cutoff_time {
        today + chrono::Duration::days(1)
    } else {
        today
    };

    let target_datetime = target_date
        .and_time(cutoff_time)
        .and_local_timezone(tz)
        .single()
        .unwrap_or_else(|| {
            // DST edge case: fallback to +1 min
            (target_date.and_time(cutoff_time) + chrono::Duration::minutes(1))
                .and_local_timezone(tz)
                .latest()
                .unwrap_or_else(|| {
                    tracing::error!(
                        "Cannot resolve local time for cutoff scheduler, using fallback"
                    );
                    now + chrono::Duration::hours(1)
                })
        });

    let duration = target_datetime.signed_duration_since(now);
    if duration.num_seconds() <= 0 {
        std::time::Duration::from_secs(60)
    } else {
        duration
            .to_std()
            .unwrap_or(std::time::Duration::from_secs(60))
    }
}
