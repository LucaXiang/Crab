use crate::core::Config;
use crate::core::ServerState;
use crate::message::{MessageBus, TransportConfig};
use std::sync::Arc;

/// 消息总线服务
///
/// 封装 MessageBus，提供：
/// - TCP 服务器启动
/// - 后台消息处理器
/// - 生命周期管理
#[derive(Clone, Debug)]
pub struct MessageBusService {
    /// 消息总线实例
    bus: Arc<MessageBus>,
    /// TCP 监听端口
    tcp_port: u16,
}

impl MessageBusService {
    /// 创建消息总线服务
    pub fn new(config: &Config) -> Self {
        let transport_config = TransportConfig {
            tcp_listen_addr: format!("0.0.0.0:{}", config.message_tcp_port),
            channel_capacity: 1024,
            tls_config: None, // TLS config will be provided during start_tcp_server
        };

        Self {
            bus: Arc::new(MessageBus::from_config(transport_config)),
            tcp_port: config.message_tcp_port,
        }
    }

    /// 获取消息总线引用
    pub fn bus(&self) -> &Arc<MessageBus> {
        &self.bus
    }

    /// 启动 TCP 服务器 (带 TLS 配置)
    pub async fn start_tcp_server(
        &self,
        tls_config: Arc<rustls::ServerConfig>,
    ) -> Result<(), crate::utils::AppError> {
        tracing::debug!(port = self.tcp_port, "Starting Message Bus TCP server");
        self.bus.start_tcp_server(Some(tls_config)).await
    }

    /// 启动后台消息处理器
    ///
    /// MessageHandler 订阅 client_tx，接收来自客户端的消息
    pub fn start_background_tasks(&self, state: ServerState) {
        // 1. 启动 MessageHandler
        let handler_receiver = self.bus.subscribe_to_clients();
        let handler_shutdown = self.bus.shutdown_token().clone();
        let server_tx = self.bus.sender().clone();

        let handler = crate::message::MessageHandler::with_default_processors(
            handler_receiver,
            handler_shutdown,
            state.clone().into(),
        )
        .with_broadcast_tx(server_tx);

        tokio::spawn(async move {
            handler.run().await;
        });

        tracing::debug!("Message handler started in background");
    }
}
