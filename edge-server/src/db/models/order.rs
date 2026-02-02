//! Order Model (Graph Model)
//!
//! 归档订单使用图边关系，只存储核心数据。
//! 所有订单变更通过 OrderManager 事件溯源处理。

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

// =============================================================================
// Archived Order Types (used by archive service)
// =============================================================================

/// Order status enum (archived orders only)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    Completed,
    Void,
    Moved,
    Merged,
}

/// Archived order entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<RecordId>,
    pub receipt_number: String,
    pub zone_name: Option<String>,
    pub table_name: Option<String>,
    pub status: OrderStatus,
    pub is_retail: bool,
    pub guest_count: Option<i32>,
    pub original_total: f64,
    pub subtotal: f64,
    pub total_amount: f64,
    pub paid_amount: f64,
    pub discount_amount: f64,
    pub surcharge_amount: f64,
    pub comp_total_amount: f64,
    pub order_manual_discount_amount: f64,
    pub order_manual_surcharge_amount: f64,
    pub tax: f64,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub operator_id: Option<String>,
    pub operator_name: Option<String>,
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub related_order_id: Option<RecordId>,
    // === Void Metadata (only when status == Void) ===
    /// 作废类型: "CANCELLED" | "LOSS_SETTLED"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub void_type: Option<String>,
    /// 损失原因: "CUSTOMER_FLED" | "CUSTOMER_INSOLVENT" | "OTHER"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loss_reason: Option<String>,
    /// 损失金额（未收回部分）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loss_amount: Option<f64>,
    /// 作废备注
    #[serde(skip_serializing_if = "Option::is_none")]
    pub void_note: Option<String>,
    pub prev_hash: String,
    pub curr_hash: String,
    pub created_at: Option<i64>,
}

/// Split item in a payment
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SplitItem {
    pub instance_id: String,
    pub name: String,
    pub quantity: i32,
    #[serde(default)]
    pub unit_price: f64,
}

// =============================================================================
// API Response Types (for frontend)
// =============================================================================

fn default_guest_count() -> i32 {
    1
}

/// Order summary for list view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSummary {
    pub order_id: String,
    pub receipt_number: String,
    pub table_name: Option<String>,
    pub status: String,
    #[serde(default)]
    pub is_retail: bool,
    #[serde(default)]
    pub total: f64,
    #[serde(default = "default_guest_count")]
    pub guest_count: i32,
    #[serde(default)]
    pub start_time: i64,
    pub end_time: Option<i64>,
    // === Void Metadata ===
    #[serde(default)]
    pub void_type: Option<String>,
    #[serde(default)]
    pub loss_reason: Option<String>,
    #[serde(default)]
    pub loss_amount: Option<f64>,
}

/// Order item option for detail view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemOptionDetail {
    pub attribute_name: String,
    pub option_name: String,
    pub price_modifier: f64,
}

/// Order item for detail view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemDetail {
    pub id: String,
    pub instance_id: String,
    pub name: String,
    pub spec_name: Option<String>,
    pub category_name: Option<String>,
    #[serde(default)]
    pub price: f64,
    #[serde(default)]
    pub quantity: i32,
    #[serde(default)]
    pub unpaid_quantity: i32,
    #[serde(default)]
    pub unit_price: f64,
    #[serde(default)]
    pub line_total: f64,
    #[serde(default)]
    pub discount_amount: f64,
    #[serde(default)]
    pub surcharge_amount: f64,
    pub note: Option<String>,
    #[serde(default)]
    pub selected_options: Vec<OrderItemOptionDetail>,
}

/// Payment for detail view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPaymentDetail {
    #[serde(default)]
    pub payment_id: Option<String>,
    pub method: String,
    pub amount: f64,
    pub timestamp: i64,
    #[serde(default)]
    pub cancelled: bool,
    pub cancel_reason: Option<String>,
    #[serde(default)]
    pub split_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_split_items")]
    pub split_items: Vec<SplitItem>,
    #[serde(default)]
    pub aa_shares: Option<i32>,
    #[serde(default)]
    pub aa_total_shares: Option<i32>,
}

/// Deserialize JSON string to Vec<SplitItem>
fn deserialize_split_items<'de, D>(deserializer: D) -> Result<Vec<SplitItem>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) if !s.is_empty() => {
            serde_json::from_str(&s).map_err(serde::de::Error::custom)
        }
        _ => Ok(Vec::new()),
    }
}

/// Event for detail view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderEventDetail {
    pub event_id: String,
    pub event_type: String,
    pub timestamp: i64,
    #[serde(deserialize_with = "deserialize_json_string")]
    pub payload: Option<serde_json::Value>,
}

/// Deserialize JSON string to Value
fn deserialize_json_string<'de, D>(deserializer: D) -> Result<Option<serde_json::Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) if !s.is_empty() => {
            serde_json::from_str(&s).map(Some).map_err(serde::de::Error::custom)
        }
        _ => Ok(Some(serde_json::Value::Object(serde_json::Map::new()))),
    }
}

/// Full order detail (for frontend)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderDetail {
    pub order_id: String,
    pub receipt_number: String,
    pub table_name: Option<String>,
    pub zone_name: Option<String>,
    pub status: String,
    #[serde(default)]
    pub is_retail: bool,
    #[serde(default = "default_guest_count")]
    pub guest_count: i32,
    #[serde(default)]
    pub total: f64,
    #[serde(default)]
    pub paid_amount: f64,
    #[serde(default)]
    pub total_discount: f64,
    #[serde(default)]
    pub total_surcharge: f64,
    #[serde(default)]
    pub comp_total_amount: f64,
    #[serde(default)]
    pub order_manual_discount_amount: f64,
    #[serde(default)]
    pub order_manual_surcharge_amount: f64,
    #[serde(default)]
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub operator_name: Option<String>,
    // === Void Metadata ===
    #[serde(default)]
    pub void_type: Option<String>,
    #[serde(default)]
    pub loss_reason: Option<String>,
    #[serde(default)]
    pub loss_amount: Option<f64>,
    #[serde(default)]
    pub void_note: Option<String>,
    #[serde(default)]
    pub items: Vec<OrderItemDetail>,
    #[serde(default)]
    pub payments: Vec<OrderPaymentDetail>,
    #[serde(default)]
    pub timeline: Vec<OrderEventDetail>,
}
