//! Transport layer abstraction for message bus
//!
//! Provides a pluggable transport layer architecture:
//! ```text
//! ┌─────────────────────────────────────────┐
//! │           MessageBus                     │
//! │  ┌───────────────────────────────────┐  │
//! │  │  broadcast::Sender<BusMessage>    │  │
//! │  └───────────────────────────────────┘  │
//! └────────────────┬────────────────────────┘
//!                  │
//!         ┌────────┴────────┐
//!         │ Transport Trait │  ◄── 可插拔
//!         └────────┬────────┘
//!                  │
//!     ┌────────────┼────────────┐
//!     ▼            ▼            ▼
//! TcpTransport  TlsTransport  MemoryTransport
//! (TCP)        (TLS)          (同进程)
//! ```

// use async_trait::async_trait;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

pub mod handler;
pub mod processor;

pub use handler::MessageHandler;
pub use processor::{MessageProcessor, ProcessResult};
pub use shared::message::{
    BusMessage, EventType, NotificationPayload, OrderIntentPayload, OrderSyncPayload,
};

use crate::common::AppError;

// ========== TCP Transport ==========

/// TCP transport implementation
///
/// Splits the TCP stream into separate read and write halves for concurrent operations.
/// This allows reading and writing to happen simultaneously without blocking each other.
#[derive(Debug, Clone)]
pub struct TcpTransport {
    reader: Arc<Mutex<OwnedReadHalf>>,
    writer: Arc<Mutex<OwnedWriteHalf>>,
}

impl TcpTransport {
    pub async fn connect(addr: &str) -> Result<Self, AppError> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| AppError::Internal(format!("TCP connect failed: {}", e)))?;
        let (reader, writer) = stream.into_split();
        Ok(Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
        })
    }

    fn from_stream(stream: TcpStream) -> Self {
        let (reader, writer) = stream.into_split();
        Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
        }
    }

    pub async fn read_message(&self) -> Result<BusMessage, AppError> {
        let mut reader = self.reader.lock().await;

        // Read event type (1 byte)
        let mut type_buf = [0u8; 1];
        reader
            .read_exact(&mut type_buf)
            .await
            .map_err(|e| AppError::Internal(format!("Read type failed: {}", e)))?;

        let event_type = EventType::try_from(type_buf[0])
            .map_err(|_| AppError::Invalid("Invalid event type".into()))?;

        // Read payload length (4 bytes)
        let mut len_buf = [0u8; 4];
        reader
            .read_exact(&mut len_buf)
            .await
            .map_err(|e| AppError::Internal(format!("Read len failed: {}", e)))?;

        let len = u32::from_le_bytes(len_buf) as usize;

        // Read payload
        let mut payload = vec![0u8; len];
        reader
            .read_exact(&mut payload)
            .await
            .map_err(|e| AppError::Internal(format!("Read payload failed: {}", e)))?;

        Ok(BusMessage::new(event_type, payload))
    }

    pub async fn write_message(&self, msg: &BusMessage) -> Result<(), AppError> {
        let mut writer = self.writer.lock().await;
        let mut data = Vec::new();
        data.push(msg.event_type as u8);
        data.extend_from_slice(&(msg.payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&msg.payload);

        writer
            .write_all(&data)
            .await
            .map_err(|e| AppError::Internal(format!("Write failed: {}", e)))?;
        Ok(())
    }

    pub async fn close(&self) -> Result<(), AppError> {
        // Drop the reader and writer locks to close the connection
        drop(self.reader.lock().await);
        drop(self.writer.lock().await);
        Ok(())
    }
}

// ========== Memory Transport (In-Process) ==========

/// In-process memory transport for same-process communication
///
/// Uses tokio broadcast channel internally for zero-copy messaging.
#[derive(Debug, Clone)]
pub struct MemoryTransport {
    rx: Arc<Mutex<broadcast::Receiver<BusMessage>>>,
    tx: Option<Arc<broadcast::Sender<BusMessage>>>,
}

impl MemoryTransport {
    /// Create from a message bus sender (for receiving broadcasts)
    pub fn new(tx: &broadcast::Sender<BusMessage>) -> Self {
        Self {
            rx: Arc::new(Mutex::new(tx.subscribe())),
            tx: None,
        }
    }

