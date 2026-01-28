//! Order events - immutable facts recorded after command processing

use super::types::{
    CartItemSnapshot, ItemChanges, ItemModificationResult, LossReason, PaymentSummaryItem,
    SplitItem, VoidType,
};
use serde::{Deserialize, Serialize};

/// Order event - immutable audit record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderEvent {
    /// Event unique ID
    pub event_id: String,
    /// Global sequence number (for ordering and replay)
    /// This is the AUTHORITATIVE ordering mechanism for state evolution
    pub sequence: u64,
    /// Order this event belongs to
    pub order_id: String,
    /// Server timestamp (Unix milliseconds) - AUTHORITATIVE for state evolution
    /// Always set by server when event is created
    pub timestamp: i64,
    /// Client timestamp (Unix milliseconds) - for audit and debugging
    /// Preserved from original command, may differ from server time due to clock skew
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<i64>,
    /// Operator who triggered this event
    pub operator_id: String,
    /// Operator name (snapshot for audit)
    pub operator_name: String,
    /// Command that triggered this event (for audit tracing)
    pub command_id: String,
    /// Event type
    pub event_type: OrderEventType,
    /// Event payload
    pub payload: EventPayload,
}

/// Event type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

impl std::fmt::Display for OrderEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderEventType::TableOpened => write!(f, "TABLE_OPENED"),
            OrderEventType::OrderCompleted => write!(f, "ORDER_COMPLETED"),
            OrderEventType::OrderVoided => write!(f, "ORDER_VOIDED"),
            OrderEventType::OrderRestored => write!(f, "ORDER_RESTORED"),
            OrderEventType::ItemsAdded => write!(f, "ITEMS_ADDED"),
            OrderEventType::ItemModified => write!(f, "ITEM_MODIFIED"),
            OrderEventType::ItemRemoved => write!(f, "ITEM_REMOVED"),
            OrderEventType::ItemRestored => write!(f, "ITEM_RESTORED"),
            OrderEventType::PaymentAdded => write!(f, "PAYMENT_ADDED"),
            OrderEventType::PaymentCancelled => write!(f, "PAYMENT_CANCELLED"),
            OrderEventType::OrderSplit => write!(f, "ORDER_SPLIT"),
            OrderEventType::OrderMoved => write!(f, "ORDER_MOVED"),
            OrderEventType::OrderMovedOut => write!(f, "ORDER_MOVED_OUT"),
            OrderEventType::OrderMerged => write!(f, "ORDER_MERGED"),
            OrderEventType::OrderMergedOut => write!(f, "ORDER_MERGED_OUT"),
            OrderEventType::TableReassigned => write!(f, "TABLE_REASSIGNED"),
            OrderEventType::OrderInfoUpdated => write!(f, "ORDER_INFO_UPDATED"),
            OrderEventType::RuleSkipToggled => write!(f, "RULE_SKIP_TOGGLED"),
        }
    }
}

