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
            4 => Ok(EventType::Notification),
            5 => Ok(EventType::ServerCommand),
            _ => Err(()),
        }
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
    /// ```
    pub fn server_command(payload: &ServerCommandPayload) -> Self {
        let payload_bytes = serde_json::to_vec(payload).expect("Failed to serialize ServerCommand");
        Self::new(EventType::ServerCommand, payload_bytes)
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
        assert_eq!(EventType::try_from(4).unwrap(), EventType::Notification);
        assert_eq!(EventType::try_from(5).unwrap(), EventType::ServerCommand);
        assert!(EventType::try_from(1).is_err());
        assert!(EventType::try_from(99).is_err());
    }
}
