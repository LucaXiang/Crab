//! Price Rule Matcher
//!
//! Logic for matching rules to products and checking time validity.

use crate::db::models::{PriceRule, ProductScope};
use chrono::{Datelike, Local};
use tracing::trace;

/// Check if a rule matches a product based on scope
pub fn matches_product_scope(
    rule: &PriceRule,
    product_id: &str,
    category_id: Option<&str>,
    tags: &[String],
) -> bool {
    let result = match rule.product_scope {
        ProductScope::Global => {
            trace!(
                rule_name = %rule.name,
                product_scope = ?rule.product_scope,
                product_id,
                "[ProductScope] Global scope - matches all"
            );
            true
        }
        ProductScope::Product => {
            if let Some(target) = &rule.target {
                // target is Thing like "product:xxx", product_id should also be in full format
                // Use tb and id.to_raw() for consistent format (avoids SurrealDB's ⟨⟩ brackets)
                let target_str = format!("{}:{}", target.tb, target.id.to_raw());
                let matches = target_str == product_id;
                trace!(
                    rule_name = %rule.name,
                    product_scope = ?rule.product_scope,
                    target = %target_str,
                    product_id,
                    matches,
                    "[ProductScope] Product scope check"
                );
                matches
            } else {
                trace!(
                    rule_name = %rule.name,
                    product_scope = ?rule.product_scope,
                    product_id,
                    "[ProductScope] Product scope - no target defined"
                );
                false
            }
        }
        ProductScope::Category => {
            if let (Some(target), Some(cat_id)) = (&rule.target, category_id) {
                // target is Thing like "category:xxx", cat_id should also be in full format
                // Use tb and id.to_raw() for consistent format (avoids SurrealDB's ⟨⟩ brackets)
                let target_str = format!("{}:{}", target.tb, target.id.to_raw());
                let matches = target_str == cat_id;
                trace!(
                    rule_name = %rule.name,
                    product_scope = ?rule.product_scope,
                    target = %target_str,
                    category_id = %cat_id,
                    product_id,
                    matches,
                    "[ProductScope] Category scope check"
                );
                matches
            } else {
                trace!(
                    rule_name = %rule.name,
                    product_scope = ?rule.product_scope,
                    target = ?rule.target.as_ref().map(|t| format!("{}:{}", t.tb, t.id.to_raw())),
                    category_id = ?category_id,
                    product_id,
                    "[ProductScope] Category scope - missing target or category_id"
                );
                false
            }
        }
        ProductScope::Tag => {
            if let Some(target) = &rule.target {
                // target is Thing like "tag:xxx", tags should also be in full format
                // Use tb and id.to_raw() for consistent format (avoids SurrealDB's ⟨⟩ brackets)
                let target_str = format!("{}:{}", target.tb, target.id.to_raw());
                let matches = tags.iter().any(|t| t == &target_str);
                trace!(
                    rule_name = %rule.name,
                    product_scope = ?rule.product_scope,
                    target = %target_str,
                    product_id,
                    tags = ?tags,
                    matches,
                    "[ProductScope] Tag scope check"
                );
                matches
            } else {
                trace!(
                    rule_name = %rule.name,
                    product_scope = ?rule.product_scope,
                    product_id,
                    "[ProductScope] Tag scope - no target defined"
                );
                false
            }
        }
    };

    trace!(
        rule_name = %rule.name,
        product_id,
        category_id = ?category_id,
        tags_count = tags.len(),
        result,
        "[ProductScope] Final match result"
    );

    result
}

/// Zone scope constants
pub const ZONE_SCOPE_ALL: &str = "zone:all";
pub const ZONE_SCOPE_RETAIL: &str = "zone:retail";

/// Check if a rule matches the zone scope
/// zone_scope: "zone:all" = all zones, "zone:retail" = retail only, "zone:xxx" = specific zone
pub fn matches_zone_scope(rule: &PriceRule, zone_id: Option<&str>, is_retail: bool) -> bool {
    match rule.zone_scope.as_str() {
        ZONE_SCOPE_ALL => true,      // All zones
        ZONE_SCOPE_RETAIL => is_retail, // Retail only
        zone_scope => {
            // Specific zone - direct string comparison
            if let Some(zid) = zone_id {
                zid == zone_scope
            } else {
                false
            }
        }
    }
}

