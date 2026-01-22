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

/// Check if a rule is currently active based on time constraints
pub fn is_time_valid(rule: &PriceRule, current_time: i64) -> bool {
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
            time_mode: TimeMode::Always,
            start_time: None,
            end_time: None,
            schedule_config: None,
            is_active: true,
            created_by: None,
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
}
