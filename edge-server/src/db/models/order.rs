//! Order Model (Graph Model)
//!
//! 归档订单使用图边关系，只存储核心数据

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

// =============================================================================
// Order (主表)
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
    pub tax: f64,
    pub start_time: String,
    pub end_time: Option<String>,
    pub operator_id: Option<String>,
    pub operator_name: Option<String>,
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub related_order_id: Option<RecordId>,
    pub prev_hash: String,
    pub curr_hash: String,
    pub created_at: Option<String>,
}

// =============================================================================
// Order Item (图边: has_item)
// =============================================================================

/// Archived order item (connected via has_item edge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<RecordId>,
    pub spec: String,
    pub instance_id: String,
    pub name: String,
    pub spec_name: Option<String>,
    pub price: f64,
    pub quantity: i32,
    pub unpaid_quantity: i32,
    pub unit_price: f64,
    pub line_total: f64,
    pub discount_amount: f64,
    pub surcharge_amount: f64,
    pub tax: f64,
    pub note: Option<String>,
}

/// Order item option (connected via has_option edge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemOption {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<RecordId>,
    pub attribute_name: String,
    pub option_name: String,
    pub price: f64,
}

// =============================================================================
// Order Payment (图边: has_payment)
// =============================================================================

/// Split item in a payment
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SplitItem {
    pub instance_id: String,
    pub name: String,
    pub quantity: i32,
    #[serde(default)]
    pub unit_price: f64,
}

/// Archived payment record (connected via has_payment edge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPayment {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<RecordId>,
    pub method: String,
    pub amount: f64,
    pub time: String,
    pub reference: Option<String>,
    #[serde(default)]
    pub cancelled: bool,
    pub cancel_reason: Option<String>,
    #[serde(default)]
    pub split_items: Vec<SplitItem>,
}

// =============================================================================
// Order Event (图边: has_event)
// =============================================================================

/// Order event types (matches shared::order::OrderEventType)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderEventType {
    // Lifecycle
    TableOpened,
    OrderCompleted,
    OrderVoided,
    OrderRestored,
    // Items
    ItemsAdded,
    ItemModified,
    ItemRemoved,
    ItemRestored,
    // Payments
    PaymentAdded,
    PaymentCancelled,
    // Split
    OrderSplit,
    // Table operations
    OrderMoved,
    OrderMovedOut,
    OrderMerged,
    OrderMergedOut,
    TableReassigned,
    // Other
    OrderInfoUpdated,
    // Price Rules
    RuleSkipToggled,
}

/// Order event entity (connected via has_event edge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderEvent {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<RecordId>,
    pub event_type: OrderEventType,
    pub timestamp: String,
    pub data: Option<serde_json::Value>,
    pub prev_hash: String,
    pub curr_hash: String,
}

/// Has event edge relation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasEvent {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<RecordId>,
    #[serde(rename = "in", with = "serde_helpers::record_id")]
    pub from: RecordId,
    #[serde(with = "serde_helpers::record_id")]
    pub out: RecordId,
}

// =============================================================================
// API Request Types
// =============================================================================

/// Create order payload (for active orders, not archived)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCreate {
    pub receipt_number: String,
    pub zone_name: Option<String>,
    pub table_name: Option<String>,
    pub guest_count: Option<i32>,
    pub prev_hash: String,
}

/// Embedded attribute selection (for active order items)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrderItemAttribute {
    #[serde(default)]
    pub attr_id: String,
    #[serde(default)]
    pub option_idx: i32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub price: f64,
}

/// Add item to order payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAddItem {
    pub spec: String,
    pub name: String,
    pub spec_name: Option<String>,
    pub price: f64,
    pub quantity: i32,
    pub attributes: Option<Vec<OrderItemAttribute>>,
    pub note: Option<String>,
}

/// Add payment payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAddPayment {
    pub method: String,
    pub amount: f64,
    pub reference: Option<String>,
}

// =============================================================================
// API Response Types (for frontend)
// =============================================================================

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
}

fn default_guest_count() -> i32 {
    1
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
    pub method: String,
    pub amount: f64,
    pub timestamp: i64,
    pub note: Option<String>,
    #[serde(default)]
    pub cancelled: bool,
    pub cancel_reason: Option<String>,
    #[serde(default, deserialize_with = "deserialize_split_items")]
    pub split_items: Vec<SplitItem>,
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
    use serde::Deserialize;
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) if !s.is_empty() => {
            serde_json::from_str(&s).map(Some).map_err(serde::de::Error::custom)
        }
        _ => Ok(Some(serde_json::Value::Object(serde_json::Map::new()))),
    }
}

/// Deserialize JSON string to Vec<SplitItem>
fn deserialize_split_items<'de, D>(deserializer: D) -> Result<Vec<SplitItem>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) if !s.is_empty() => {
            serde_json::from_str(&s).map_err(serde::de::Error::custom)
        }
        _ => Ok(Vec::new()),
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
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub operator_name: Option<String>,
    #[serde(default)]
    pub items: Vec<OrderItemDetail>,
    #[serde(default)]
    pub payments: Vec<OrderPaymentDetail>,
    #[serde(default)]
    pub timeline: Vec<OrderEventDetail>,
}
