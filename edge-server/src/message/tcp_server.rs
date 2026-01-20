//! TCP 服务器实现
//!
//! 负责处理 TCP/TLS 客户端连接，包括：
//! - 监听连接
//! - TLS 握手
//! - 协议握手验证
//! - 消息转发

use std::net::SocketAddr;
use std::sync::Arc;

use dashmap::DashMap;
use shared::message::{BusMessage, EventType, HandshakePayload, PROTOCOL_VERSION, ResponsePayload};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_rustls::TlsAcceptor;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::bus::MessageBus;
use super::transport::{TcpTransport, TlsTransport, Transport};
use crate::utils::AppError;

impl MessageBus {
    /// Start TCP server (for network clients)
    ///
    /// This is a TCP server that:
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

        self.accept_loop(listener, tls_acceptor).await
    }

    /// Main accept loop
    async fn accept_loop(
        &self,
        listener: TcpListener,
        tls_acceptor: Option<TlsAcceptor>,
    ) -> Result<(), AppError> {
        loop {
            tokio::select! {
                _ = self.shutdown_token().cancelled() => {
                    tracing::info!("Message bus TCP server shutting down");
                    break;
                }

                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            tracing::debug!("Client connected: {}", addr);
                            self.spawn_client_handler(stream, addr, tls_acceptor.clone());
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

    /// Spawn a new task to handle client connection
    fn spawn_client_handler(
        &self,
        stream: TcpStream,
        addr: SocketAddr,
        tls_acceptor: Option<TlsAcceptor>,
    ) {
        let server_tx = self.sender().clone();
        let client_tx = self.sender_to_server().clone();
        let shutdown_token = self.shutdown_token().clone();
        let clients = self.clients.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_client_connection(
                stream,
                addr,
                tls_acceptor,
                server_tx,
                client_tx,
                shutdown_token,
                clients,
            )
            .await
            {
                tracing::debug!("Client {} handler finished: {}", addr, e);
            }
        });
    }
}

/// Handle a single client connection
async fn handle_client_connection(
    stream: TcpStream,
    addr: SocketAddr,
    tls_acceptor: Option<TlsAcceptor>,
    server_tx: broadcast::Sender<BusMessage>,
    client_tx: broadcast::Sender<BusMessage>,
    shutdown_token: CancellationToken,
    clients: Arc<DashMap<String, Arc<dyn Transport>>>,
) -> Result<(), AppError> {
    // TLS handshake if configured
    let transport: Arc<dyn Transport> = if let Some(acceptor) = tls_acceptor {
        match acceptor.accept(stream).await {
            Ok(tls_stream) => {
                tracing::debug!("🔐 Client {} TLS handshake successful", addr);
                Arc::new(TlsTransport::new(tls_stream))
            }
            Err(e) => {
                tracing::error!("Client {} TLS handshake failed: {}", addr, e);
                return Err(AppError::internal(format!("TLS handshake failed: {}", e)));
            }
        }
    } else {
        Arc::new(TcpTransport::from_stream(stream))
    };

    // Protocol handshake
    let client_id = perform_handshake(&transport, addr).await?;

    // Register client
    clients.insert(client_id.clone(), transport.clone());
    tracing::debug!("Client registered: {}", client_id);

    // 创建共享的断开检测 token
    let disconnect_token = CancellationToken::new();
    let disconnect_token_clone = disconnect_token.clone();

    // Start message forwarding (当客户端断开时，forwarder 也要停止)
    let forward_handle = spawn_server_to_client_forwarder(
        transport.clone(),
        server_tx.subscribe(),
        shutdown_token.clone(),
        client_id.clone(),
        disconnect_token_clone,
    );

    // Read messages from client - 当检测到断开时，取消 disconnect_token
    read_client_messages(
        &transport,
        &client_tx,
        &shutdown_token,
        &client_id,
        addr,
        disconnect_token,
    )
    .await;

    // Cleanup
    drop(forward_handle);
    let _ = transport.close().await;
    clients.remove(&client_id);
    tracing::debug!(client_id = %client_id, "Client removed from registry");

    Ok(())
}

/// Perform protocol handshake with client
async fn perform_handshake(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
) -> Result<String, AppError> {
    tracing::debug!("Waiting for handshake from {}", addr);

    let msg = transport.read_message().await.map_err(|e| {
        tracing::warn!("❌ Client {} handshake error: {}", addr, e);
        e
    })?;

    if msg.event_type != EventType::Handshake {
        tracing::warn!(
            "❌ Client {} failed to handshake: expected Handshake, got {}",
            addr,
            msg.event_type
        );
        return Err(AppError::invalid("Expected Handshake message"));
    }

    let payload: HandshakePayload = msg.parse_payload().map_err(|e| {
        tracing::warn!("❌ Client {} sent invalid handshake payload: {}", addr, e);
        AppError::invalid(format!("Invalid handshake payload: {}", e))
    })?;

    // Version check
    if payload.version != PROTOCOL_VERSION {
        tracing::warn!(
            "❌ Client {} protocol version mismatch: expected {}, got {}",
            addr,
            PROTOCOL_VERSION,
            payload.version
        );

        send_handshake_error(
            transport,
            &msg,
            &format!(
                "Protocol version mismatch: server={}, client={}. Please update your client.",
                PROTOCOL_VERSION, payload.version
            ),
        )
        .await;

        return Err(AppError::invalid("Protocol version mismatch"));
    }

    // Identity verification (mTLS)
    if let (Some(peer_id), Some(client_name)) = (transport.peer_identity(), &payload.client_name) {
        if &peer_id != client_name {
            tracing::warn!(
                "❌ Client {} identity mismatch: TLS cert says '{}', handshake says '{}'",
                addr,
                peer_id,
                client_name
            );

            send_handshake_error(
                transport,
                &msg,
                &format!(
                    "Identity verification failed: Certificate subject='{}' does not match Handshake client_name='{}'.",
                    peer_id, client_name
                ),
            )
            .await;

            return Err(AppError::invalid("Identity verification failed"));
        } else {
            tracing::debug!("✅ Client {} identity verified via mTLS: {}", addr, peer_id);
        }
    }

    let client_id = payload
        .client_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    tracing::debug!(
        "✅ Client {} handshake success (v{}, client: {:?}, id: {})",
        addr,
        payload.version,
        payload.client_name,
        client_id
    );

    // 发送 RPC 响应 (用 correlation_id 关联客户端的 request_id)
    let response_payload =
        ResponsePayload::success(format!("Connected as client: {}", client_id), None);
    let response = BusMessage::response(&response_payload).with_correlation_id(msg.request_id);
    if let Err(e) = transport.write_message(&response).await {
        tracing::warn!("Failed to send handshake response: {}", e);
    }

    Ok(client_id)
}

