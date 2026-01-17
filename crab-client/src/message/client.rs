use crate::ClientType;
use crate::message::MessageError;
use crate::message::transport::{MemoryTransport, TcpTransport, TlsTransport, Transport};
use rustls::ClientConfig;
use shared::message::{BusMessage, EventType, HandshakePayload, PROTOCOL_VERSION};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio::sync::oneshot;
use uuid::Uuid;

/// Message Client
///
/// A unified client for communicating with the Edge Server.
/// Supports both Command-Response (Ack) pattern and Event Subscription.
#[derive(Debug, Clone)]
pub struct MessageClient {
    transport: ClientTransport,
    event_tx: broadcast::Sender<BusMessage>,
    pending_requests: Arc<Mutex<HashMap<Uuid, oneshot::Sender<BusMessage>>>>,
}

#[derive(Debug, Clone)]
enum ClientTransport {
    Tcp(TcpTransport),
    Tls(TlsTransport),
    Memory(MemoryTransport),
}

impl ClientTransport {
    async fn read_message(&self) -> Result<BusMessage, MessageError> {
        match self {
            ClientTransport::Tcp(t) => t.read_message().await,
            ClientTransport::Tls(t) => t.read_message().await,
            ClientTransport::Memory(t) => t.read_message().await,
        }
    }

    async fn write_message(&self, msg: &BusMessage) -> Result<(), MessageError> {
        match self {
            ClientTransport::Tcp(t) => t.write_message(msg).await,
            ClientTransport::Tls(t) => t.write_message(msg).await,
            ClientTransport::Memory(t) => t.write_message(msg).await,
        }
    }

    async fn close(&self) -> Result<(), MessageError> {
        match self {
            ClientTransport::Tcp(t) => t.close().await,
            ClientTransport::Tls(t) => t.close().await,
            ClientTransport::Memory(t) => t.close().await,
        }
    }
}

impl MessageClient {
    /// Create a message client from a ClientConfig
    pub async fn from_config(config: &crate::ClientConfig) -> Result<Self, MessageError> {
        match config.client_type {
            ClientType::Http => {
                // HTTP client type is not supported for message client
                Err(MessageError::Connection(
                    "Message client does not support HTTP client type".to_string(),
                ))
            }
            ClientType::Message => {
                if let Some(ref tcp_addr) = config.message_tcp_addr {
                    // Check if we need TLS configuration
                    let client_transport = if let Some(ref ca_cert_pem) = config.tls_ca_cert {
                        // Optional: Validate certificate chain if root CA is provided
                        if let Some(ref root_ca_pem) = config.tls_root_ca_cert {
                            tracing::info!("Validating certificate chain: Root CA -> Tenant CA");
                            match crab_cert::verify_chain_against_root(ca_cert_pem, root_ca_pem) {
                                Ok(_) => tracing::info!("✅ Certificate chain validation passed"),
                                Err(e) => {
                                    tracing::warn!("⚠️ Certificate chain validation failed: {}", e);
                                    // Continue anyway for backward compatibility
                                }
                            }
                        }

                        let mut ca_reader = std::io::Cursor::new(ca_cert_pem);
                        let ca_certs: Vec<rustls::pki_types::CertificateDer> =
                            rustls_pemfile::certs(&mut ca_reader)
                                .collect::<Result<Vec<_>, _>>()
                                .unwrap_or_else(|e| {
                                    tracing::warn!("Failed to parse CA certificates: {}", e);
                                    Vec::new()
                                });

                        let mut root_store = rustls::RootCertStore::empty();
                        for cert in ca_certs {
                            root_store.add(cert).unwrap_or_else(|e| {
                                tracing::warn!("Failed to add CA certificate: {}", e);
                            });
                        }

                        // 检查root store是否为空
                        if root_store.len() == 0 {
                            return Err(MessageError::Connection(
                                "No valid CA certificates found".to_string(),
                            ));
                        }

                        let verifier =
                            std::sync::Arc::new(crab_cert::SkipHostnameVerifier::new(root_store));

                        let config_builder = rustls::ClientConfig::builder()
                            .dangerous()
                            .with_custom_certificate_verifier(verifier);

                        let tls_config = if let (Some(cert_pem), Some(key_pem)) =
                            (&config.tls_client_cert, &config.tls_client_key)
                        {
                            let mut cert_reader = std::io::Cursor::new(cert_pem);
                            let certs: Vec<rustls::pki_types::CertificateDer> =
                                rustls_pemfile::certs(&mut cert_reader)
                                    .collect::<Result<Vec<_>, _>>()
                                    .unwrap_or_else(|e| {
                                        tracing::warn!(
                                            "Failed to parse client certificates: {}",
                                            e
                                        );
                                        Vec::new()
                                    });

                            let mut key_reader = std::io::Cursor::new(key_pem);
                            let key = rustls_pemfile::private_key(&mut key_reader)
                                .unwrap_or_else(|e| {
                                    tracing::warn!("Failed to parse client key: {}", e);
                                    panic!("Failed to parse client key");
                                })
                                .expect("Client key must be present");

                            config_builder
                                .with_client_auth_cert(certs, key)
                                .expect("Failed to set client auth")
                        } else {
                            config_builder.with_no_client_auth()
                        };

                        // Connect via TLS
                        let transport =
                            TlsTransport::connect(tcp_addr, "localhost", tls_config).await?;
                        ClientTransport::Tls(transport)
                    } else {
                        // Plain TCP connection
                        let transport = TcpTransport::connect(tcp_addr).await?;
                        ClientTransport::Tcp(transport)
                    };

                    // 🤝 Perform Handshake
                    let payload = HandshakePayload {
                        version: shared::message::PROTOCOL_VERSION,
                        client_name: Some("crab-client".to_string()),
                        client_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                        client_id: None, // Let server generate
                    };

                    client_transport
                        .write_message(&BusMessage::handshake(&payload))
                        .await?;

                    Ok(Self::new(client_transport))
                } else {
                    Err(MessageError::Connection(
                        "Message client requires tcp_addr configuration".to_string(),
                    ))
                }
            }
        }
    }

