use crate::message::{MessageBus, TransportConfig};
use crate::server::Config;
use crate::server::ServerState;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct MessageBusService {
    bus: Arc<MessageBus>,
    tcp_port: u16,
}

impl MessageBusService {
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

    pub fn bus(&self) -> &Arc<MessageBus> {
        &self.bus
    }

    /// Explicitly start the TCP server with TLS config
    pub async fn start_tcp_server(
        &self,
        tls_config: Arc<rustls::ServerConfig>,
    ) -> Result<(), crate::AppError> {
        tracing::info!(
            "Starting Message Bus TCP server on port {}...",
            self.tcp_port
        );
        self.bus.start_tcp_server(Some(tls_config)).await
    }

    /// Start background message handler
    /// Requires ServerState because message processors need access to full server state
    pub fn start_background_tasks(&self, state: ServerState) {
        // 1. Start MessageHandler
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

        tracing::info!("Message handler with ACID support started in background");
    }
}
