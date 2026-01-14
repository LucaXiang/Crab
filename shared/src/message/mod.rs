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
    /// use shared::message::{BusMessage, OrderIntentPayload, TableId, OperatorId, OrderAction, DishItem};
    ///
    /// let payload = OrderIntentPayload {
    ///     table_id: TableId::new_unchecked("T01"),
    ///     order_id: None,
    ///     operator: Some(OperatorId::new("waiter_001")),
    ///     action: OrderAction::AddDish {
    ///         dishes: vec![DishItem::simple("D001", 1)],
    ///     },
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
    /// use shared::message::{BusMessage, DataSyncPayload, DishId};
    ///
    /// let payload = DataSyncPayload::DishPrice {
    ///     dish_id: DishId::new("D001"),
    ///     old_price: 3800,
    ///     new_price: 4200
    /// };
    /// BusMessage::data_sync(&payload);
    /// ```
    pub fn data_sync(payload: &DataSyncPayload) -> Self {
        Self::new(
            EventType::DataSync,
            serde_json::to_vec(payload).expect("Failed to serialize data sync"),
        )
    }

    /// Create a notification message
    ///
    /// # Examples
    ///
    /// ```rust
    /// use shared::message::{BusMessage, NotificationPayload};
    ///
    /// let payload = NotificationPayload::info("打印机缺纸", "请及时添加打印纸");
    /// BusMessage::notification(&payload);
    /// ```
    pub fn notification(payload: &NotificationPayload) -> Self {
        Self::new(
            EventType::Notification,
            serde_json::to_vec(payload).expect("Failed to serialize notification"),
        )
    }

    /// Create a server command message (from central server to edge server)
    ///
    /// 上层服务器向边缘服务端发送指令
    ///
    /// # Examples
    ///
    /// ```rust
    /// use shared::message::{BusMessage, ServerCommandPayload, ServerCommand};
    /// use serde_json::json;
    ///
    /// // 配置更新指令
    /// BusMessage::server_command(&ServerCommandPayload {
    ///     command: ServerCommand::ConfigUpdate {
    ///         key: "printer.enabled".to_string(),
    ///         value: json!(false),
    ///     }
    /// });
    ///
    /// // 数据同步指令
    /// use shared::message::{DataSyncType};
    /// BusMessage::server_command(&ServerCommandPayload {
    ///     command: ServerCommand::SyncData {
    ///         data_type: DataSyncType::Dishes,
    ///         force: true,
    ///     }
    /// });
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
        let payload = NotificationPayload::info("Test Title", "Test Body");
        let msg = BusMessage::notification(&payload);
        assert_eq!(msg.event_type, EventType::Notification);
        let parsed: NotificationPayload = msg.parse_payload().unwrap();
        assert_eq!(parsed.title, "Test Title");
        assert_eq!(parsed.message, "Test Body");
    }

    #[test]
    fn test_order_intent_message() {
        let payload = OrderIntentPayload {
            table_id: TableId::new_unchecked("T01"),
            order_id: None,
            operator: Some(OperatorId::new("waiter_001")),
            action: OrderAction::AddDish {
                dishes: vec![DishItem::simple("D001", 1)],
            },
        };
        let msg = BusMessage::order_intent(&payload);
        assert_eq!(msg.event_type, EventType::OrderIntent);
        let parsed: OrderIntentPayload = msg.parse_payload().unwrap();
        match parsed.action {
            OrderAction::AddDish { dishes } => {
                assert_eq!(dishes[0].dish_id.as_str(), "D001");
            }
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_order_sync_message() {
        let payload = OrderSyncPayload {
            table_id: TableId::new_unchecked("T01"),
            order_id: Some(OrderId::new_unchecked("ORD123")),
            status: OrderStatus::Pending,
            source: OperatorId::new("server"),
            data: None,
            action: OrderAction::AddDish {
                dishes: vec![DishItem::simple("D001", 1)],
            },
        };
        let msg = BusMessage::order_sync(&payload);
        assert_eq!(msg.event_type, EventType::OrderSync);
        let parsed: OrderSyncPayload = msg.parse_payload().unwrap();
        assert_eq!(parsed.table_id.as_str(), "T01");
        assert_eq!(parsed.order_id.unwrap().as_str(), "ORD123");
    }

    #[test]
    fn test_data_sync_message() {
        let payload = DataSyncPayload::DishPrice {
            dish_id: DishId::new("D001"),
            old_price: 100,
            new_price: 200,
        };
        let msg = BusMessage::data_sync(&payload);
        assert_eq!(msg.event_type, EventType::DataSync);
        let parsed: DataSyncPayload = msg.parse_payload().unwrap();
        match parsed {
            DataSyncPayload::DishPrice {
                dish_id,
                old_price,
                new_price,
            } => {
                assert_eq!(dish_id.as_str(), "D001");
                assert_eq!(old_price, 100);
                assert_eq!(new_price, 200);
            }
            _ => panic!("Wrong sync type"),
        }
    }

    #[test]
    fn test_server_command_message() {
        let payload = ServerCommandPayload {
            command: ServerCommand::ConfigUpdate {
                key: "printer.enabled".to_string(),
                value: serde_json::json!(false),
            },
        };
        let msg = BusMessage::server_command(&payload);
        assert_eq!(msg.event_type, EventType::ServerCommand);
        let parsed: ServerCommandPayload = msg.parse_payload().unwrap();
        match parsed.command {
            ServerCommand::ConfigUpdate { key, value } => {
                assert_eq!(key, "printer.enabled");
                assert_eq!(value, false);
            }
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_event_type_from_u8() {
        assert_eq!(EventType::try_from(1).unwrap(), EventType::OrderIntent);
        assert_eq!(EventType::try_from(2).unwrap(), EventType::OrderSync);
        assert_eq!(EventType::try_from(3).unwrap(), EventType::DataSync);
        assert_eq!(EventType::try_from(4).unwrap(), EventType::Notification);
        assert_eq!(EventType::try_from(5).unwrap(), EventType::ServerCommand);
        assert!(EventType::try_from(99).is_err());
    }
}