    fn new(transport: ClientTransport) -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        let pending_requests: Arc<Mutex<HashMap<Uuid, oneshot::Sender<BusMessage>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let client = Self {
            transport: transport.clone(),
            event_tx: event_tx.clone(),
            pending_requests: pending_requests.clone(),
        };

        // Spawn background task to dispatch messages
        let pending_requests_clone = pending_requests.clone();
        tokio::spawn(async move {
            loop {
                match transport.read_message().await {
                    Ok(msg) => {
                        // 1. Check if it's a "Reply" (has correlation_id)
                        if let Some(correlation_id) = msg.correlation_id {
                            let mut pending = pending_requests_clone.lock().unwrap();
                            if let Some(tx) = pending.remove(&correlation_id) {
                                let _ = tx.send(msg.clone());
                                // We continue to broadcast because others might be interested
                                // (e.g. logging all notifications)
                            }
                        }

                        // 2. Forward messages to event bus
                        if let Err(e) = event_tx.send(msg) {
                            tracing::debug!("No subscribers for event: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Transport read error: {}", e);
                        // 连接断开，客户端需重新 connect
                        break;
                    }
                }
            }
        });

        client
    }

    /// Connect via TCP
    pub async fn connect(addr: &str, client_name: &str) -> Result<Self, MessageError> {
        let transport = TcpTransport::connect(addr).await?;
        let client_transport = ClientTransport::Tcp(transport);

        // 🤝 Perform Handshake
        let payload = HandshakePayload {
            version: shared::message::PROTOCOL_VERSION,
            client_name: Some(client_name.to_string()),
            client_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            client_id: None, // Let server generate
        };

        client_transport
            .write_message(&BusMessage::handshake(&payload))
            .await?;

        Ok(Self::new(client_transport))
    }

    /// Connect via TLS
    pub async fn connect_tls(
        addr: &str,
        domain: &str,
        config: ClientConfig,
        client_name: &str,
    ) -> Result<Self, MessageError> {
        let transport = TlsTransport::connect(addr, domain, config).await?;
        let client_transport = ClientTransport::Tls(transport);

        // 🤝 Perform Handshake
        let payload = HandshakePayload {
            version: PROTOCOL_VERSION,
            client_name: Some(client_name.to_string()),
            client_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            client_id: None,
        };
        client_transport
            .write_message(&BusMessage::handshake(&payload))
            .await?;

        Ok(Self::new(client_transport))
    }

    /// Create in-memory client
    pub fn memory(
        server_broadcast_tx: &broadcast::Sender<BusMessage>,
        client_to_server_tx: &broadcast::Sender<BusMessage>,
    ) -> Self {
        let transport = MemoryTransport::new(server_broadcast_tx, client_to_server_tx);
        Self::new(ClientTransport::Memory(transport))
    }

    /// Receive a message (Subscribe to events)
    ///
    /// This receives broadcast events (Notification, Sync, ServerCommand).
    /// It does NOT receive direct Responses (Acks) to requests, as those are handled by `send_command`.
    pub async fn recv(&self) -> Result<BusMessage, MessageError> {
        let mut rx = self.event_tx.subscribe();
        rx.recv()
            .await
            .map_err(|e| MessageError::Connection(format!("Event bus error: {}", e)))
    }

    /// Send a message (Fire and Forget)
    pub async fn send(&self, msg: &BusMessage) -> Result<(), MessageError> {
        self.transport.write_message(msg).await
    }

    /// Send a message and await the server's acknowledgment/result.
    ///
    /// This applies the "RPC pattern" to ANY message type (Notification, Sync, RequestCommand).
    /// The server will return a Notification with matching `correlation_id`.
    pub async fn request(&self, msg: &BusMessage) -> Result<BusMessage, MessageError> {
        let request_id = msg.request_id;
        let (tx, rx) = oneshot::channel();

        // Register pending request
        {
            let mut pending = self.pending_requests.lock().unwrap();
            pending.insert(request_id, tx);
        }

        // Send request
        if let Err(e) = self.send(msg).await {
            // Cleanup on send failure
            let mut pending = self.pending_requests.lock().unwrap();
            pending.remove(&request_id);
            return Err(e);
        }

        // Wait for response with timeout
        match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(MessageError::Connection(
                "Response channel closed".to_string(),
            )),
            Err(_) => {
                // Timeout cleanup
                let mut pending = self.pending_requests.lock().unwrap();
                pending.remove(&request_id);
                Err(MessageError::Timeout("Request timed out".to_string()))
            }
        }
    }

    /// Send a command (RPC style)
    ///
    /// This sends a command and awaits a matching response (Notification) from the server.
    /// It uses `correlation_id` to match the response to the request.
    ///
    /// Returns the response `BusMessage` (usually a Notification).
    pub async fn send_command(
        &self,
        action: &str,
        params: Option<serde_json::Value>,
    ) -> Result<BusMessage, MessageError> {
        let payload = shared::message::RequestCommandPayload {
            action: action.to_string(),
            params,
        };

        let msg = BusMessage::new(
            EventType::RequestCommand,
            serde_json::to_vec(&payload).unwrap(),
        );

        self.request(&msg).await
    }

    /// Close the client connection
    pub async fn close(&self) -> Result<(), MessageError> {
        self.transport.close().await
    }
}
