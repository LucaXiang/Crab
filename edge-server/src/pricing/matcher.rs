//! Price Rule Matcher
//!
//! Logic for matching rules to products and checking time validity.

use crate::db::models::{PriceRule, ProductScope, TimeMode};
use chrono::{Datelike, Local, NaiveTime, Timelike, Weekday};

/// Check if a rule matches a product based on scope
pub fn matches_product_scope(
    rule: &PriceRule,
    product_id: &str,
    category_id: Option<&str>,
    tags: &[String],
) -> bool {
    match rule.product_scope {
        ProductScope::Global => true,
        ProductScope::Product => {
            if let Some(target) = &rule.target {
                // target is Thing like "product:xxx"
                let target_id = target.id.to_raw();
                target_id == product_id || format!("product:{}", product_id) == target.to_string()
            } else {
                false
            }
        }
        ProductScope::Category => {
            if let (Some(target), Some(cat_id)) = (&rule.target, category_id) {
                let target_id = target.id.to_raw();
                target_id == cat_id || format!("category:{}", cat_id) == target.to_string()
            } else {
                false
            }
        }
        ProductScope::Tag => {
            if let Some(target) = &rule.target {
                let target_id = target.id.to_raw();
                tags.iter()
                    .any(|t| t == &target_id || format!("tag:{}", t) == target.to_string())
            } else {
                false
            }
        }
    }
}

/// Check if a rule matches the zone scope
/// zone_scope: -1=all zones, 0=retail only, >0=specific zone
pub fn matches_zone_scope(rule: &PriceRule, zone_id: Option<&str>, is_retail: bool) -> bool {
    match rule.zone_scope {
        -1 => true,     // All zones
        0 => is_retail, // Retail only
        zone_scope => {
            // Specific zone
            if let Some(zid) = zone_id {
                zid == zone_scope.to_string()
            } else {
                false
            }
        }
    }
}

/// Check if rule is valid at the given timestamp
///
/// This function checks the new time control fields:
/// - valid_from/valid_until: absolute validity period
/// - active_days: days of week filter (0=Sunday, 1=Monday, ..., 6=Saturday)
/// - active_start_time/active_end_time: time of day filter (HH:MM format)
pub fn is_time_valid(rule: &PriceRule, current_time: i64) -> bool {
    // Check valid_from
    if let Some(from) = rule.valid_from
        && current_time < from
    {
        return false;
    }

    // Check valid_until
    if let Some(until) = rule.valid_until
        && current_time > until
    {
        return false;
    }

    // Check active_days (0=Sunday, 1=Monday, ..., 6=Saturday)
    if let Some(ref days) = rule.active_days {
        let datetime = chrono::DateTime::from_timestamp_millis(current_time)
            .unwrap_or_else(chrono::Utc::now);
        // chrono's weekday: Mon=0, Tue=1, ..., Sun=6
        // We need: Sun=0, Mon=1, ..., Sat=6
        let weekday = datetime.weekday().num_days_from_sunday() as u8;
        if !days.contains(&weekday) {
            return false;
        }
    }

    // Check active_start_time/active_end_time ("HH:MM" format)
    if rule.active_start_time.is_some() || rule.active_end_time.is_some() {
        let datetime = chrono::DateTime::from_timestamp_millis(current_time)
            .unwrap_or_else(chrono::Utc::now);
        let current_time_str = datetime.format("%H:%M").to_string();

        if let Some(ref start) = rule.active_start_time
            && current_time_str < *start
        {
            return false;
        }

        if let Some(ref end) = rule.active_end_time
            && current_time_str > *end
        {
            return false;
        }
    }

    true
}

/// Legacy time validation based on time_mode
/// Kept for backward compatibility
#[allow(dead_code)]
pub fn is_time_valid_legacy(rule: &PriceRule, current_time: i64) -> bool {
    match rule.time_mode {
        TimeMode::Always => true,
        TimeMode::Schedule => check_schedule(rule, current_time),
        TimeMode::Onetime => check_onetime(rule, current_time),
    }
}

