//! Applied MG Rule - tracks MG discount applied to an order item

use crate::models::price_rule::{AdjustmentType, ProductScope};
use serde::{Deserialize, Serialize};

/// Applied MG rule record
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppliedMgRule {
    pub rule_id: i64,
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,
    pub product_scope: ProductScope,
    pub adjustment_type: AdjustmentType,
    pub adjustment_value: f64,
    pub calculated_amount: f64,
    pub skipped: bool,
}
