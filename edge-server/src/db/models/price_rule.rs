//! Price Rule Model

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

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
    pub id: Option<Thing>,
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,
    pub description: Option<String>,
    pub rule_type: RuleType,
    pub product_scope: ProductScope,
    /// Target record based on scope (category/tag/product)
    pub target: Option<Thing>,
    /// Zone scope: -1=all, 0=retail, >0=specific zone
    pub zone_scope: i32,
    pub adjustment_type: AdjustmentType,
    /// Adjustment value (percentage: 30=30%, fixed: amount in currency unit e.g. 5.00)
    pub adjustment_value: f64,
    #[serde(default)]
    pub priority: i32,
    #[serde(default, deserialize_with = "serde_helpers::bool_false")]
    pub is_stackable: bool,
    /// Whether this rule is exclusive (cannot be combined with other rules)
    #[serde(default, deserialize_with = "serde_helpers::bool_false")]
    pub is_exclusive: bool,
    #[serde(default)]
    pub time_mode: TimeMode,
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
    #[serde(
        default = "default_true",
        deserialize_with = "serde_helpers::bool_true"
    )]
    pub is_active: bool,
    pub created_by: Option<Thing>,
    /// Created timestamp (milliseconds since epoch)
    #[serde(default)]
    pub created_at: i64,
}

fn default_true() -> bool {
    true
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
    pub target: Option<Thing>,
    pub zone_scope: Option<i32>,
    pub adjustment_type: AdjustmentType,
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
    pub created_by: Option<Thing>,
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
    pub target: Option<Thing>,
    pub zone_scope: Option<i32>,
    pub adjustment_type: Option<AdjustmentType>,
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