/// Check schedule-based time constraint
fn check_schedule(rule: &PriceRule, _current_time: i64) -> bool {
    let now = Local::now();

    // Check day of week
    if let Some(ref config) = rule.schedule_config {
        if let Some(ref days) = config.days_of_week {
            let current_day = match now.weekday() {
                Weekday::Sun => 0,
                Weekday::Mon => 1,
                Weekday::Tue => 2,
                Weekday::Wed => 3,
                Weekday::Thu => 4,
                Weekday::Fri => 5,
                Weekday::Sat => 6,
            };
            if !days.contains(&current_day) {
                return false;
            }
        }

        // Check time range
        if let (Some(start), Some(end)) = (&config.start_time, &config.end_time)
            && let (Ok(start_time), Ok(end_time)) = (
                NaiveTime::parse_from_str(start, "%H:%M"),
                NaiveTime::parse_from_str(end, "%H:%M"),
            )
        {
            let current_time = NaiveTime::from_hms_opt(now.hour(), now.minute(), 0)
                .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap());

            // Handle overnight ranges (e.g., 22:00 - 02:00)
            if start_time <= end_time {
                if !(current_time >= start_time && current_time <= end_time) {
                    return false;
                }
            } else {
                // Overnight
                if !(current_time >= start_time || current_time <= end_time) {
                    return false;
                }
            }
        }
    }

    true
}