    /// Create with client sender for simulating client messages
    pub fn with_client_sender(
        broadcast_tx: &broadcast::Sender<BusMessage>,
        client_tx: &broadcast::Sender<BusMessage>,
    ) -> Self {
        Self {
            rx: Arc::new(Mutex::new(broadcast_tx.subscribe())),
            tx: Some(Arc::new(client_tx.clone())),
        }
    }

    pub async fn read_message(&self) -> Result<BusMessage, AppError> {
        let mut rx = self.rx.lock().await;
        rx.recv()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn write_message(&self, msg: &BusMessage) -> Result<(), AppError> {
        // Send to server via client_tx (for simulating client messages)
        if let Some(tx) = &self.tx {
            tx.send(msg.clone())
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn close(&self) -> Result<(), AppError> {
        Ok(())
    }
}

// ========== Message Bus ==========

/// Configuration for transport layer
#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub tcp_listen_addr: String,
    /// Capacity of the broadcast channel (default: 1024)
    pub channel_capacity: usize,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            tcp_listen_addr: "0.0.0.0:8081".to_string(),
            channel_capacity: 1024,
        }
    }
}

/// Unified message bus with pluggable transport
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────────────────────────┐
/// │                     MessageBus                           │
/// │  ┌───────────────────────────────────────────────────┐  │
/// │  │  broadcast::Sender<BusMessage>                    │  │
/// │  └───────────────────────────────────────────────────┘  │
/// └────────────────────────┬────────────────────────────────┘
///                         │
///              ┌──────────┴──────────┐
///              │    Transport Trait  │  ◄── 可插拔实现
///              └──────────┬──────────┘
///                         │
///     ┌───────────────────┼───────────────────┐
///     ▼                   ▼                   ▼
/// TcpTransport      TlsTransport      MemoryTransport
/// (plain TCP)       (future: TLS)      (in-process)
/// ```
#[derive(Debug, Clone)]
pub struct MessageBus {
    client_tx: broadcast::Sender<BusMessage>,
    server_tx: broadcast::Sender<BusMessage>,
    config: TransportConfig,
    shutdown_token: CancellationToken,
}

impl MessageBus {
    /// Create a new message bus with default configuration
    pub fn new() -> Self {
        Self::from_config(TransportConfig::default())
    }

    /// Create a new message bus from configuration
    pub fn from_config(config: TransportConfig) -> Self {
        let capacity = config.channel_capacity;
        let (client_tx, _) = broadcast::channel(capacity);
        let (server_tx, _) = broadcast::channel(capacity);
        Self {
            client_tx,
            server_tx,
            config,
            shutdown_token: CancellationToken::new(),
        }
    }

