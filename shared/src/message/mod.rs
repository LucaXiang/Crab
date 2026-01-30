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
pub const PROTOCOL_VERSION: u16 = 2;

/// 简化消息总线事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    /// 握手消息
    Handshake = 0,
    /// 系统通知
    Notification = 1,
    /// 服务器指令
    ServerCommand = 2,
    /// 客户端请求
    RequestCommand = 3,
    /// 同步信号
    Sync = 4,
    /// 请求响应
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

/// 简化的消息结构 - 只包含业务必需字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<T> {
    pub event_type: EventType,
    pub data: T,
    pub request_id: Uuid,             // 用于消息追踪
    pub correlation_id: Option<Uuid>, // 用于RPC响应关联
}

impl<T> Message<T> {
    /// 创建新消息
    pub fn new(event_type: EventType, data: T) -> Self {
        Self {
            event_type,
            data,
            request_id: Uuid::new_v4(),
            correlation_id: None,
        }
    }

    /// 创建带关联ID的消息 (用于RPC)
    pub fn with_correlation(event_type: EventType, data: T, correlation_id: Uuid) -> Self {
        Self {
            event_type,
            data,
            request_id: Uuid::new_v4(),
            correlation_id: Some(correlation_id),
        }
    }

    /// 获取业务数据
    pub fn data(&self) -> &T {
        &self.data
    }

    /// 检查是否是RPC响应
    pub fn is_response(&self) -> bool {
        matches!(self.event_type, EventType::Response)
    }

    /// 检查是否是RPC请求
    pub fn is_request(&self) -> bool {
        matches!(self.event_type, EventType::RequestCommand)
    }

    /// 获取关联ID (如果这是响应消息)
    pub fn correlation_id(&self) -> Option<&Uuid> {
        self.correlation_id.as_ref()
    }

    /// 转换为BusMessage用于传输
    pub fn into_bus_message(self) -> BusMessage
    where
        T: Serialize,
    {
        let payload = serde_json::to_vec(&self.data).expect("Failed to serialize message data");

        BusMessage {
            request_id: self.request_id,
            event_type: self.event_type,
            source: None,
            correlation_id: self.correlation_id,
            target: None,
            payload,
        }
    }
}

impl<T: Serialize + DeserializeOwned> Message<T> {
    /// 序列化为二进制
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// 从二进制解析
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// 便利的类型别名
pub type NotificationMessage = Message<NotificationPayload>;
pub type ServerCommandMessage = Message<ServerCommandPayload>;
pub type RequestCommandMessage = Message<RequestCommandPayload>;
pub type SyncMessage = Message<SyncPayload>;
pub type ResponseMessage = Message<ResponsePayload>;

/// 消息总线消息体
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BusMessage {
    pub request_id: Uuid,
    pub event_type: EventType,
    pub source: Option<String>,
    pub correlation_id: Option<Uuid>,
    pub target: Option<String>,
    pub payload: Vec<u8>,
}

impl BusMessage {
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

    /// 设置关联 ID (用于 RPC 响应)
    pub fn with_correlation_id(mut self, id: Uuid) -> Self {
        self.correlation_id = Some(id);
        self
    }

    /// 创建握手消息
    pub fn handshake(payload: &HandshakePayload) -> Self {
        Self::new(
            EventType::Handshake,
            serde_json::to_vec(payload).expect("Failed to serialize handshake payload"),
        )
    }

    /// 创建服务器指令消息
    pub fn server_command(payload: &ServerCommandPayload) -> Self {
        let payload_bytes = serde_json::to_vec(payload).expect("Failed to serialize ServerCommand");
        Self::new(EventType::ServerCommand, payload_bytes)
    }

    /// 创建通知消息
    pub fn notification(payload: &NotificationPayload) -> Self {
        Self::new(
            EventType::Notification,
            serde_json::to_vec(payload).expect("Failed to serialize notification"),
        )
    }

    /// 创建请求指令消息
    pub fn request_command(payload: &RequestCommandPayload) -> Self {
        Self::new(
            EventType::RequestCommand,
            serde_json::to_vec(payload).expect("Failed to serialize request command"),
        )
    }

    /// 创建同步信号消息
    pub fn sync(payload: &SyncPayload) -> Self {
        Self::new(
            EventType::Sync,
            serde_json::to_vec(payload).expect("Failed to serialize sync payload"),
        )
    }

    /// 创建响应消息
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
    fn test_simple_message_creation() {
        let notification = NotificationMessage::new(
            EventType::Notification,
            NotificationPayload::info("Test", "Hello World"),
        );

        assert_eq!(notification.event_type, EventType::Notification);
        assert_eq!(notification.data().title, "Test");
    }

    #[test]
    fn test_rpc_message() {
        // RPC消息只需要检查event_type是否为RequestCommand
        let msg = RequestCommandMessage::new(
            EventType::RequestCommand,
            RequestCommandPayload {
                action: "ping".to_string(),
                params: None,
            },
        );

        assert_eq!(msg.event_type, EventType::RequestCommand);
        assert!(msg.is_request());
        assert!(msg.correlation_id.is_none()); // 初始创建时correlation_id为None
    }

    #[test]
    fn test_message_conversion() {
        let original_msg = NotificationMessage::new(
            EventType::Notification,
            NotificationPayload::info("Test", "Data"),
        );

        // 测试序列化/反序列化
        let bytes = original_msg.to_bytes().unwrap();
        let recovered_msg = NotificationMessage::from_bytes(&bytes).unwrap();
        assert_eq!(recovered_msg.data().title, "Test");
    }

    #[test]
    fn test_handshake_message() {
        let payload = HandshakePayload {
            version: PROTOCOL_VERSION,
            client_name: Some("test-client".to_string()),
            client_version: Some("0.1.0".to_string()),
            client_id: Some("uuid-v4".to_string()),
        };

        let msg = BusMessage::handshake(&payload);
        assert_eq!(msg.event_type, EventType::Handshake);
        assert!(!msg.request_id.is_nil());

        let parsed: HandshakePayload = msg.parse_payload().unwrap();
        assert_eq!(parsed.version, PROTOCOL_VERSION);
    }
}
