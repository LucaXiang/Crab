//! 消息总线核心实现
//!
//! # 架构
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                     MessageBus                           │
//! │  ┌───────────────────────────────────────────────────┐  │
//! │  │  broadcast::Sender<BusMessage>                    │  │
//! │  └───────────────────────────────────────────────────┘  │
//! └────────────────────────┬────────────────────────────────┘
//!                         │
//!              ┌──────────┴──────────┐
//!              │    Transport Trait  │  ◄── 可插拔实现
//!              └──────────┬──────────┘
//!                         │
//!     ┌───────────────────┼───────────────────┐
//!     ▼                   ▼                   ▼
//! TcpTransport      TlsTransport      MemoryTransport
//! (TCP 明文)        (mTLS 加密)       (同进程通信)
//! ```
//!
//! # 消息流
//!
//! ```text
//! Client ──▶ send_to_server() ──▶ client_tx ──▶ MessageHandler
//!                                           │
//! Server ──▶ publish() ────────▶ server_tx ──┤
//!                                           ▼
//!                                    Connected Clients
//! ```

use std::sync::Arc;

use dashmap::DashMap;
use shared::message::BusMessage;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use super::ConnectedClient;
use super::transport::{MemoryTransport, Transport};
use crate::utils::AppError;

/// Configuration for transport layer
#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub tcp_listen_addr: String,
    /// Capacity of the broadcast channel (default: 1024)
    pub channel_capacity: usize,
    /// TLS configuration for mTLS (optional)
    pub tls_config: Option<Arc<rustls::ServerConfig>>,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            tcp_listen_addr: "0.0.0.0:8081".to_string(),
            channel_capacity: 1024,
            tls_config: None,
        }
    }
}

/// 消息总线 - 负责消息路由和转发
///
/// # 职责
///
/// - 消息路由 (send_to_server, publish, send_to_client)
/// - 客户端管理 (connect, disconnect, get_connected_clients)
/// - 传输层抽象 (TCP/TLS/Memory)
#[derive(Debug, Clone)]
pub struct MessageBus {
    /// 客户端到服务器的消息通道
    client_tx: broadcast::Sender<BusMessage>,
    /// 服务器到客户端的广播通道
    server_tx: broadcast::Sender<BusMessage>,
    /// 传输层配置
    pub(crate) config: TransportConfig,
    /// 关闭信号令牌
    shutdown_token: CancellationToken,
    /// 已连接的客户端 (Client ID -> Transport)
    pub(crate) clients: Arc<DashMap<String, Arc<dyn Transport>>>,
}

impl MessageBus {
    /// 创建默认配置的消息总线
    pub fn new() -> Self {
        Self::from_config(TransportConfig::default())
    }

    /// 从配置创建消息总线
    pub fn from_config(config: TransportConfig) -> Self {
        let capacity = config.channel_capacity;
        let (client_tx, _) = broadcast::channel(capacity);
        let (server_tx, _) = broadcast::channel(capacity);
        Self {
            client_tx,
            server_tx,
            config,
            shutdown_token: CancellationToken::new(),
            clients: Arc::new(DashMap::new()),
        }
    }

    /// 创建指定容量的消息总线
    pub fn with_capacity(capacity: usize) -> Self {
        let config = TransportConfig {
            channel_capacity: capacity,
            ..Default::default()
        };
        Self::from_config(config)
    }

    /// 配置传输层参数
    pub fn with_config(mut self, config: TransportConfig) -> Self {
        self.config = config;
        self
    }

    /// 发布消息 (服务器 -> 所有订阅者)
    ///
    /// 用于广播通知到所有连接的客户端
    pub async fn publish(&self, msg: BusMessage) -> Result<(), AppError> {
        self.server_tx
            .send(msg)
            .map_err(|e| AppError::internal(e.to_string()))?;
        Ok(())
    }

    /// 发送消息到服务器 (客户端 -> 服务器)
    ///
    /// 消息通过 broadcast 通道发送到 MessageHandler 处理
    pub async fn send_to_server(&self, msg: BusMessage) -> Result<(), AppError> {
        self.client_tx
            .send(msg)
            .map_err(|e| AppError::internal(e.to_string()))?;
        Ok(())
    }

    /// 发送消息到指定客户端 (单播)
    ///
    /// # 错误
    ///
    /// 客户端未连接返回 404
    pub async fn send_to_client(&self, client_id: &str, msg: BusMessage) -> Result<(), AppError> {
        if let Some(transport) = self.clients.get(client_id) {
            transport.write_message(&msg).await.map_err(|e| {
                AppError::internal(format!("Failed to send to client {}: {}", client_id, e))
            })?;
            Ok(())
        } else {
            Err(AppError::not_found(format!(
                "Client {} not connected",
                client_id
            )))
        }
    }

    /// 订阅客户端消息 (服务器专用)
    ///
    /// MessageHandler 使用此方法接收来自客户端的请求
    pub fn subscribe_to_clients(&self) -> broadcast::Receiver<BusMessage> {
        self.client_tx.subscribe()
    }

    /// 订阅服务器广播 (客户端专用)
    ///
    /// 客户端使用此方法接收服务器通知
    pub fn subscribe(&self) -> broadcast::Receiver<BusMessage> {
        self.server_tx.subscribe()
    }

    /// 获取内存传输层 (同进程通信)
    ///
    /// 用于测试或 Oneshot 模式
    pub fn memory_transport(&self) -> MemoryTransport {
        MemoryTransport::new(&self.server_tx)
    }

    /// 获取客户端内存传输层 (可发送消息到服务器)
    pub fn client_memory_transport(&self) -> MemoryTransport {
        MemoryTransport::with_client_sender(&self.server_tx, &self.client_tx)
    }

    /// 获取客户端发送端 (client→server 通道)
    pub fn sender_to_server(&self) -> &broadcast::Sender<BusMessage> {
        &self.client_tx
    }

    /// 获取广播发送端 (高级用法)
    pub fn sender(&self) -> &broadcast::Sender<BusMessage> {
        &self.server_tx
    }

    /// 获取关闭令牌 (用于监控关闭信号)
    pub fn shutdown_token(&self) -> &CancellationToken {
        &self.shutdown_token
    }

    /// 获取已连接客户端列表
    pub fn get_connected_clients(&self) -> Vec<ConnectedClient> {
        self.clients
            .iter()
            .map(|entry| ConnectedClient {
                id: entry.key().clone(),
                peer_identity: entry.value().peer_identity(),
                addr: entry.value().peer_addr(),
            })
            .collect()
    }

    /// 优雅关闭消息总线
    ///
    /// 取消所有运行中的任务，包括 TCP 服务器
    pub fn shutdown(&self) {
        tracing::info!("Shutting down message bus");
        self.shutdown_token.cancel();
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}
