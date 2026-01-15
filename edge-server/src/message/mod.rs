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

// use async_trait::async_trait;
use async_trait::async_trait;
use crab_cert::CertMetadata;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf, split};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use tokio_rustls::{TlsAcceptor, server::TlsStream};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub mod handler;
pub mod processor;

pub use handler::MessageHandler;
pub use processor::{MessageProcessor, ProcessResult};
pub use shared::message::{
    BusMessage, EventType, NotificationPayload, RequestCommandPayload, ServerCommandPayload,
    SyncPayload,
};

use crate::common::AppError;

// ========== Transport 传输层特征 ==========

#[async_trait]
pub trait Transport: Send + Sync + std::fmt::Debug {
    async fn read_message(&self) -> Result<BusMessage, AppError>;
    async fn write_message(&self, msg: &BusMessage) -> Result<(), AppError>;
    async fn close(&self) -> Result<(), AppError>;
    fn peer_identity(&self) -> Option<String> {
        None
    }
}

// 辅助函数
async fn read_from_stream<R: AsyncReadExt + Unpin>(reader: &mut R) -> Result<BusMessage, AppError> {
    // 读取事件类型 (1 字节)
    let mut type_buf = [0u8; 1];
    reader
        .read_exact(&mut type_buf)
        .await
        .map_err(|e| AppError::internal(format!("Read type failed: {}", e)))?;

    let event_type =
        EventType::try_from(type_buf[0]).map_err(|_| AppError::invalid("Invalid event type"))?;

    // 读取 Request ID (16 字节)
    let mut uuid_buf = [0u8; 16];
    reader
        .read_exact(&mut uuid_buf)
        .await
        .map_err(|e| AppError::internal(format!("Read UUID failed: {}", e)))?;
    let request_id = Uuid::from_bytes(uuid_buf);

    // 读取 Correlation ID (16 字节)
    let mut correlation_buf = [0u8; 16];
    reader
        .read_exact(&mut correlation_buf)
        .await
        .map_err(|e| AppError::internal(format!("Read Correlation UUID failed: {}", e)))?;
    let correlation_id_raw = Uuid::from_bytes(correlation_buf);
    let correlation_id = if correlation_id_raw.is_nil() {
        None
    } else {
        Some(correlation_id_raw)
    };

    // 读取载荷长度 (4 字节)
    let mut len_buf = [0u8; 4];
    reader
        .read_exact(&mut len_buf)
        .await
        .map_err(|e| AppError::internal(format!("Read len failed: {}", e)))?;

    let len = u32::from_le_bytes(len_buf) as usize;

    // 读取载荷内容
    let mut payload = vec![0u8; len];
    reader
        .read_exact(&mut payload)
        .await
        .map_err(|e| AppError::internal(format!("Read payload failed: {}", e)))?;

    Ok(BusMessage {
        request_id,
        event_type,
        source: None,
        correlation_id,
        target: None,
        payload,
    })
}

async fn write_to_stream<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    msg: &BusMessage,
) -> Result<(), AppError> {
    let mut data = Vec::new();
    data.push(msg.event_type as u8);
    data.extend_from_slice(msg.request_id.as_bytes());

    // Write correlation_id (16 bytes) - using 0 if None
    let correlation_bytes = msg.correlation_id.unwrap_or(Uuid::nil()).into_bytes();
    data.extend_from_slice(&correlation_bytes);

    data.extend_from_slice(&(msg.payload.len() as u32).to_le_bytes());
    data.extend_from_slice(&msg.payload);

    writer
        .write_all(&data)
        .await
        .map_err(|e| AppError::internal(format!("Write failed: {}", e)))?;
    Ok(())
}

// ========== TCP 传输层实现 ==========

/// TCP 传输实现
#[derive(Debug, Clone)]
pub struct TcpTransport {
    reader: Arc<Mutex<OwnedReadHalf>>,
    writer: Arc<Mutex<OwnedWriteHalf>>,
}

