//! Price Rule Model

use serde::{Deserialize, Serialize};

/// Rule type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "db", derive(sqlx::Type))]
#[cfg_attr(feature = "db", sqlx(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum RuleType {
    Discount,
    Surcharge,
}

/// Product scope enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "db", derive(sqlx::Type))]
#[cfg_attr(feature = "db", sqlx(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum ProductScope {
    Global,
    Category,
    Tag,
    Product,
}

/// Adjustment type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "db", derive(sqlx::Type))]
#[cfg_attr(feature = "db", sqlx(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum AdjustmentType {
    Percentage,
    FixedAmount,
}

/// Zone scope constants (no longer prefixed with "zone:")
pub const ZONE_SCOPE_ALL: &str = "all";
pub const ZONE_SCOPE_RETAIL: &str = "retail";

/// Price rule entity (价格调整规则)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct PriceRule {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,
    pub description: Option<String>,
    pub rule_type: RuleType,
    pub product_scope: ProductScope,
    /// Target record ID based on scope (category/tag/product ID)
    pub target_id: Option<i64>,
    /// Zone scope: "all", "retail", or specific zone ID as string
    pub zone_scope: String,
    pub adjustment_type: AdjustmentType,
    /// Adjustment value (percentage: 30=30%, fixed: 5.00=€5)
    pub adjustment_value: f64,
    pub is_stackable: bool,
    pub is_exclusive: bool,
    /// Valid from datetime (Unix millis)
    pub valid_from: Option<i64>,
    /// Valid until datetime (Unix millis)
    pub valid_until: Option<i64>,
    /// Active days of week (JSON array: 0=Sunday..6=Saturday)
    #[cfg_attr(feature = "db", sqlx(json))]
    pub active_days: Option<Vec<u8>>,
    /// Active start time (HH:MM format)
    pub active_start_time: Option<String>,
    /// Active end time (HH:MM format)
    pub active_end_time: Option<String>,
    pub is_active: bool,
    pub created_by: Option<i64>,
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
    pub target_id: Option<i64>,
    pub zone_scope: Option<String>,
    pub adjustment_type: AdjustmentType,
    pub adjustment_value: f64,
    pub is_stackable: Option<bool>,
    pub is_exclusive: Option<bool>,
    pub valid_from: Option<i64>,
    pub valid_until: Option<i64>,
    pub active_days: Option<Vec<u8>>,
    pub active_start_time: Option<String>,
    pub active_end_time: Option<String>,
    pub created_by: Option<i64>,
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
    pub target_id: Option<i64>,
    pub zone_scope: Option<String>,
    pub adjustment_type: Option<AdjustmentType>,
    pub adjustment_value: Option<f64>,
    pub is_stackable: Option<bool>,
    pub is_exclusive: Option<bool>,
    pub valid_from: Option<i64>,
    pub valid_until: Option<i64>,
    pub active_days: Option<Vec<u8>>,
    pub active_start_time: Option<String>,
    pub active_end_time: Option<String>,
    pub is_active: Option<bool>,
}
