//! Price Rule Model

use super::serde_helpers;
use super::serde_thing;
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

/// Zone scope constants
pub const ZONE_SCOPE_ALL: &str = "zone:all";
pub const ZONE_SCOPE_RETAIL: &str = "zone:retail";

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
    #[serde(default, with = "serde_thing::option")]
    pub id: Option<Thing>,
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,
    pub description: Option<String>,
    pub rule_type: RuleType,
    pub product_scope: ProductScope,
    /// Target record based on scope (category/tag/product)
    #[serde(default, with = "serde_thing::option")]
    pub target: Option<Thing>,
    /// Zone scope: "zone:all", "zone:retail", or specific zone ID like "zone:xxx"
    #[serde(default = "default_zone_scope")]
    pub zone_scope: String,
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
    #[serde(default, with = "serde_thing::option")]
    pub created_by: Option<Thing>,
    /// Created timestamp (milliseconds since epoch)
    #[serde(default)]
    pub created_at: i64,
}

fn default_true() -> bool {
    true
}

fn default_zone_scope() -> String {
    ZONE_SCOPE_ALL.to_string()
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
    /// Target ID as string (e.g., "category:xxx", "tag:xxx", "product:xxx")
    pub target: Option<String>,
    /// Zone scope: "zone:all", "zone:retail", or specific zone ID like "zone:xxx"
    pub zone_scope: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_type: Option<RuleType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_scope: Option<ProductScope>,
    /// Target ID as string (e.g., "category:xxx", "tag:xxx", "product:xxx")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Zone scope: "zone:all", "zone:retail", or specific zone ID like "zone:xxx"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone_scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjustment_type: Option<AdjustmentType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjustment_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_stackable: Option<bool>,
    /// Whether this rule is exclusive (cannot be combined with other rules)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_exclusive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_mode: Option<TimeMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}
