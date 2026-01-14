//! Message types for the message bus
//!
//! These types are shared between edge-server and clients for both
//! in-process (memory) and network (TCP) communication.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod payload;
pub use payload::*;

/// Event types for bus messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    /// Order intent - client requests to server
    /// 订单意图（客户端 → 边缘服务端）：点菜请求、付款请求、结账请求等
    OrderIntent = 1,

    /// Order synchronization - server broadcasts to all clients
    /// 订单同步（边缘服务端 → 所有客户端）：订单状态变更广播
    OrderSync = 2,

    /// Data synchronization - base data updates
    /// 数据同步（边缘服务端 → 所有客户端）：菜品原型数据变更
    DataSync = 3,

    /// System notifications
    /// 系统通知（边缘服务端 → 所有客户端）：打印机状态、网络异常等
    Notification = 4,

    /// Server commands - from upstream/central server to edge server
    /// 服务器指令（上层服务器 → 边缘服务端）：配置更新、数据同步指令、远程控制等
    ServerCommand = 5,
}

impl TryFrom<u8> for EventType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(EventType::OrderIntent),
            2 => Ok(EventType::OrderSync),
            3 => Ok(EventType::DataSync),
            4 => Ok(EventType::Notification),
            5 => Ok(EventType::ServerCommand),
            _ => Err(()),
        }
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventType::OrderIntent => write!(f, "order_intent"),
            EventType::OrderSync => write!(f, "order_sync"),
            EventType::DataSync => write!(f, "data_sync"),
            EventType::Notification => write!(f, "notification"),
            EventType::ServerCommand => write!(f, "server_command"),
        }
    }
}

/// Binary message for the message bus
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BusMessage {
    /// Event type identifier
    pub event_type: EventType,
    /// Binary payload
    pub payload: Vec<u8>,
}

impl BusMessage {
    /// Create a new bus message
    pub fn new(event_type: EventType, payload: Vec<u8>) -> Self {
        Self {
            event_type,
            payload,
        }
    }

    /// Create a order intent message (client → server)
    ///
    /// 客户端发送操作意图给服务端，服务端处理后广播 OrderSync
    ///
    /// # Examples
    ///
    /// ```rust
    /// use shared::message::{BusMessage, OrderIntentPayload};
    /// use serde_json::json;
    ///
    /// let payload = OrderIntentPayload {
    ///     action: "add_dish".to_string(),
    ///     table_id: "T01".to_string(),
    ///     order_id: None,
    ///     data: json!({}),
    ///     operator: Some("waiter_001".to_string()),
    /// };
    /// BusMessage::order_intent(&payload);
    /// ```
    pub fn server_command(payload: &ServerCommandPayload) -> Self {
        let payload_bytes = serde_json::to_vec(payload).expect("Failed to serialize ServerCommand");
        Self::new(EventType::ServerCommand, payload_bytes)
    }

    pub fn order_intent<T: serde::Serialize>(payload: &T) -> Self {
        Self::new(
            EventType::OrderIntent,
            serde_json::to_vec(payload).expect("Failed to serialize order intent"),
        )
    }

    /// Create an order sync message (server → client)
    pub fn order_sync<T: serde::Serialize>(payload: &T) -> Self {
        Self::new(
            EventType::OrderSync,
            serde_json::to_vec(payload).expect("Failed to serialize order sync"),
        )
    }

