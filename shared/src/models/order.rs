//! Order Model

use serde::{Deserialize, Serialize};

/// Order status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    #[default]
    Open,
    Paid,
    Void,
}

/// Order item attribute selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemAttribute {
    /// Attribute reference (String ID)
    pub attr_id: String,
    pub option_idx: i32,
    pub name: String,
    pub price: i32,
}

/// Order item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    /// Product specification reference (String ID)
    pub spec: String,
    pub name: String,
    pub spec_name: Option<String>,
    pub price: i32,
    pub quantity: i32,
    pub attributes: Vec<OrderItemAttribute>,
    pub discount_amount: i32,
    pub surcharge_amount: i32,
    pub note: Option<String>,
    pub is_sent: bool,
}

/// Order payment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPayment {
    pub method: String,
    pub amount: i32,
    pub time: String,
    pub reference: Option<String>,
}

/// Order entity
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
    pub total_amount: i32,
    pub paid_amount: i32,
    pub discount_amount: i32,
    pub surcharge_amount: i32,
    pub items: Vec<OrderItem>,
    pub payments: Vec<OrderPayment>,
    pub prev_hash: String,
    pub curr_hash: String,
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
    pub price: i32,
    pub quantity: i32,
    pub attributes: Option<Vec<OrderItemAttribute>>,
    pub note: Option<String>,
}

/// Add payment payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAddPayment {
    pub method: String,
    pub amount: i32,
    pub reference: Option<String>,
}

/// Update totals payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderUpdateTotals {
    pub total_amount: i32,
    pub discount_amount: i32,
    pub surcharge_amount: i32,
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
