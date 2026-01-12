use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Payload for OrderIntent (Client -> Server)
/// 客户端发送订单操作意图（点菜、结算、支付等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderIntentPayload {
    /// Action name (e.g., "add_dish", "payment", "checkout")
    pub action: String,
    /// Table ID associated with the order
    pub table_id: String,
    /// Order ID (optional, if creating new order)
    pub order_id: Option<String>,
    /// Data payload specific to the action
    pub data: Value,
    /// Operator ID
    pub operator: Option<String>,
}

/// Payload for OrderSync (Server -> Client)
/// 服务端广播订单状态变更
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSyncPayload {
    /// Action that caused the sync
    pub action: String,
    /// Table ID
    pub table_id: String,
    /// Order ID
    pub order_id: Option<String>,
    /// Status (e.g., "updated", "completed", "error")
    pub status: String,
    /// Source of the change
    pub source: String,
    /// Updated order data or diff
    pub data: Option<Value>,
}

/// Payload for Notification (Server -> Client)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    pub title: String,
    pub message: String,
    pub level: String, // info, warn, error
    pub data: Option<Value>,
}