/// Event payload variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventPayload {
    // ========== Lifecycle ==========
    TableOpened {
        #[serde(skip_serializing_if = "Option::is_none")]
        table_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        table_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        zone_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        zone_name: Option<String>,
        guest_count: i32,
        is_retail: bool,
        /// Server-generated receipt number (always present)
        receipt_number: String,
    },

    OrderCompleted {
        receipt_number: String,
        final_total: f64,
        payment_summary: Vec<PaymentSummaryItem>,
    },

    OrderVoided {
        /// 作废类型（默认 Cancelled）
        #[serde(default)]
        void_type: VoidType,
        /// 损失原因（仅 LossSettled 时使用）
        #[serde(skip_serializing_if = "Option::is_none")]
        loss_reason: Option<LossReason>,
        /// 损失金额（仅 LossSettled 时使用，用于报税）
        #[serde(skip_serializing_if = "Option::is_none")]
        loss_amount: Option<f64>,
        /// 备注
        #[serde(skip_serializing_if = "Option::is_none")]
        note: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        authorizer_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        authorizer_name: Option<String>,
    },

    OrderRestored {},

    // ========== Items ==========
    ItemsAdded {
        /// Complete snapshots of added items
        items: Vec<CartItemSnapshot>,
    },

    ItemModified {
        /// Operation description for audit
        operation: String,
        /// Source item before modification
        source: Box<CartItemSnapshot>,
        /// Number of items affected
        affected_quantity: i32,
        /// Changes applied
        changes: Box<ItemChanges>,
        /// Previous values for comparison
        previous_values: Box<ItemChanges>,
        /// Resulting items after modification
        results: Vec<ItemModificationResult>,
        /// Authorizer info
        #[serde(skip_serializing_if = "Option::is_none")]
        authorizer_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        authorizer_name: Option<String>,
    },

    ItemRemoved {
        instance_id: String,
        item_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        quantity: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        authorizer_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        authorizer_name: Option<String>,
    },

    ItemRestored {
        instance_id: String,
        item_name: String,
    },

    // ========== Payments ==========
    PaymentAdded {
        payment_id: String,
        method: String,
        amount: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        tendered: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        change: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        note: Option<String>,
    },

    PaymentCancelled {
        payment_id: String,
        method: String,
        amount: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        authorizer_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        authorizer_name: Option<String>,
    },

    // ========== Split ==========
    OrderSplit {
        split_amount: f64,
        payment_method: String,
        items: Vec<SplitItem>,
    },

    // ========== Table Operations ==========
    OrderMoved {
        source_table_id: String,
        source_table_name: String,
        target_table_id: String,
        target_table_name: String,
        items: Vec<CartItemSnapshot>,
    },

    OrderMovedOut {
        target_table_id: String,
        target_table_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },

    OrderMerged {
        source_table_id: String,
        source_table_name: String,
        items: Vec<CartItemSnapshot>,
    },

    OrderMergedOut {
        target_table_id: String,
        target_table_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },

    TableReassigned {
        source_table_id: String,
        source_table_name: String,
        target_table_id: String,
        target_table_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        target_zone_name: Option<String>,
        original_start_time: i64,
        items: Vec<CartItemSnapshot>,
    },

    // ========== Other ==========
    /// Order info updated (receipt_number is immutable - set at OpenTable)
    OrderInfoUpdated {
        #[serde(skip_serializing_if = "Option::is_none")]
        guest_count: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        table_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_pre_payment: Option<bool>,
    },

    // ========== Price Rules ==========
    RuleSkipToggled {
        rule_id: String,
        skipped: bool,
        /// Recalculated amounts after toggle
        subtotal: f64,
        discount: f64,
        surcharge: f64,
        total: f64,
    },
}

impl OrderEvent {
    /// Create a new event
    ///
    /// # Arguments
    /// * `sequence` - Global sequence number (authoritative ordering)
    /// * `order_id` - Order this event belongs to
    /// * `operator_id` - Operator who triggered this event
    /// * `operator_name` - Operator name (snapshot for audit)
    /// * `command_id` - Command that triggered this event
    /// * `client_timestamp` - Client-provided timestamp (for audit, may have clock skew)
    /// * `event_type` - Event type
    /// * `payload` - Event payload
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sequence: u64,
        order_id: String,
        operator_id: String,
        operator_name: String,
        command_id: String,
        client_timestamp: Option<i64>,
        event_type: OrderEventType,
        payload: EventPayload,
    ) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            sequence,
            order_id,
            // Server timestamp is ALWAYS set by server - this is authoritative
            timestamp: chrono::Utc::now().timestamp_millis(),
            // Client timestamp preserved for audit (may differ due to clock skew)
            client_timestamp,
            operator_id,
            operator_name,
            command_id,
            event_type,
            payload,
        }
    }

    /// Create event from command (extracts metadata including client timestamp)
    pub fn from_command(
        sequence: u64,
        order_id: String,
        command: &super::OrderCommand,
        event_type: OrderEventType,
        payload: EventPayload,
    ) -> Self {
        Self::new(
            sequence,
            order_id,
            command.operator_id.clone(),
            command.operator_name.clone(),
            command.command_id.clone(),
            Some(command.timestamp), // Preserve client timestamp
            event_type,
            payload,
        )
    }
}
