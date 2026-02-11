//! Marketing Group & MG Discount Rule Models

use serde::{Deserialize, Serialize};

use super::price_rule::{AdjustmentType, ProductScope};

/// Marketing Group entity (营销组 = 会员等级)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct MarketingGroup {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub points_earn_rate: Option<f64>,
    pub points_per_unit: Option<f64>,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Create marketing group payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketingGroupCreate {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
    pub points_earn_rate: Option<f64>,
    pub points_per_unit: Option<f64>,
}

/// Update marketing group payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketingGroupUpdate {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
    pub points_earn_rate: Option<f64>,
    pub points_per_unit: Option<f64>,
    pub is_active: Option<bool>,
}

/// MG Discount Rule entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct MgDiscountRule {
    pub id: i64,
    pub marketing_group_id: i64,
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,
    pub product_scope: ProductScope,
    pub target_id: Option<i64>,
    pub adjustment_type: AdjustmentType,
    pub adjustment_value: f64,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Create MG discount rule payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MgDiscountRuleCreate {
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,
    pub product_scope: ProductScope,
    pub target_id: Option<i64>,
    pub adjustment_type: AdjustmentType,
    pub adjustment_value: f64,
}

/// Update MG discount rule payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MgDiscountRuleUpdate {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub receipt_name: Option<String>,
    pub product_scope: Option<ProductScope>,
    pub target_id: Option<i64>,
    pub adjustment_type: Option<AdjustmentType>,
    pub adjustment_value: Option<f64>,
    pub is_active: Option<bool>,
}
