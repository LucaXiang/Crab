//! Price Rule Model

use serde::{Deserialize, Serialize};

/// Rule type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuleType {
    Discount,
    Surcharge,
}

/// Product scope enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProductScope {
    Global,
    Category,
    Tag,
    Product,
}

/// Adjustment type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdjustmentType {
    Percentage,
    FixedAmount,
}

/// Time mode enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeMode {
    #[default]
    Always,
    Schedule,
    Onetime,
}

/// Schedule config for recurring rules
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScheduleConfig {
    /// Days of week (0=Sunday, 1=Monday, ...)
    pub days_of_week: Option<Vec<i32>>,
    /// Start time (HH:MM)
    pub start_time: Option<String>,
    /// End time (HH:MM)
    pub end_time: Option<String>,
}

/// Price rule entity (价格调整规则)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceRule {
    pub id: Option<String>,
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,
    pub description: Option<String>,
    pub rule_type: RuleType,
    pub product_scope: ProductScope,
    /// Target record based on scope (category/tag/product ID)
    pub target: Option<String>,
    /// Zone scope: "zone:all", "zone:retail", or specific zone ID like "zone:xxx"
    pub zone_scope: String,
    pub adjustment_type: AdjustmentType,
    /// Adjustment value (percentage: 30=30%, fixed: currency unit e.g. 5.00 = ¥5.00)
    pub adjustment_value: f64,
    pub priority: i32,
    pub is_stackable: bool,
    /// Whether this rule is exclusive (cannot be combined with other rules)
    #[serde(default)]
    pub is_exclusive: bool,
    pub time_mode: TimeMode,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub schedule_config: Option<ScheduleConfig>,
    /// Valid from timestamp (milliseconds since epoch)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<i64>,
    /// Valid until timestamp (milliseconds since epoch)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<i64>,
    /// Active days of week (0=Sunday, 1=Monday, ..., 6=Saturday)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_days: Option<Vec<u8>>,
    /// Active start time (HH:MM format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_start_time: Option<String>,
    /// Active end time (HH:MM format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_end_time: Option<String>,
    pub is_active: bool,
    pub created_by: Option<String>,
    /// Created timestamp (milliseconds since epoch)
    #[serde(default)]
    pub created_at: i64,
}

/// Create price rule payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceRuleCreate {
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,
    pub description: Option<String>,
    pub rule_type: RuleType,
    pub product_scope: ProductScope,
    pub target: Option<String>,
    /// Zone scope: "zone:all", "zone:retail", or specific zone ID
    pub zone_scope: Option<String>,
    pub adjustment_type: AdjustmentType,
    /// Adjustment value (percentage: 30=30%, fixed: currency unit e.g. 5.00 = ¥5.00)
    pub adjustment_value: f64,
    pub priority: Option<i32>,
    pub is_stackable: Option<bool>,
    /// Whether this rule is exclusive (cannot be combined with other rules)
    pub is_exclusive: Option<bool>,
    pub time_mode: Option<TimeMode>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub schedule_config: Option<ScheduleConfig>,
    /// Valid from timestamp (milliseconds since epoch)
    pub valid_from: Option<i64>,
    /// Valid until timestamp (milliseconds since epoch)
    pub valid_until: Option<i64>,
    /// Active days of week (0=Sunday, 1=Monday, ..., 6=Saturday)
    pub active_days: Option<Vec<u8>>,
    /// Active start time (HH:MM format)
    pub active_start_time: Option<String>,
    /// Active end time (HH:MM format)
    pub active_end_time: Option<String>,
    pub created_by: Option<String>,
}

/// Update price rule payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceRuleUpdate {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub receipt_name: Option<String>,
    pub description: Option<String>,
    pub rule_type: Option<RuleType>,
    pub product_scope: Option<ProductScope>,
    pub target: Option<String>,
    /// Zone scope: "zone:all", "zone:retail", or specific zone ID
    pub zone_scope: Option<String>,
    pub adjustment_type: Option<AdjustmentType>,
    /// Adjustment value (percentage: 30=30%, fixed: currency unit e.g. 5.00 = ¥5.00)
    pub adjustment_value: Option<f64>,
    pub priority: Option<i32>,
    pub is_stackable: Option<bool>,
    /// Whether this rule is exclusive (cannot be combined with other rules)
    pub is_exclusive: Option<bool>,
    pub time_mode: Option<TimeMode>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub schedule_config: Option<ScheduleConfig>,
    /// Valid from timestamp (milliseconds since epoch)
    pub valid_from: Option<i64>,
    /// Valid until timestamp (milliseconds since epoch)
    pub valid_until: Option<i64>,
    /// Active days of week (0=Sunday, 1=Monday, ..., 6=Saturday)
    pub active_days: Option<Vec<u8>>,
    /// Active start time (HH:MM format)
    pub active_start_time: Option<String>,
    /// Active end time (HH:MM format)
    pub active_end_time: Option<String>,
    pub is_active: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_rule_time_fields() {
        let rule = PriceRule {
            id: Some("rule-1".to_string()),
            name: "test".to_string(),
            display_name: "Test Rule".to_string(),
            receipt_name: "TEST".to_string(),
            description: None,
            rule_type: RuleType::Discount,
            product_scope: ProductScope::Global,
            target: None,
            zone_scope: "zone:all".to_string(),
            adjustment_type: AdjustmentType::Percentage,
            adjustment_value: 10.0,
            priority: 0,
            is_stackable: true,
            is_exclusive: false,
            valid_from: Some(1704067200000),  // 2024-01-01
            valid_until: Some(1735689600000), // 2025-01-01
            active_days: Some(vec![1, 2, 3, 4, 5]), // Mon-Fri
            active_start_time: Some("11:00".to_string()),
            active_end_time: Some("14:00".to_string()),
            is_active: true,
            created_by: None,
            created_at: 1704067200000,
            time_mode: TimeMode::Schedule,
            start_time: None,
            end_time: None,
            schedule_config: None,
        };

        assert!(!rule.is_exclusive);
        assert!(rule.valid_from.is_some());
        assert_eq!(rule.active_days.as_ref().unwrap().len(), 5);
    }
}