    /// Create a new message bus with custom channel capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let config = TransportConfig {
            channel_capacity: capacity,
            ..Default::default()
        };
        Self::from_config(config)
    }

    /// Configure transport layer
    pub fn with_config(mut self, config: TransportConfig) -> Self {
        self.config = config;
        self
    }

    /// Publish a message FROM SERVER to all subscribers (broadcast)
    pub async fn publish(&self, msg: BusMessage) -> Result<(), AppError> {
        self.server_tx
            .send(msg)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    /// Send a message TO SERVER (from client)
    pub async fn send_to_server(&self, msg: BusMessage) -> Result<(), AppError> {
        self.client_tx
            .send(msg)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    /// Subscribe to receive messages FROM CLIENTS (server use only)
    pub fn subscribe_to_clients(&self) -> broadcast::Receiver<BusMessage> {
        self.client_tx.subscribe()
    }

    /// Subscribe to receive broadcasts FROM SERVER (clients use this)
    pub fn subscribe(&self) -> broadcast::Receiver<BusMessage> {
        self.server_tx.subscribe()
    }

    /// Get a memory transport for in-process communication
    pub fn memory_transport(&self) -> MemoryTransport {
        MemoryTransport::new(&self.server_tx)
    }

    /// Get a client memory transport that can send messages to server
    pub fn client_memory_transport(&self) -> MemoryTransport {
        MemoryTransport::with_client_sender(&self.server_tx, &self.client_tx)
    }

    /// Get the sender for clients to send TO server (client→server channel)
    pub fn sender_to_server(&self) -> &broadcast::Sender<BusMessage> {
        &self.client_tx
    }

    /// Get the broadcast sender (for advanced use)
    pub fn sender(&self) -> &broadcast::Sender<BusMessage> {
        &self.server_tx
    }

    /// Get the shutdown token (for monitoring shutdown signals)
    pub fn shutdown_token(&self) -> &CancellationToken {
        &self.shutdown_token
    }

    /// Gracefully shutdown the message bus
    ///
    /// This cancels all running tasks including the TCP server.
    pub fn shutdown(&self) {
        tracing::info!("Shutting down message bus");
        self.shutdown_token.cancel();
    }

    /// Start TCP server (for network clients)
    ///
    /// This is a simple TCP server that:
    /// 1. Accepts connections
    /// 2. Reads messages from clients and publishes to client_tx (server receives)
    /// 3. Forwards server broadcast messages to connected clients
    /// 4. Gracefully shuts down on cancellation signal
    pub async fn start_tcp_server(&self) -> Result<(), AppError> {
        let listener = TcpListener::bind(&self.config.tcp_listen_addr)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to bind: {}", e)))?;

        tracing::info!(
            "Message bus TCP server listening on {}",
            self.config.tcp_listen_addr
        );

        let server_tx = self.server_tx.clone();
        let client_tx = self.client_tx.clone();
        let shutdown_token = self.shutdown_token.clone();

        loop {
            tokio::select! {
                // Listen for shutdown signal
                _ = shutdown_token.cancelled() => {
                    tracing::info!("Message bus TCP server shutting down");
                    break;
                }

                // Accept new connections
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            tracing::info!("Client connected: {}", addr);

                            let transport = TcpTransport::from_stream(stream);
                            let mut rx = server_tx.subscribe();
                            let transport_clone = transport.clone();
                            let client_shutdown = shutdown_token.clone();

                            // Spawn task to forward messages to this client (server → client)
                            tokio::spawn(async move {
                                loop {
                                    tokio::select! {
                                        // Listen for shutdown signal
                                        _ = client_shutdown.cancelled() => {
                                            tracing::info!("Client {} handler shutting down", addr);
                                            break;
                                        }

                                        // Receive messages from bus (server broadcasts)
                                        msg_result = rx.recv() => {
                                            match msg_result {
                                                Ok(msg) => {
                                                    if let Err(e) = transport_clone.write_message(&msg).await {
                                                        tracing::info!("Client {} disconnected: {}", addr, e);
                                                        break;
                                                    }
                                                }
                                                Err(_) => {
                                                    // Channel closed
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            });

                            // Spawn task to read messages from client (client → server)
                            let client_tx_clone = client_tx.clone();
                            let client_shutdown2 = shutdown_token.clone();
                            tokio::spawn(async move {
                                loop {
                                    tokio::select! {
                                        _ = client_shutdown2.cancelled() => {
                                            break;
                                        }
                                        // Read message from client
                                        read_result = async { transport.read_message().await } => {
                                            match read_result {
                                                Ok(msg) => {
                                                    // Publish to client_tx so server handlers receive it
                                                    // TableIntent messages will NOT be broadcast back to clients
                                                    // (see should_broadcast() in filter.rs)
                                                    if let Err(e) = client_tx_clone.send(msg) {
                                                        tracing::warn!("Failed to publish client message: {}", e);
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::info!("Client {} read error: {}", addr, e);
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("Failed to accept connection: {}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

// ========== TCP Client ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_transport() {
        let bus = MessageBus::new();
        let transport = bus.memory_transport();

        // Publish
        let msg = BusMessage::notification("Test", "Hello");
        bus.publish(msg.clone()).await.unwrap();

        // Receive via transport
        let received = transport.read_message().await.unwrap();
        assert_eq!(received.event_type, EventType::Notification);
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = MessageBus::new();
        let t1 = bus.memory_transport();
        let t2 = bus.memory_transport();

        let msg = BusMessage::table_sync("test_action", serde_json::json!({"test": "data"}));
        bus.publish(msg.clone()).await.unwrap();

        let r1 = t1.read_message().await.unwrap();
        let r2 = t2.read_message().await.unwrap();

        assert_eq!(r1.event_type, EventType::TableSync);
        assert_eq!(r2.event_type, EventType::TableSync);
    }
}
