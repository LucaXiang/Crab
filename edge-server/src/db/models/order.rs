//! Order Model (Graph/Document hybrid)

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

/// Order status enum (archived orders only)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    Completed,
    Void,
    Moved,
    Merged,
}

/// Embedded order item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    /// Product specification reference
    #[serde(with = "serde_helpers::record_id")]
    pub spec: RecordId,
    /// Snapshot: product name
    pub name: String,
    /// Snapshot: specification name
    pub spec_name: Option<String>,
    /// Price in currency unit (e.g., 10.50 = ¥10.50)
    pub price: f64,
    pub quantity: i32,
    /// Selected attributes: [{ attr_id, option_idx, name, price }]
    pub attributes: Vec<OrderItemAttribute>,
    /// Item-level discount amount in currency unit
    pub discount_amount: f64,
    /// Item-level surcharge amount in currency unit
    pub surcharge_amount: f64,
    /// Notes
    pub note: Option<String>,
    /// Sent to kitchen
    pub is_sent: bool,
}

/// Embedded attribute selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemAttribute {
    #[serde(with = "serde_helpers::record_id")]
    pub attr_id: RecordId,
    pub option_idx: i32,
    pub name: String,
    /// Price in currency unit (e.g., 2.50 = ¥2.50)
    pub price: f64,
}

/// Embedded payment record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPayment {
    pub method: String,
    /// Amount in currency unit (e.g., 50.00 = ¥50.00)
    pub amount: f64,
    pub time: String,
    pub reference: Option<String>,
}

/// Order entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<RecordId>,
    pub receipt_number: String,
    /// Zone name snapshot
    pub zone_name: Option<String>,
    /// Table name snapshot
    pub table_name: Option<String>,
    pub status: OrderStatus,
    pub start_time: String,
    pub end_time: Option<String>,
    pub guest_count: Option<i32>,
    /// Total amount in currency unit
    pub total_amount: f64,
    /// Paid amount in currency unit
    pub paid_amount: f64,
    /// Discount amount in currency unit
    pub discount_amount: f64,
    /// Surcharge amount in currency unit
    pub surcharge_amount: f64,
    /// Embedded order items
    pub items: Vec<OrderItem>,
    /// Embedded payments
    pub payments: Vec<OrderPayment>,
    /// Hash chain: previous hash
    pub prev_hash: String,
    /// Hash chain: current hash
    pub curr_hash: String,
    /// Related order (for MOVED/MERGED)
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub related_order_id: Option<RecordId>,
    /// Operator ID who completed/voided the order
    pub operator_id: Option<String>,
    pub created_at: Option<String>,
}

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

/// Create order payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCreate {
    pub receipt_number: String,
    pub zone_name: Option<String>,
    pub table_name: Option<String>,
    pub guest_count: Option<i32>,
    pub prev_hash: String,
}

/// Add item to order payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAddItem {
    #[serde(with = "serde_helpers::record_id")]
    pub spec: RecordId,
    pub name: String,
    pub spec_name: Option<String>,
    /// Price in currency unit (e.g., 10.50 = ¥10.50)
    pub price: f64,
    pub quantity: i32,
    pub attributes: Option<Vec<OrderItemAttribute>>,
    pub note: Option<String>,
}

/// Add payment payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAddPayment {
    pub method: String,
    /// Amount in currency unit (e.g., 50.00 = ¥50.00)
    pub amount: f64,
    pub reference: Option<String>,
}
