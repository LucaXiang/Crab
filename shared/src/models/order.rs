//! Order Model

use serde::{Deserialize, Serialize};

/// Order status (archived orders only - no ACTIVE status in SurrealDB)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    #[default]
    Completed,
    Void,
    Moved,
    Merged,
}

/// Order item attribute selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemAttribute {
    /// Attribute reference (String ID)
    pub attr_id: String,
    pub option_idx: i32,
    pub name: String,
    /// Price in currency unit
    pub price: f64,
}

/// Order item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    /// Product specification reference (String ID)
    pub spec: String,
    pub name: String,
    pub spec_name: Option<String>,
    /// Price in currency unit
    pub price: f64,
    pub quantity: i32,
    pub attributes: Vec<OrderItemAttribute>,
    /// Discount amount in currency unit
    pub discount_amount: f64,
    /// Surcharge amount in currency unit
    pub surcharge_amount: f64,
    pub note: Option<String>,
    pub is_sent: bool,
}

/// Order payment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPayment {
    pub method: String,
    /// Amount in currency unit
    pub amount: f64,
    pub time: String,
    pub reference: Option<String>,
}

/// Order entity (archived order in SurrealDB)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: Option<String>,
    pub receipt_number: String,
    pub zone_name: Option<String>,
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
    pub items: Vec<OrderItem>,
    pub payments: Vec<OrderPayment>,
    pub prev_hash: String,
    pub curr_hash: String,
    /// Related order ID (for MOVED/MERGED orders)
    pub related_order_id: Option<String>,
    /// Operator ID who completed/voided the order
    pub operator_id: Option<String>,
    pub created_at: Option<String>,
}

/// Order event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderEventType {
    Created,
    ItemAdded,
    ItemRemoved,
    ItemUpdated,
    Paid,
    PartialPaid,
    Void,
    Refund,
    TableChanged,
    GuestCountChanged,
}

/// Order event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderEvent {
    pub id: Option<String>,
    pub event_type: OrderEventType,
    pub timestamp: String,
    pub data: Option<serde_json::Value>,
    pub prev_hash: String,
    pub curr_hash: String,
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

/// Add item payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAddItem {
    /// Product specification reference (String ID)
    pub spec: String,
    pub name: String,
    pub spec_name: Option<String>,
    /// Price in currency unit
    pub price: f64,
    pub quantity: i32,
    pub attributes: Option<Vec<OrderItemAttribute>>,
    pub note: Option<String>,
}

/// Add payment payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAddPayment {
    pub method: String,
    /// Amount in currency unit
    pub amount: f64,
    pub reference: Option<String>,
}

/// Update totals payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderUpdateTotals {
    /// Total amount in currency unit
    pub total_amount: f64,
    /// Discount amount in currency unit
    pub discount_amount: f64,
    /// Surcharge amount in currency unit
    pub surcharge_amount: f64,
}

/// Update status payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderUpdateStatus {
    pub status: OrderStatus,
}

/// Update hash payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderUpdateHash {
    pub prev_hash: String,
    pub curr_hash: String,
}

/// Add event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAddEvent {
    pub event_type: OrderEventType,
    pub data: Option<serde_json::Value>,
    pub prev_hash: String,
    pub curr_hash: String,
}

/// Remove item payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRemoveItem {
    pub index: usize,
}