/// Delay before closing connection after sending error (allows client to receive the message)
const HANDSHAKE_ERROR_DELAY_MS: u64 = 100;

/// Send handshake error to client
async fn send_handshake_error(transport: &Arc<dyn Transport>, msg: &BusMessage, message: &str) {
    let response_payload = ResponsePayload::error(message, None);
    let response = BusMessage::response(&response_payload).with_correlation_id(msg.request_id);

    if let Err(e) = transport.write_message(&response).await {
        tracing::error!("Failed to send handshake error: {}", e);
    }

    // Give client some time to receive the message before closing
    tokio::time::sleep(tokio::time::Duration::from_millis(HANDSHAKE_ERROR_DELAY_MS)).await;
}

/// Spawn task to forward messages from server to client
fn spawn_server_to_client_forwarder(
    transport: Arc<dyn Transport>,
    mut rx: broadcast::Receiver<BusMessage>,
    shutdown_token: CancellationToken,
    client_id: String,
    disconnect_token: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_token.cancelled() => {
                    tracing::debug!("Client {} forwarder shutting down", client_id);
                    break;
                }
                _ = disconnect_token.cancelled() => {
                    tracing::debug!(client_id = %client_id, "Client disconnected, forwarder stopping");
                    break;
                }
                msg_result = rx.recv() => {
                    match msg_result {
                        Ok(msg) => {
                            // Unicast filtering: only send if target matches or no target
                            if msg.target.as_ref().is_some_and(|target| target != &client_id) {
                                continue;
                            }

                            if let Err(e) = transport.write_message(&msg).await {
                                tracing::debug!(client_id = %client_id, "Client write failed: {}", e);
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            // WiFi lag recovery: client fell behind, notify to resync
                            tracing::warn!(
                                client_id = %client_id,
                                dropped_messages = n,
                                "Client lagged behind, sending resync notification"
                            );
                            
                            // Send a Sync message to trigger client-side full resync
                            let resync_msg = BusMessage {
                                event_type: EventType::Sync,
                                request_id: Uuid::new_v4(),
                                correlation_id: None,
                                payload: serde_json::json!({
                                    "reason": "lagged",
                                    "dropped_messages": n,
                                    "action": "full_resync"
                                }).to_string().into_bytes(),
                                source: Some("server".to_string()),
                                target: Some(client_id.clone()),
                            };
                            
                            if let Err(e) = transport.write_message(&resync_msg).await {
                                tracing::debug!(client_id = %client_id, "Failed to send resync notification: {}", e);
                                break;
                            }
                            
                            // Continue listening - don't disconnect the client
                            continue;
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            // Channel truly closed
                            tracing::debug!(client_id = %client_id, "Broadcast channel closed");
                            break;
                        }
                    }
                }
            }
        }

        tracing::debug!(client_id = %client_id, "Client forwarder stopped");
    })
}

/// Read messages from client and forward to server
async fn read_client_messages(
    transport: &Arc<dyn Transport>,
    client_tx: &broadcast::Sender<BusMessage>,
    shutdown_token: &CancellationToken,
    client_id: &str,
    addr: SocketAddr,
    disconnect_token: CancellationToken,
) {
    loop {
        tokio::select! {
            _ = shutdown_token.cancelled() => {
                break;
            }

            read_result = transport.read_message() => {
                match read_result {
                    Ok(mut msg) => {
                        // Inject client ID (Source Tracking)
                        msg.source = Some(client_id.to_string());

                        // 🛡️ Security: Block ServerCommand from clients
                        if msg.event_type == EventType::ServerCommand {
                            tracing::warn!(
                                target: "security",
                                client_addr = %addr,
                                "⚠️ Security Alert: Client attempted to send ServerCommand. Dropping message."
                            );
                            continue;
                        }

                        // Publish to client_tx so server handlers receive it
                        if let Err(e) = client_tx.send(msg) {
                            tracing::warn!("Failed to publish client message: {}", e);
                        }
                    }
                    Err(e) => {
                        if e.code == shared::error::ErrorCode::ClientDisconnected {
                            tracing::debug!(client_id = %client_id, "Client {} disconnected", addr);
                        } else {
                            tracing::debug!(client_id = %client_id, "Client {} read error: {}", addr, e);
                        }
                        // 通知 forwarder 客户端已断开
                        disconnect_token.cancel();
                        break;
                    }
                }
            }
        }
    }
}
