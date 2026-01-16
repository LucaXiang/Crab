//! 消息总线传输层抽象
//!
//! 提供可插拔的传输层架构：
//! ```text
//! ┌─────────────────────────────────────────┐
//! │           MessageBus (消息总线)           │
//! │  ┌───────────────────────────────────┐  │
//! │  │  broadcast::Sender<BusMessage>    │  │
//! │  └───────────────────────────────────┘  │
//! └────────────────┬────────────────────────┘
//!                  │
//!         ┌────────┴────────┐
//!         │ Transport Trait │  ◄── 可插拔接口
//!         └────────┬────────┘
//!                  │
//!     ┌────────────┼────────────┐
//!     ▼            ▼            ▼
//! TcpTransport  TlsTransport  MemoryTransport
//! (TCP 协议)    (TLS 加密)    (同进程通信)
//! ```
//!
//! # 模块结构
//!
//! - `transport/` - 传输层实现 (TCP, TLS, Memory)
//! - `bus` - 消息总线核心
//! - `tcp_server` - TCP 服务器实现
//! - `handler` - 消息处理器
//! - `processor` - 消息处理逻辑

mod bus;
pub mod handler;
pub mod processor;
mod tcp_server;
pub mod transport;

// ========== Re-exports ==========

// Transport layer
pub use transport::{MemoryTransport, TcpTransport, TlsTransport, Transport};

// Message bus
pub use bus::{MessageBus, TransportConfig};

// Handler & Processor
pub use handler::MessageHandler;
pub use processor::{MessageProcessor, ProcessResult};

// Shared message types
pub use shared::message::{
    BusMessage, EventType, NotificationPayload, RequestCommandPayload, ServerCommandPayload,
    SyncPayload,
};

// ========== Types ==========

/// 已连接的客户端信息
#[derive(Debug, Clone)]
pub struct ConnectedClient {
    pub id: String,
    pub peer_identity: Option<String>,
    pub addr: Option<String>,
}

// ========== Tests ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_transport() {
        let bus = MessageBus::new();
        let transport = bus.memory_transport();

        // Publish
        let payload = NotificationPayload::info("Test", "Hello");
        let msg = BusMessage::notification(&payload);
        bus.publish(msg.clone())
            .await
            .expect("Failed to publish test message");

        // Receive via transport
        let received = transport
            .read_message()
            .await
            .expect("Failed to read test message");
        assert_eq!(received.event_type, EventType::Notification);
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = MessageBus::new();
        let t1 = bus.memory_transport();
        let t2 = bus.memory_transport();

        let payload = NotificationPayload::warning("System", "Shutting down");
        let msg = BusMessage::notification(&payload);
        bus.publish(msg.clone())
            .await
            .expect("Failed to publish test message to multiple subscribers");

        let r1 = t1
            .read_message()
            .await
            .expect("First subscriber failed to receive message");
        let r2 = t2
            .read_message()
            .await
            .expect("Second subscriber failed to receive message");

        assert_eq!(r1.event_type, EventType::Notification);
        assert_eq!(r2.event_type, EventType::Notification);
    }
}
