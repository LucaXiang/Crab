// crab-client/src/message/mod.rs
// 消息模块 - RPC 客户端配置和错误类型

pub use shared::message::{BusMessage, EventType};

use std::time::Duration;
use thiserror::Error;
use tokio::sync::broadcast;

/// Error type for message client operations
#[derive(Debug, Error)]
pub enum MessageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Request timed out: {0}")]
    Timeout(String),

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("Protocol error: {0}")]
    Protocol(String),
}

impl From<broadcast::error::RecvError> for MessageError {
    fn from(e: broadcast::error::RecvError) -> Self {
        MessageError::Connection(e.to_string())
    }
}

impl From<serde_json::Error> for MessageError {
    fn from(e: serde_json::Error) -> Self {
        MessageError::InvalidMessage(e.to_string())
    }
}

pub type MessageResult<T> = Result<T, MessageError>;

/// 消息客户端配置
#[derive(Debug, Clone)]
pub struct MessageClientConfig {
    /// 默认请求超时
    pub request_timeout: Duration,
    /// 是否启用自动重连
    pub auto_reconnect: bool,
    /// 重连延迟
    pub reconnect_delay: Duration,
}

impl Default for MessageClientConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(5),  // 局域网 5 秒足够
            auto_reconnect: true,
            reconnect_delay: Duration::from_secs(1),
        }
    }
}

impl MessageClientConfig {
    /// 创建默认配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置请求超时
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// 设置自动重连
    pub fn with_auto_reconnect(mut self, enabled: bool) -> Self {
        self.auto_reconnect = enabled;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_config_default() {
        let config = MessageClientConfig::default();
        assert_eq!(config.request_timeout, Duration::from_secs(5));  // 局域网默认 5 秒
        assert!(config.auto_reconnect);
    }

    #[tokio::test]
    async fn test_config_builder() {
        let config = MessageClientConfig::new()
            .with_request_timeout(Duration::from_secs(60))
            .with_auto_reconnect(false);

        assert_eq!(config.request_timeout, Duration::from_secs(60));
        assert!(!config.auto_reconnect);
    }

    #[tokio::test]
    async fn test_in_memory_rpc() {
        use crate::InMemoryMessageClient;

        let (tx, _rx) = broadcast::channel(16);
        let client = InMemoryMessageClient::new(tx);

        let request = BusMessage::request_command(
            &shared::message::RequestCommandPayload {
                action: "test.action".to_string(),
                params: None,
            },
        );

        // 使用 request 发送请求并等待响应
        let _response = client.request(&request, Duration::from_secs(1)).await;

        // 验证客户端可以发起请求
        assert!(client.is_connected());
    }
}
