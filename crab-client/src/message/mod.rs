// crab-client/src/message/mod.rs
// 消息模块 - RPC 客户端配置和错误类型

pub use shared::message::{BusMessage, EventType};

use std::time::Duration;

/// 消息客户端配置
#[derive(Debug, Clone)]
pub struct MessageClientConfig {
    /// 默认请求超时
    pub request_timeout: Duration,
    /// 是否启用自动重连
    pub auto_reconnect: bool,
    /// 重连延迟
    pub reconnect_delay: Duration,
    /// 最大重连延迟 (指数退避上限)
    pub max_reconnect_delay: Duration,
    /// 最大重连尝试次数 (0 表示无限重试)
    pub max_reconnect_attempts: u32,
    /// 心跳间隔 (0 表示禁用)
    pub heartbeat_interval: Duration,
    /// 心跳超时 (超过此时间未收到 pong 则认为断连)
    pub heartbeat_timeout: Duration,
    /// 重连时网络探测间隔 (在退避等待期间探测网络恢复)
    pub reconnect_probe_interval: Duration,
}

impl Default for MessageClientConfig {
    /// 局域网优化配置
    ///
    /// 特点：
    /// - 快速检测断连（最长 5 秒）
    /// - 快速重连（最长 10 秒退避）
    /// - 网络恢复 1 秒内重连
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(3),      // 局域网 3 秒足够
            auto_reconnect: true,
            reconnect_delay: Duration::from_millis(500),  // 首次重连 500ms
            max_reconnect_delay: Duration::from_secs(10), // 最长 10 秒退避
            max_reconnect_attempts: 20,                   // 最多 20 次重连
            heartbeat_interval: Duration::from_secs(5),   // 每 5 秒心跳
            heartbeat_timeout: Duration::from_secs(2),    // 2 秒超时
            reconnect_probe_interval: Duration::from_secs(1), // 每 1 秒探测
        }
    }
}

impl MessageClientConfig {
    /// 创建默认配置 (局域网优化)
    pub fn new() -> Self {
        Self::default()
    }

    /// 局域网配置 (默认)
    ///
    /// 特点：快速检测、快速恢复
    /// - 断连检测：最长 7 秒 (5s 心跳间隔 + 2s 超时)
    /// - 网络恢复：1 秒内重连
    pub fn lan() -> Self {
        Self::default()
    }

    /// 广域网/互联网配置
    ///
    /// 特点：容忍高延迟、减少心跳开销
    /// - 断连检测：最长 35 秒 (30s 心跳间隔 + 5s 超时)
    /// - 退避上限：60 秒
    pub fn wan() -> Self {
        Self {
            request_timeout: Duration::from_secs(10),
            auto_reconnect: true,
            reconnect_delay: Duration::from_secs(1),
            max_reconnect_delay: Duration::from_secs(60),
            max_reconnect_attempts: 20,
            heartbeat_interval: Duration::from_secs(30),
            heartbeat_timeout: Duration::from_secs(5),
            reconnect_probe_interval: Duration::from_secs(5),
        }
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

    /// 设置心跳间隔 (0 表示禁用)
    pub fn with_heartbeat_interval(mut self, interval: Duration) -> Self {
        self.heartbeat_interval = interval;
        self
    }

    /// 设置心跳超时
    pub fn with_heartbeat_timeout(mut self, timeout: Duration) -> Self {
        self.heartbeat_timeout = timeout;
        self
    }

    /// 设置最大重连尝试次数 (0 表示无限重试)
    pub fn with_max_reconnect_attempts(mut self, attempts: u32) -> Self {
        self.max_reconnect_attempts = attempts;
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
        assert_eq!(config.request_timeout, Duration::from_secs(3)); // 局域网默认 3 秒
        assert_eq!(config.heartbeat_interval, Duration::from_secs(5)); // 5 秒心跳
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

        // 创建双向通道: client -> server 和 server -> client
        let (client_tx, _) = broadcast::channel(16);
        let (server_tx, _) = broadcast::channel(16);
        let client = InMemoryMessageClient::new(client_tx, server_tx);

        let request = BusMessage::request_command(&shared::message::RequestCommandPayload {
            action: "test.action".to_string(),
            params: None,
        });

        // 使用 request 发送请求并等待响应
        let _response = client.request(&request, Duration::from_secs(1)).await;

        // 验证客户端可以发起请求
        assert!(client.is_connected());
    }
}