/// Check one-time date range constraint
fn check_onetime(rule: &PriceRule, current_time: i64) -> bool {
    // start_time and end_time are ISO8601 date strings
    if let (Some(start), Some(end)) = (&rule.start_time, &rule.end_time)
        && let (Ok(start_dt), Ok(end_dt)) = (
            chrono::DateTime::parse_from_rfc3339(start),
            chrono::DateTime::parse_from_rfc3339(end),
        )
    {
        let current =
            chrono::DateTime::from_timestamp_millis(current_time).unwrap_or_else(chrono::Utc::now);

        return current >= start_dt && current <= end_dt;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{AdjustmentType, RuleType};
    use surrealdb::sql::Thing;

    fn make_rule(product_scope: ProductScope, target: Option<&str>) -> PriceRule {
        PriceRule {
            id: None,
            name: "test".to_string(),
            display_name: "Test".to_string(),
            receipt_name: "TEST".to_string(),
            description: None,
            rule_type: RuleType::Discount,
            product_scope,
            target: target.map(|t| {
                Thing::from((
                    t.split(':').next().unwrap_or("product"),
                    t.split(':').last().unwrap_or(t),
                ))
            }),
            zone_scope: -1,
            adjustment_type: AdjustmentType::Percentage,
            adjustment_value: 10,
            priority: 0,
            is_stackable: true,
            is_exclusive: false,
            time_mode: TimeMode::Always,
            start_time: None,
            end_time: None,
            schedule_config: None,
            valid_from: None,
            valid_until: None,
            active_days: None,
            active_start_time: None,
            active_end_time: None,
            is_active: true,
            created_by: None,
            created_at: 0,
        }
    }

    #[test]
    fn test_global_scope_matches_all() {
        let rule = make_rule(ProductScope::Global, None);
        assert!(matches_product_scope(&rule, "123", Some("cat1"), &[]));
    }

    #[test]
    fn test_product_scope_matches_specific() {
        let rule = make_rule(ProductScope::Product, Some("product:123"));

        // Debug: print what we're comparing
        if let Some(target) = &rule.target {
            eprintln!("target: {:?}", target);
            eprintln!("target.to_string(): {}", target.to_string());
            eprintln!("target.id: {:?}", target.id);
            eprintln!("target.id.to_string(): {}", target.id.to_string());
            eprintln!("target.id.to_raw(): {}", target.id.to_raw());
            eprintln!("target.tb: {}", target.tb);
        }

        assert!(matches_product_scope(&rule, "123", Some("cat1"), &[]));
        assert!(!matches_product_scope(&rule, "456", Some("cat1"), &[]));
    }

    #[test]
    fn test_zone_scope_all() {
        let mut rule = make_rule(ProductScope::Global, None);
        rule.zone_scope = -1;
        assert!(matches_zone_scope(&rule, Some("1"), false));
        assert!(matches_zone_scope(&rule, None, true));
    }

    #[test]
    fn test_zone_scope_retail_only() {
        let mut rule = make_rule(ProductScope::Global, None);
        rule.zone_scope = 0;
        assert!(matches_zone_scope(&rule, None, true));
        assert!(!matches_zone_scope(&rule, Some("1"), false));
    }

    // ===========================================
    // New time field tests
    // ===========================================

    #[test]
    fn test_valid_from_future() {
        // Rule not yet effective (valid_from is in the future)
        let mut rule = make_rule(ProductScope::Global, None);
        let now = chrono::Utc::now().timestamp_millis();
        rule.valid_from = Some(now + 3600_000); // 1 hour in the future

        assert!(!is_time_valid(&rule, now));
    }

    #[test]
    fn test_valid_until_past() {
        // Rule expired (valid_until is in the past)
        let mut rule = make_rule(ProductScope::Global, None);
        let now = chrono::Utc::now().timestamp_millis();
        rule.valid_until = Some(now - 3600_000); // 1 hour ago

        assert!(!is_time_valid(&rule, now));
    }

    #[test]
    fn test_valid_within_range() {
        // Rule within valid range
        let mut rule = make_rule(ProductScope::Global, None);
        let now = chrono::Utc::now().timestamp_millis();
        rule.valid_from = Some(now - 3600_000); // 1 hour ago
        rule.valid_until = Some(now + 3600_000); // 1 hour from now

        assert!(is_time_valid(&rule, now));
    }

    #[test]
    fn test_active_days() {
        // Check day of week filter
        let mut rule = make_rule(ProductScope::Global, None);

        // Use a known timestamp: 2024-01-15 12:00:00 UTC is Monday (weekday=1)
        let monday_noon = chrono::DateTime::parse_from_rfc3339("2024-01-15T12:00:00Z")
            .unwrap()
            .timestamp_millis();

        // Only active on Monday (1)
        rule.active_days = Some(vec![1]); // Monday
        assert!(is_time_valid(&rule, monday_noon));

        // Only active on Sunday (0)
        rule.active_days = Some(vec![0]); // Sunday
        assert!(!is_time_valid(&rule, monday_noon));

        // Active on Monday and Friday
        rule.active_days = Some(vec![1, 5]); // Monday and Friday
        assert!(is_time_valid(&rule, monday_noon));
    }

    #[test]
    fn test_active_time_range() {
        // Check time of day filter
        let mut rule = make_rule(ProductScope::Global, None);

        // Use a known timestamp: 2024-01-15 14:30:00 UTC
        let timestamp = chrono::DateTime::parse_from_rfc3339("2024-01-15T14:30:00Z")
            .unwrap()
            .timestamp_millis();

        // Active from 10:00 to 18:00
        rule.active_start_time = Some("10:00".to_string());
        rule.active_end_time = Some("18:00".to_string());
        assert!(is_time_valid(&rule, timestamp)); // 14:30 is within range

        // Active from 16:00 to 20:00
        rule.active_start_time = Some("16:00".to_string());
        rule.active_end_time = Some("20:00".to_string());
        assert!(!is_time_valid(&rule, timestamp)); // 14:30 is before 16:00

        // Active from 08:00 to 12:00
        rule.active_start_time = Some("08:00".to_string());
        rule.active_end_time = Some("12:00".to_string());
        assert!(!is_time_valid(&rule, timestamp)); // 14:30 is after 12:00
    }

    #[test]
    fn test_combined_time_constraints() {
        // Test combining valid_from/until with active_days and time range
        let mut rule = make_rule(ProductScope::Global, None);

        // 2024-01-15 14:30:00 UTC is Monday
        let timestamp = chrono::DateTime::parse_from_rfc3339("2024-01-15T14:30:00Z")
            .unwrap()
            .timestamp_millis();

        // Set all constraints to pass
        rule.valid_from = Some(timestamp - 86400_000); // 1 day ago
        rule.valid_until = Some(timestamp + 86400_000); // 1 day from now
        rule.active_days = Some(vec![1]); // Monday
        rule.active_start_time = Some("10:00".to_string());
        rule.active_end_time = Some("18:00".to_string());

        assert!(is_time_valid(&rule, timestamp));

        // Now make one constraint fail - wrong day
        rule.active_days = Some(vec![0]); // Sunday
        assert!(!is_time_valid(&rule, timestamp));

        // Reset day, make time fail
        rule.active_days = Some(vec![1]); // Monday
        rule.active_start_time = Some("16:00".to_string());
        assert!(!is_time_valid(&rule, timestamp));
    }

    #[test]
    fn test_no_time_constraints() {
        // Rule with no time constraints should always be valid
        let rule = make_rule(ProductScope::Global, None);
        let now = chrono::Utc::now().timestamp_millis();

        assert!(is_time_valid(&rule, now));
    }
}