impl TcpTransport {
    pub async fn connect(addr: &str) -> Result<Self, AppError> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| AppError::internal(format!("TCP connect failed: {}", e)))?;
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
        read_from_stream(&mut *reader).await
    }

    pub async fn write_message(&self, msg: &BusMessage) -> Result<(), AppError> {
        let mut writer = self.writer.lock().await;
        write_to_stream(&mut *writer, msg).await
    }

    pub async fn close(&self) -> Result<(), AppError> {
        let mut writer = self.writer.lock().await;
        writer
            .shutdown()
            .await
            .map_err(|e| AppError::internal(format!("TCP close failed: {}", e)))?;
        Ok(())
    }
}

#[async_trait]
impl Transport for TcpTransport {
    async fn read_message(&self) -> Result<BusMessage, AppError> {
        self.read_message().await
    }

    async fn write_message(&self, msg: &BusMessage) -> Result<(), AppError> {
        self.write_message(msg).await
    }

    async fn close(&self) -> Result<(), AppError> {
        let mut writer = self.writer.lock().await;
        writer
            .shutdown()
            .await
            .map_err(|e| AppError::internal(format!("TCP close failed: {}", e)))?;
        Ok(())
    }
}

// ========== TLS Transport ==========

#[derive(Debug, Clone)]
pub struct TlsTransport {
    reader: Arc<Mutex<ReadHalf<TlsStream<TcpStream>>>>,
    writer: Arc<Mutex<WriteHalf<TlsStream<TcpStream>>>>,
    peer_identity: Option<String>,
}

impl TlsTransport {
    pub fn new(stream: TlsStream<TcpStream>) -> Self {
        // Extract identity from TLS session
        let (_, connection) = stream.get_ref();
        let peer_identity = if let Some(certs) = connection.peer_certificates() {
            if let Some(cert) = certs.first() {
                CertMetadata::from_der(cert.as_ref())
                    .ok()
                    .and_then(|m| m.client_name.or(m.common_name))
            } else {
                None
            }
        } else {
            None
        };

        let (reader, writer) = split(stream);
        Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
            peer_identity,
        }
    }
}

#[async_trait]
impl Transport for TlsTransport {
    async fn read_message(&self) -> Result<BusMessage, AppError> {
        let mut reader = self.reader.lock().await;
        read_from_stream(&mut *reader).await
    }

    async fn write_message(&self, msg: &BusMessage) -> Result<(), AppError> {
        let mut writer = self.writer.lock().await;
        write_to_stream(&mut *writer, msg).await
    }

    async fn close(&self) -> Result<(), AppError> {
        let mut writer = self.writer.lock().await;
        writer
            .shutdown()
            .await
            .map_err(|e| AppError::internal(format!("TLS close failed: {}", e)))?;
        Ok(())
    }

    fn peer_identity(&self) -> Option<String> {
        self.peer_identity.clone()
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
            .map_err(|e| AppError::internal(e.to_string()))
    }

