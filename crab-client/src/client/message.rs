// crab-client/src/client/message.rs
// 消息客户端 - TCP/TLS 通信

use async_trait::async_trait;
use shared::message::BusMessage;

/// 消息客户端 trait
#[async_trait]
pub trait MessageClient: Send + Sync {
    async fn send(&self, msg: &BusMessage) -> Result<(), crate::MessageError>;
    async fn recv(&self) -> Result<BusMessage, crate::MessageError>;
}

/// 网络消息客户端 (TCP/TLS) - 简化实现
#[derive(Debug, Clone)]
pub struct NetworkMessageClient;

impl NetworkMessageClient {
    pub async fn connect(_addr: &str) -> Result<Self, crate::MessageError> {
        Ok(Self)
    }
}

#[async_trait]
impl MessageClient for NetworkMessageClient {
    async fn send(&self, _msg: &BusMessage) -> Result<(), crate::MessageError> {
        Ok(())
    }

    async fn recv(&self) -> Result<BusMessage, crate::MessageError> {
        Err(crate::MessageError::Connection("Not connected".to_string()))
    }
}

/// 内存消息客户端 (同进程)
#[derive(Debug, Clone)]
pub struct MemoryMessageClient;

impl MemoryMessageClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MessageClient for MemoryMessageClient {
    async fn send(&self, _msg: &BusMessage) -> Result<(), crate::MessageError> {
        Ok(())
    }

    async fn recv(&self) -> Result<BusMessage, crate::MessageError> {
        Err(crate::MessageError::Connection("Not initialized".to_string()))
    }
}
