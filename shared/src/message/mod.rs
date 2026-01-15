//! 消息总线消息类型定义
//!
//! 这些类型在 edge-server 和 clients 之间共享，用于
//! 进程内（内存）和网络（TCP）通信。

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;

use uuid::Uuid;

pub mod payload;
pub use payload::*;

/// 协议版本号
pub const PROTOCOL_VERSION: u16 = 1;

/// 消息总线事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    /// 握手消息 (客户端 -> 服务端)
    /// 连接建立后发送的第一条消息，用于版本协商和身份标识
    Handshake = 0,

    /// 系统通知
    /// 系统通知（边缘服务端 → 所有客户端）：打印机状态、网络异常等
    Notification = 1,

    /// 服务器指令 - 来自上游/中心服务器的指令
    /// 服务器指令（上层服务器 → 边缘服务端）：配置更新、数据同步指令、远程控制等
    ServerCommand = 2,

    /// 客户端请求指令 - 来自客户端的请求
    /// 客户端请求（客户端 → 边缘服务端）：业务请求、资源操作等
    RequestCommand = 3,

    /// 同步信号 - 广播给所有客户端
    /// 同步信号（服务端 → 所有客户端）：通知客户端刷新数据
    Sync = 4,

    /// 请求响应 - 服务端对客户端请求的响应
    /// 包含执行结果（成功/失败）和数据
    Response = 5,
}

impl TryFrom<u8> for EventType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(EventType::Handshake),
            1 => Ok(EventType::Notification),
            2 => Ok(EventType::ServerCommand),
            3 => Ok(EventType::RequestCommand),
            4 => Ok(EventType::Sync),
            5 => Ok(EventType::Response),
            _ => Err(()),
        }
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventType::Handshake => write!(f, "handshake"),
            EventType::Notification => write!(f, "notification"),
            EventType::ServerCommand => write!(f, "server_command"),
            EventType::RequestCommand => write!(f, "request_command"),
            EventType::Sync => write!(f, "sync"),
            EventType::Response => write!(f, "response"),
        }
    }
}

/// 消息总线二进制消息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BusMessage {
    /// 唯一请求 ID (UUID v4)
    pub request_id: Uuid,
    /// 事件类型标识符
    pub event_type: EventType,
    /// 消息来源 (可选)
    ///
    /// 对于来自客户端的消息，由服务端在接收时自动填充。
    /// 对于来自服务端的消息，通常为空。
    pub source: Option<String>,
    /// 关联 ID (可选)
    ///
    /// 用于 "一问一答" (RPC) 模式。
    /// 如果这是一条响应消息（或由某请求触发的通知），此字段应包含原始请求的 `request_id`。
    pub correlation_id: Option<Uuid>,
    /// 目标客户端 ID (可选)
    ///
    /// 如果设置，此消息仅发送给指定的客户端 (Unicast)。
    /// 如果为空，则广播给所有客户端 (Broadcast)。
    pub target: Option<String>,
    /// 二进制载荷
    pub payload: Vec<u8>,
}

impl BusMessage {
    /// 创建新的总线消息
    pub fn new(event_type: EventType, payload: Vec<u8>) -> Self {
        Self {
            request_id: Uuid::new_v4(),
            event_type,
            source: None,
            correlation_id: None,
            target: None,
            payload,
        }
    }

    /// 设置目标客户端
    pub fn with_target(mut self, target: &str) -> Self {
        self.target = Some(target.to_string());
        self
    }

    /// 创建握手消息 (客户端 -> 服务端)
    ///
    /// # 示例
    ///
    /// ```rust
    /// use shared::message::{BusMessage, HandshakePayload, PROTOCOL_VERSION};
    ///
    /// let payload = HandshakePayload {
    ///     version: PROTOCOL_VERSION,
    ///     client_name: Some("test-client".to_string()),
    ///     client_version: Some("0.1.0".to_string()),
    ///     client_id: Some("uuid-v4".to_string()),
    /// };
    /// BusMessage::handshake(&payload);
    /// ```
    pub fn handshake(payload: &HandshakePayload) -> Self {
        Self::new(
            EventType::Handshake,
            serde_json::to_vec(payload).expect("Failed to serialize handshake payload"),
        )
    }

    /// 创建服务器指令消息 (上层服务器 -> 边缘服务端)
    ///
    /// 上层服务器向边缘服务端发送指令
    ///
    /// # 示例
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

    /// 创建通知消息 (服务端 -> 客户端)
    ///
    /// # 示例
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

    /// 创建请求指令消息 (客户端 -> 边缘服务端)
    ///
    /// 客户端向边缘服务端发送请求
    ///
    /// # 示例
    ///
    /// ```rust
    /// use shared::message::{BusMessage, RequestCommandPayload};
    /// use serde_json::json;
    ///
    /// let payload = RequestCommandPayload {
    ///     action: "order.add_item".to_string(),
    ///     params: Some(json!({ "dish_id": "123", "quantity": 1 })),
    /// };
    /// BusMessage::request_command(&payload);
    /// ```
    pub fn request_command(payload: &RequestCommandPayload) -> Self {
        Self::new(
            EventType::RequestCommand,
            serde_json::to_vec(payload).expect("Failed to serialize request command"),
        )
    }

    /// 创建同步信号消息 (服务端 -> 所有客户端)
    ///
    /// 服务端通知所有客户端同步数据
    ///
    /// # 示例
    ///
    /// ```rust
    /// use shared::message::{BusMessage, SyncPayload};
    ///
    /// let payload = SyncPayload {
    ///     resource: "order".to_string(),
    ///     id: Some("1001".to_string()),
    ///     action: "updated".to_string(),
    /// };
    /// BusMessage::sync(&payload);
    /// ```
    pub fn sync(payload: &SyncPayload) -> Self {
        Self::new(
            EventType::Sync,
            serde_json::to_vec(payload).expect("Failed to serialize sync payload"),
        )
    }

    /// 创建响应消息 (服务端 -> 客户端)
    pub fn response(payload: &ResponsePayload) -> Self {
        Self::new(
            EventType::Response,
            serde_json::to_vec(payload).expect("Failed to serialize response payload"),
        )
    }

    /// 解析载荷为指定类型
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
        assert!(!msg.request_id.is_nil()); // Ensure UUID is generated
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
        assert_eq!(EventType::try_from(1).unwrap(), EventType::Notification);
        assert_eq!(EventType::try_from(2).unwrap(), EventType::ServerCommand);
        assert_eq!(EventType::try_from(4).unwrap(), EventType::Sync);
        assert_eq!(EventType::try_from(5).unwrap(), EventType::Response);
        assert!(EventType::try_from(99).is_err());
    }
}
