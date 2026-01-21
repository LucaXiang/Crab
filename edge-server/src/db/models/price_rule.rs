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
    /// Adjustment value (percentage: 30=30%, fixed: cents)
    pub adjustment_value: i32,
    #[serde(default)]
    pub priority: i32,
    #[serde(default, deserialize_with = "serde_helpers::bool_false")]
    pub is_stackable: bool,
    #[serde(default)]
    pub time_mode: TimeMode,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub schedule_config: Option<ScheduleConfig>,
    #[serde(default = "default_true", deserialize_with = "serde_helpers::bool_true")]
    pub is_active: bool,
    pub created_by: Option<Thing>,
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
    pub adjustment_value: i32,
    pub priority: Option<i32>,
    pub is_stackable: Option<bool>,
    pub time_mode: Option<TimeMode>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub schedule_config: Option<ScheduleConfig>,
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
    pub adjustment_value: Option<i32>,
    pub priority: Option<i32>,
    pub is_stackable: Option<bool>,
    pub time_mode: Option<TimeMode>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub schedule_config: Option<ScheduleConfig>,
    pub is_active: Option<bool>,
}
