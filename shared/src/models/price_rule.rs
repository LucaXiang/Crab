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

/// Zone scope constants
pub const ZONE_SCOPE_ALL: &str = "zone:all";
pub const ZONE_SCOPE_RETAIL: &str = "zone:retail";

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
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub is_stackable: bool,
    /// Whether this rule is exclusive (cannot be combined with other rules)
    #[serde(default)]
    pub is_exclusive: bool,
    /// Valid from datetime (Unix millis)
    pub valid_from: Option<i64>,
    /// Valid until datetime (Unix millis)
    pub valid_until: Option<i64>,
    /// Active days of week (0=Sunday, 1=Monday, ..., 6=Saturday)
    pub active_days: Option<Vec<u8>>,
    /// Active start time (HH:MM format)
    pub active_start_time: Option<String>,
    /// Active end time (HH:MM format)
    pub active_end_time: Option<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
    pub created_by: Option<String>,
    /// Created datetime (Unix millis)
    #[serde(default = "default_created_at")]
    pub created_at: i64,
}

fn default_created_at() -> i64 {
    0
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
    /// Valid from datetime (Unix millis)
    pub valid_from: Option<i64>,
    /// Valid until datetime (Unix millis)
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
    /// Valid from datetime (Unix millis)
    pub valid_from: Option<i64>,
    /// Valid until datetime (Unix millis)
    pub valid_until: Option<i64>,
    /// Active days of week (0=Sunday, 1=Monday, ..., 6=Saturday)
    pub active_days: Option<Vec<u8>>,
    /// Active start time (HH:MM format)
    pub active_start_time: Option<String>,
    /// Active end time (HH:MM format)
    pub active_end_time: Option<String>,
    pub is_active: Option<bool>,
}
