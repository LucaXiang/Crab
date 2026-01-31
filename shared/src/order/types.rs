//! Shared types for order event sourcing

use super::AppliedRule;
use serde::{Deserialize, Serialize};

// ============================================================================
// Void Types
// ============================================================================

/// 作废类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VoidType {
    /// 取消订单 - 未付款，直接取消
    #[default]
    Cancelled,
    /// 损失结算 - 已付部分，剩余计入损失（用于报税）
    LossSettled,
}

/// 损失原因（预设选项）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LossReason {
    /// 客人逃单
    CustomerFled,
    /// 客人无力支付
    CustomerInsolvent,
    /// 其他
    Other,
}

// ============================================================================
// Service Type
// ============================================================================

/// 服务类型（零售订单使用）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ServiceType {
    /// 堂食
    #[default]
    DineIn,
    /// 外卖/打包
    Takeout,
}

// ============================================================================
// Cart Item Types
// ============================================================================

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

    // === Manual Adjustment ===
    /// Manual discount percentage (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_discount_percent: Option<f64>,
    /// Surcharge amount
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surcharge: Option<f64>,

    // === Rule Adjustments ===
    /// Rule discount amount (calculated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_discount_amount: Option<f64>,
    /// Rule surcharge amount (calculated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_surcharge_amount: Option<f64>,
    /// Applied rules list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_rules: Option<Vec<AppliedRule>>,

    // === Computed Fields ===
    /// Unit price for display (computed by backend: price with manual discount and surcharge)
    /// This is the final per-unit price shown to customers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_price: Option<f64>,
    /// Line total (computed by backend: unit_price * quantity)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_total: Option<f64>,
    /// Tax amount for this item
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax: Option<f64>,
    /// Tax rate for this item (e.g., 21 for 21% IVA)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_rate: Option<i32>,

    /// Item note
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Authorizer ID (for discounts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizer_id: Option<String>,
    /// Authorizer name snapshot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizer_name: Option<String>,
    /// Category name snapshot (for statistics)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_name: Option<String>,
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
    /// Manual discount percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_discount_percent: Option<f64>,
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
    pub external_id: Option<i64>,
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
    /// Manual discount percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_discount_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surcharge: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Selected options (None = no change, Some(vec) = replace options)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_options: Option<Vec<ItemOption>>,
    /// Selected specification (None = no change, Some(spec) = replace specification)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_specification: Option<SpecificationInfo>,
}

/// Split item for split bill
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SplitItem {
    #[serde(default)]
    pub instance_id: String,
    /// Item name (for display/audit)
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub quantity: i32,
    /// Unit price (for display/audit, not used in calculation)
    #[serde(default)]
    pub unit_price: f64,
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

/// Split type for categorizing split payments
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SplitType {
    ItemSplit,
    AmountSplit,
    AaSplit,
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
    /// Split payment items snapshot (for restoration on cancel)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split_items: Option<Vec<CartItemSnapshot>>,
    /// AA split: number of shares this payment covers (for rollback on cancel)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aa_shares: Option<i32>,
    /// Split type: which split mode produced this payment
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split_type: Option<SplitType>,
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
    TableOccupied,
    // Storage errors (maps to ErrorCode 94xx)
    StorageFull,
    OutOfMemory,
    StorageCorrupted,
    SystemBusy,
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
    pub manual_discount_percent: Option<f64>,
    /// Action taken: "UNCHANGED", "CREATED", "UPDATED"
    pub action: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cart_item_snapshot_rule_fields() {
        let item = CartItemSnapshot {
            id: "prod-1".to_string(),
            instance_id: "inst-1".to_string(),
            name: "Test".to_string(),
            price: 100.0,
            original_price: Some(120.0),
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: Some(10.0),
            surcharge: None,
            rule_discount_amount: Some(5.0),
            rule_surcharge_amount: Some(3.0),
            applied_rules: Some(vec![]),
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
        };

        assert_eq!(item.manual_discount_percent, Some(10.0));
        assert_eq!(item.rule_discount_amount, Some(5.0));
    }
}