    /// Create a data sync message (for base data updates)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use shared::message::BusMessage;
    /// use serde_json::json;
    ///
    /// // 菜品价格更新
    /// BusMessage::data_sync("dish_price", json!({
    ///     "dish_id": "D001",
    ///     "old_price": 3800,
    ///     "new_price": 4200
    /// }));
    ///
    /// // 菜品沽清
    /// BusMessage::data_sync("dish_sold_out", json!({
    ///     "dish_id": "D001",
    ///     "available": false
    /// }));
    ///
    /// // 新菜品上架
    /// BusMessage::data_sync("dish_added", json!({
    ///     "dish_id": "D999",
    ///     "name": "新品推荐",
    ///     "price": 5800,
    ///     "category": "hot"
    /// }));
    /// ```
    pub fn data_sync(sync_type: &str, data: serde_json::Value) -> Self {
        let payload = serde_json::json!({
            "sync_type": sync_type,
            "data": data,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        Self::new(EventType::DataSync, serde_json::to_vec(&payload).unwrap())
    }

    /// Create a notification message
    ///
    /// # Examples
    ///
    /// ```rust
    /// use shared::message::BusMessage;
    ///
    /// // 系统通知
    /// BusMessage::notification("打印机缺纸", "请及时添加打印纸");
    ///
    /// // 网络异常
    /// BusMessage::notification("网络异常", "与服务器连接中断");
    /// ```
    pub fn notification(title: &str, body: &str) -> Self {
        let payload = serde_json::json!({
            "title": title,
            "body": body,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        Self::new(
            EventType::Notification,
            serde_json::to_vec(&payload).unwrap(),
        )
    }

    /// Create a server command message (from central server to edge server)
    ///
    /// 上层服务器向边缘服务端发送指令
    ///
    /// # Examples
    ///
    /// ```rust
    /// use shared::message::{BusMessage, ServerCommandPayload};
    /// use serde_json::json;
    ///
    /// // 配置更新指令
    /// BusMessage::server_command(&ServerCommandPayload {
    ///     command: "config_update".to_string(),
    ///     data: json!({
    ///         "key": "printer.enabled",
    ///         "value": false,
    ///         "reason": "maintenance"
    ///     })
    /// });
    ///
    /// // 数据同步指令
    /// BusMessage::server_command("sync_dishes", json!({
    ///     "force": true,
    ///     "categories": ["hot", "cold"]
    /// }));
    ///
    /// // 远程控制指令
    /// BusMessage::server_command("restart", json!({
    ///     "delay_seconds": 30,
    ///     "reason": "system_upgrade"
    /// }));
    /// ```
    /// Parse payload as JSON
    pub fn parse_payload<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_message() {
        let msg = BusMessage::notification("Test Title", "Test Body");
        assert_eq!(msg.event_type, EventType::Notification);
        let parsed: serde_json::Value = msg.parse_payload().unwrap();
        assert_eq!(parsed["title"], "Test Title");
        assert_eq!(parsed["body"], "Test Body");
    }

    #[test]
    fn test_table_intent_message() {
        let msg = BusMessage::table_intent(
            "add_dish",
            serde_json::json!({"table_id": "T01", "dishes": [{"id": "D001"}]}),
        );
        assert_eq!(msg.event_type, EventType::TableIntent);
        let parsed: serde_json::Value = msg.parse_payload().unwrap();
        assert_eq!(parsed["action"], "add_dish");
    }

    #[test]
    fn test_table_sync_message() {
        let msg = BusMessage::table_sync(
            "dish_added",
            serde_json::json!({"table_id": "T01", "order_id": "ORD123"}),
        );
        assert_eq!(msg.event_type, EventType::TableSync);
        let parsed: serde_json::Value = msg.parse_payload().unwrap();
        assert_eq!(parsed["action"], "dish_added");
    }

    #[test]
    fn test_data_sync_message() {
        let msg = BusMessage::data_sync(
            "dish_price",
            serde_json::json!({"dish_id": "D001", "new_price": 4200}),
        );
        assert_eq!(msg.event_type, EventType::DataSync);
        let parsed: serde_json::Value = msg.parse_payload().unwrap();
        assert_eq!(parsed["sync_type"], "dish_price");
    }

    #[test]
    fn test_server_command_message() {
        let payload = ServerCommandPayload {
            command: "config_update".to_string(),
            data: serde_json::json!({"key": "printer.enabled", "value": false}),
        };
        let msg = BusMessage::server_command(&payload);
        assert_eq!(msg.event_type, EventType::ServerCommand);
        let parsed: ServerCommandPayload = msg.parse_payload().unwrap();
        assert_eq!(parsed.command, "config_update");
    }

    #[test]
    fn test_event_type_from_u8() {
        assert_eq!(EventType::try_from(1).unwrap(), EventType::TableIntent);
        assert_eq!(EventType::try_from(2).unwrap(), EventType::TableSync);
        assert_eq!(EventType::try_from(3).unwrap(), EventType::DataSync);
        assert_eq!(EventType::try_from(4).unwrap(), EventType::Notification);
        assert_eq!(EventType::try_from(5).unwrap(), EventType::ServerCommand);
        assert!(EventType::try_from(99).is_err());
    }
}