/// Check if rule is valid at the given timestamp
///
/// This function checks time control fields:
/// - valid_from/valid_until: absolute validity period (DateTime<Utc>)
/// - active_days: days of week filter (0=Sunday, 1=Monday, ..., 6=Saturday) - local time
/// - active_start_time/active_end_time: time of day filter (HH:MM format) - local time
///
/// Note: active_days and active_time use LOCAL time (with DST handling) since rules
/// are typically configured in local business hours.
pub fn is_time_valid(rule: &PriceRule, current_time: i64) -> bool {
    // Convert current_time to DateTime for comparison
    let current_datetime =
        chrono::DateTime::from_timestamp_millis(current_time).unwrap_or_else(chrono::Utc::now);

    // Check valid_from
    if let Some(ref from) = rule.valid_from
        && current_datetime < *from
    {
        return false;
    }

    // Check valid_until
    if let Some(ref until) = rule.valid_until
        && current_datetime > *until
    {
        return false;
    }

    // Convert to local time for day-of-week and time-of-day checks
    // This handles daylight saving time automatically
    let local_datetime = current_datetime.with_timezone(&Local);

    // Check active_days (0=Sunday, 1=Monday, ..., 6=Saturday) in LOCAL time
    if let Some(ref days) = rule.active_days {
        // chrono's weekday: Mon=0, Tue=1, ..., Sun=6
        // We need: Sun=0, Mon=1, ..., Sat=6
        let weekday = local_datetime.weekday().num_days_from_sunday() as u8;
        if !days.contains(&weekday) {
            return false;
        }
    }

    // Check active_start_time/active_end_time ("HH:MM" format) in LOCAL time
    if rule.active_start_time.is_some() || rule.active_end_time.is_some() {
        let current_time_str = local_datetime.format("%H:%M").to_string();

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



#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{AdjustmentType, RuleType};
    use chrono::Utc;
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
            zone_scope: ZONE_SCOPE_ALL.to_string(),
            adjustment_type: AdjustmentType::Percentage,
            adjustment_value: 10.0,
            priority: 0,
            is_stackable: false,
            is_exclusive: false,
            valid_from: None,
            valid_until: None,
            active_days: None,
            active_start_time: None,
            active_end_time: None,
            is_active: true,
            created_by: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_global_scope_matches_all() {
        let rule = make_rule(ProductScope::Global, None);
        // All IDs should be in full format "table:id"
        assert!(matches_product_scope(
            &rule,
            "product:123",
            Some("category:cat1"),
            &[]
        ));
    }

    #[test]
    fn test_product_scope_matches_specific() {
        let rule = make_rule(ProductScope::Product, Some("product:123"));

        // All IDs should be in full format "table:id"
        assert!(matches_product_scope(
            &rule,
            "product:123",
            Some("category:cat1"),
            &[]
        ));
        assert!(!matches_product_scope(
            &rule,
            "product:456",
            Some("category:cat1"),
            &[]
        ));
    }

    #[test]
    fn test_zone_scope_all() {
        let mut rule = make_rule(ProductScope::Global, None);
        rule.zone_scope = ZONE_SCOPE_ALL.to_string();
        assert!(matches_zone_scope(&rule, Some("zone:1"), false));
        assert!(matches_zone_scope(&rule, None, true));
    }

    #[test]
    fn test_zone_scope_retail_only() {
        let mut rule = make_rule(ProductScope::Global, None);
        rule.zone_scope = ZONE_SCOPE_RETAIL.to_string();
        assert!(matches_zone_scope(&rule, None, true));
        assert!(!matches_zone_scope(&rule, Some("zone:1"), false));
    }

    // ===========================================
    // Time field tests
    // ===========================================

    #[test]
    fn test_valid_from_future() {
        // Rule not yet effective (valid_from is in the future)
        let mut rule = make_rule(ProductScope::Global, None);
        let now = Utc::now();
        let now_millis = now.timestamp_millis();
        rule.valid_from = Some(now + chrono::Duration::hours(1)); // 1 hour in the future

        assert!(!is_time_valid(&rule, now_millis));
    }

    #[test]
    fn test_valid_until_past() {
        // Rule expired (valid_until is in the past)
        let mut rule = make_rule(ProductScope::Global, None);
        let now = Utc::now();
        let now_millis = now.timestamp_millis();
        rule.valid_until = Some(now - chrono::Duration::hours(1)); // 1 hour ago

        assert!(!is_time_valid(&rule, now_millis));
    }

    #[test]
    fn test_valid_within_range() {
        // Rule within valid range
        let mut rule = make_rule(ProductScope::Global, None);
        let now = Utc::now();
        let now_millis = now.timestamp_millis();
        rule.valid_from = Some(now - chrono::Duration::hours(1)); // 1 hour ago
        rule.valid_until = Some(now + chrono::Duration::hours(1)); // 1 hour from now

        assert!(is_time_valid(&rule, now_millis));
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
        let monday = chrono::DateTime::parse_from_rfc3339("2024-01-15T14:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let timestamp = monday.timestamp_millis();

        // Set all constraints to pass
        rule.valid_from = Some(monday - chrono::Duration::days(1)); // 1 day ago
        rule.valid_until = Some(monday + chrono::Duration::days(1)); // 1 day from now
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
        let now = Utc::now().timestamp_millis();

        assert!(is_time_valid(&rule, now));
    }
}
