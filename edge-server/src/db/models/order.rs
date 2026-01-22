//! Order Model (Graph/Document hybrid)

use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// Order status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    #[default]
    Open,
    Paid,
    Void,
}

/// Embedded order item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    /// Product specification reference
    pub spec: Thing,
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
    pub attr_id: Thing,
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
    pub id: Option<Thing>,
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

/// Order event entity (connected via has_event edge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderEvent {
    pub id: Option<Thing>,
    pub event_type: OrderEventType,
    pub timestamp: String,
    pub data: Option<serde_json::Value>,
    pub prev_hash: String,
    pub curr_hash: String,
}

/// Has event edge relation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasEvent {
    pub id: Option<Thing>,
    #[serde(rename = "in")]
    pub from: Thing,
    pub out: Thing,
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
    pub spec: Thing,
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