    pub async fn write_message(&self, msg: &BusMessage) -> Result<(), AppError> {
        // Send to server via client_tx (for simulating client messages)
        if let Some(tx) = &self.tx {
            tx.send(msg.clone())
                .map_err(|e| AppError::internal(e.to_string()))?;
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
    /// Connected clients (Client ID -> Transport)
    clients: Arc<DashMap<String, Arc<dyn Transport>>>,
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
            clients: Arc::new(DashMap::new()),
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
            .map_err(|e| AppError::internal(e.to_string()))?;
        Ok(())
    }

    /// Send a message TO SERVER (from client)
    pub async fn send_to_server(&self, msg: BusMessage) -> Result<(), AppError> {
        self.client_tx
            .send(msg)
            .map_err(|e| AppError::internal(e.to_string()))?;
        Ok(())
    }

    /// Send a message to a specific client
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
    pub async fn start_tcp_server(
        &self,
        tls_config_override: Option<Arc<rustls::ServerConfig>>,
    ) -> Result<(), AppError> {
        let listener = TcpListener::bind(&self.config.tcp_listen_addr)
            .await
            .map_err(|e| AppError::internal(format!("Failed to bind: {}", e)))?;

        tracing::info!(
            "Message bus TCP server listening on {}",
            self.config.tcp_listen_addr
        );

        let server_tx = self.server_tx.clone();
        let client_tx = self.client_tx.clone();
        let shutdown_token = self.shutdown_token.clone();
        let clients = self.clients.clone();

        // Prepare TLS acceptor: prefer override (from activation), then config
        let final_tls_config = tls_config_override.or(self.config.tls_config.clone());

        let tls_acceptor = if let Some(tls_config) = final_tls_config {
            tracing::info!("🔐 Message Bus mTLS enabled");
            Some(TlsAcceptor::from(tls_config))
        } else {
            // STRICT MODE: Do not start TCP server without TLS
            tracing::error!("❌ mTLS configuration missing. Refusing to start TCP server!");
            return Err(AppError::internal(
                "Refusing to start TCP server without mTLS configuration",
            ));
        };

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

                            let tls_acceptor = tls_acceptor.clone();
                            let server_tx = server_tx.clone();
                            let client_tx = client_tx.clone();
                            let shutdown_token = shutdown_token.clone();
                            let clients = clients.clone();

                            tokio::spawn(async move {
                                let transport: Arc<dyn Transport> = if let Some(acceptor) = tls_acceptor {
                                    match acceptor.accept(stream).await {
                                        Ok(tls_stream) => {
                                            tracing::info!("🔐 Client {} TLS handshake successful", addr);
                                            Arc::new(TlsTransport::new(tls_stream))
                                        }
                                        Err(e) => {
                                            tracing::error!("Client {} TLS handshake failed: {}", addr, e);
                                            return;
                                        }
                                    }
                                } else {
                                    Arc::new(TcpTransport::from_stream(stream))
                                };

                                // 🤝 握手与版本检查
                                // 强制客户端发送的第一条消息必须是 Handshake
                                tracing::debug!("Waiting for handshake from {}", addr);
                                let client_id = match transport.read_message().await {
                                    Ok(msg) if msg.event_type == EventType::Handshake => {
                                        match msg.parse_payload::<shared::message::HandshakePayload>() {
                                            Ok(payload) => {
                                                if payload.version != shared::message::PROTOCOL_VERSION {
                                                    tracing::warn!(
                                                        "❌ Client {} protocol version mismatch: expected {}, got {}",
                                                        addr, shared::message::PROTOCOL_VERSION, payload.version
                                                    );

                                                    // Send error notification to client
                                                    let error_msg = format!(
                                                        "Protocol version mismatch: server={}, client={}. Please update your client.",
                                                        shared::message::PROTOCOL_VERSION,
                                                        payload.version
                                                    );
                                                    let notification = shared::message::NotificationPayload::error(
                                                        "Handshake Failed",
                                                        error_msg
                                                    );
                                                    let msg = BusMessage::notification(&notification);

                                                    if let Err(e) = transport.write_message(&msg).await {
                                                        tracing::error!("Failed to send handshake error to {}: {}", addr, e);
                                                    }

                                                    // Give client some time to receive the message before closing
                                                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                                    return;
                                                }

                                                // 🔐 Identity Verification (if mTLS is active)
                                                if let Some(peer_id) = transport.peer_identity() {
                                                    if let Some(client_name) = &payload.client_name {
                                                        if &peer_id != client_name {
                                                            tracing::warn!(
                                                                "❌ Client {} identity mismatch: TLS cert says '{}', handshake says '{}'",
                                                                addr, peer_id, client_name
                                                            );

                                                            let error_msg = format!(
                                                                "Identity verification failed: Certificate subject='{}' does not match Handshake client_name='{}'.",
                                                                peer_id, client_name
                                                            );
                                                            let notification = shared::message::NotificationPayload::error(
                                                                "Handshake Failed",
                                                                error_msg
                                                            );
                                                            let msg = BusMessage::notification(&notification);
                                                            let _ = transport.write_message(&msg).await;
                                                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                                            return;
                                                        } else {
                                                            tracing::info!("✅ Client {} identity verified via mTLS: {}", addr, peer_id);
                                                        }
                                                    }
                                                }

                                                let cid = payload.client_id.unwrap_or_else(|| Uuid::new_v4().to_string());
                                                tracing::info!(
                                                    "✅ Client {} handshake success (v{}, client: {:?}, id: {})",
                                                    addr, payload.version, payload.client_name, cid
                                                );
                                                cid
                                            }
                                            Err(e) => {
                                                tracing::warn!("❌ Client {} sent invalid handshake payload: {}", addr, e);
                                                return;
                                            }
                                        }
                                    }
                                    Ok(msg) => {
                                        tracing::warn!(
                                            "❌ Client {} failed to handshake: expected Handshake, got {}",
                                            addr, msg.event_type
                                        );
                                        return;
                                    }
                                    Err(e) => {
                                        tracing::warn!("❌ Client {} handshake error: {}", addr, e);
                                        return;
                                    }
                                };

                                // Register client
                                clients.insert(client_id.clone(), transport.clone());
                                tracing::info!("Client registered: {}", client_id);

                                let mut rx = server_tx.subscribe();
                                let transport_clone = transport.clone();
                                let client_shutdown = shutdown_token.clone();
                                let client_id_clone = client_id.clone();
                                let clients_clone = clients.clone();

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
                                                        // Unicast Filtering:
                                                        // If message has a target, only send if it matches this client.
                                                        if let Some(target) = &msg.target {
                                                            if target != &client_id_clone {
                                                                continue;
                                                            }
                                                        }

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
                                    // Clean up on disconnect
                                    clients_clone.remove(&client_id_clone);
                                    tracing::info!("Client disconnected: {}", client_id_clone);
                                });

                                // Read messages from client (client → server)
                                let client_tx_clone = client_tx.clone();
                                let client_shutdown2 = shutdown_token.clone();
                                loop {
                                    tokio::select! {
                                        _ = client_shutdown2.cancelled() => {
                                            break;
                                        }
                                        // Read message from client
                                        read_result = transport.read_message() => {
                                            match read_result {
                                                Ok(mut msg) => {
                                                    // Inject client ID (Source Tracking)
                                                    msg.source = Some(client_id.clone());

                                                    // 🛡️ 安全检查：严禁客户端发送 ServerCommand
                                                    // ServerCommand 仅允许由 Upstream 通过受信任通道发送
                                                    if msg.event_type == EventType::ServerCommand {
                                                        tracing::warn!(
                                                            target: "security",
                                                            client_addr = %addr,
                                                            "⚠️ Security Alert: Client attempted to send ServerCommand. Dropping message."
                                                        );
                                                        continue;
                                                    }

                                                    // Publish to client_tx so server handlers receive it
                                                    // Messages will be filtered by should_broadcast() in filter.rs
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
                                // Ensure connection is closed gracefully
                                let _ = transport.close().await;
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
        let payload = NotificationPayload::info("Test", "Hello");
        let msg = BusMessage::notification(&payload);
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

        let payload = NotificationPayload::warning("System", "Shutting down");
        let msg = BusMessage::notification(&payload);
        bus.publish(msg.clone()).await.unwrap();

        let r1 = t1.read_message().await.unwrap();
        let r2 = t2.read_message().await.unwrap();

        assert_eq!(r1.event_type, EventType::Notification);
        assert_eq!(r2.event_type, EventType::Notification);
    }
}
