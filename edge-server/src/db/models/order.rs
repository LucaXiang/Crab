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

/// Embedded order item (snapshot - all data copied as strings)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrderItem {
    #[serde(default)]
    pub spec: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub spec_name: Option<String>,
    #[serde(default)]
    pub price: f64,
    #[serde(default)]
    pub quantity: i32,
    #[serde(default)]
    pub attributes: Vec<OrderItemAttribute>,
    #[serde(default)]
    pub discount_amount: f64,
    #[serde(default)]
    pub surcharge_amount: f64,
    #[serde(default)]
    pub unit_price: f64,
    #[serde(default)]
    pub line_total: f64,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub is_sent: bool,
}

/// Embedded attribute selection (snapshot - strings only)
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

/// Embedded payment record
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrderPayment {
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub amount: f64,
    #[serde(default)]
    pub time: String,
    #[serde(default)]
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
    /// Full OrderSnapshot JSON (for detail queries)
    pub snapshot_json: String,
    /// Order items as JSON string (deprecated, kept for compatibility)
    #[serde(default)]
    pub items_json: String,
    /// Payments as JSON string (deprecated, kept for compatibility)
    #[serde(default)]
    pub payments_json: String,
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

impl Order {
    /// Parse items from JSON string
    pub fn items(&self) -> Vec<OrderItem> {
        serde_json::from_str(&self.items_json).unwrap_or_default()
    }

    /// Parse payments from JSON string
    pub fn payments(&self) -> Vec<OrderPayment> {
        serde_json::from_str(&self.payments_json).unwrap_or_default()
    }
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
    /// Product spec ID (string snapshot)
    pub spec: String,
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
