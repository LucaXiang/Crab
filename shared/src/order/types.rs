//! Shared types for order event sourcing

use serde::{Deserialize, Serialize};

/// Cart item snapshot - complete snapshot for event recording
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CartItemSnapshot {
    /// Product ID
    pub id: String,
    /// Instance ID (content-addressed hash)
    pub instance_id: String,
    /// Product name
    pub name: String,
    /// Final price after discounts
    pub price: f64,
    /// Original price before discounts
    pub original_price: Option<f64>,
    /// Quantity
    pub quantity: i32,
    /// Unpaid quantity (computed: quantity - paid_quantity)
    #[serde(default)]
    pub unpaid_quantity: i32,
    /// Selected options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_options: Option<Vec<ItemOption>>,
    /// Selected specification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_specification: Option<SpecificationInfo>,
    /// Discount percentage (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_percent: Option<f64>,
    /// Surcharge amount
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surcharge: Option<f64>,
    /// Item note
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Authorizer ID (for discounts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizer_id: Option<String>,
    /// Authorizer name snapshot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizer_name: Option<String>,
}

/// Cart item input - for adding items (without instance_id)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartItemInput {
    /// Product ID
    pub product_id: String,
    /// Product name
    pub name: String,
    /// Price
    pub price: f64,
    /// Original price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_price: Option<f64>,
    /// Quantity
    pub quantity: i32,
    /// Selected options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_options: Option<Vec<ItemOption>>,
    /// Selected specification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_specification: Option<SpecificationInfo>,
    /// Discount percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_percent: Option<f64>,
    /// Surcharge amount
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surcharge: Option<f64>,
    /// Item note
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Authorizer ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizer_id: Option<String>,
    /// Authorizer name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizer_name: Option<String>,
}

/// Item option selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemOption {
    pub attribute_id: String,
    pub attribute_name: String,
    pub option_idx: i32,
    pub option_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_modifier: Option<f64>,
}

/// Specification info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpecificationInfo {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
}

/// Item changes for modification
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ItemChanges {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surcharge: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Split item for split bill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitItem {
    pub instance_id: String,
    pub name: String,
    pub quantity: i32,
}

/// Payment input for adding payment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInput {
    pub method: String,
    pub amount: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tendered: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Payment record in snapshot
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentRecord {
    pub payment_id: String,
    pub method: String,
    pub amount: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tendered: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub timestamp: i64,
    #[serde(default)]
    pub cancelled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_reason: Option<String>,
}

/// Payment summary for completed order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentSummaryItem {
    pub method: String,
    pub amount: f64,
}

/// Command response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    /// The command ID this responds to
    pub command_id: String,
    /// Whether the command succeeded
    pub success: bool,
    /// New order ID (only for OpenTable command)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// Error details if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CommandError>,
}

impl CommandResponse {
    pub fn success(command_id: String, order_id: Option<String>) -> Self {
        Self {
            command_id,
            success: true,
            order_id,
            error: None,
        }
    }

    pub fn error(command_id: String, error: CommandError) -> Self {
        Self {
            command_id,
            success: false,
            order_id: None,
            error: Some(error),
        }
    }

    pub fn duplicate(command_id: String) -> Self {
        Self {
            command_id,
            success: true,
            order_id: None,
            error: None,
        }
    }
}

/// Command error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandError {
    pub code: CommandErrorCode,
    pub message: String,
}

impl CommandError {
    pub fn new(code: CommandErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Command error codes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommandErrorCode {
    OrderNotFound,
    OrderAlreadyCompleted,
    OrderAlreadyVoided,
    ItemNotFound,
    PaymentNotFound,
    InsufficientQuantity,
    InvalidAmount,
    InvalidOperation,
    DuplicateCommand,
    InternalError,
}

/// Sync request for reconnection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    /// Client's last known sequence number
    pub since_sequence: u64,
}

/// Sync response for reconnection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    /// Events since the requested sequence
    pub events: Vec<super::event::OrderEvent>,
    /// Current active order snapshots
    pub active_orders: Vec<super::snapshot::OrderSnapshot>,
    /// Server's current sequence number
    pub server_sequence: u64,
    /// Whether full sync is required (gap too large)
    pub requires_full_sync: bool,
}

/// Item modification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemModificationResult {
    pub instance_id: String,
    pub quantity: i32,
    pub price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_percent: Option<f64>,
    /// Action taken: "UNCHANGED", "CREATED", "UPDATED"
    pub action: String,
}
