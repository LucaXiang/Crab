use async_trait::async_trait;
use rustls::ClientConfig;
use rustls_pki_types::ServerName;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::{Mutex, broadcast};
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use uuid::Uuid;

use crate::message::MessageError;
use shared::message::{BusMessage, EventType};

/// Transport abstraction for message bus communication
#[async_trait]
pub trait Transport: Send + Sync + std::fmt::Debug {
    async fn read_message(&self) -> Result<BusMessage, MessageError>;
    async fn write_message(&self, msg: &BusMessage) -> Result<(), MessageError>;
    async fn close(&self) -> Result<(), MessageError>;
}

/// TCP Transport Implementation
#[derive(Debug, Clone)]
pub struct TcpTransport {
    reader: Arc<Mutex<OwnedReadHalf>>,
    writer: Arc<Mutex<OwnedWriteHalf>>,
}

impl TcpTransport {
    pub async fn connect(addr: &str) -> Result<Self, MessageError> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| MessageError::Connection(e.to_string()))?;
        let (reader, writer) = stream.into_split();
        Ok(Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
        })
    }
}

#[async_trait]
impl Transport for TcpTransport {
    async fn read_message(&self) -> Result<BusMessage, MessageError> {
        let mut reader = self.reader.lock().await;

        // Read event type (1 byte)
        let mut type_buf = [0u8; 1];
        reader
            .read_exact(&mut type_buf)
            .await
            .map_err(MessageError::Io)?;

        let event_type = EventType::try_from(type_buf[0])
            .map_err(|_| MessageError::InvalidMessage("Invalid event type".into()))?;

        // Read Request ID (16 bytes)
        let mut uuid_buf = [0u8; 16];
        reader
            .read_exact(&mut uuid_buf)
            .await
            .map_err(MessageError::Io)?;
        let request_id = Uuid::from_bytes(uuid_buf);

        // Read Correlation ID (16 bytes)
        let mut correlation_buf = [0u8; 16];
        reader
            .read_exact(&mut correlation_buf)
            .await
            .map_err(MessageError::Io)?;
        let correlation_id_raw = Uuid::from_bytes(correlation_buf);
        let correlation_id = if correlation_id_raw.is_nil() {
            None
        } else {
            Some(correlation_id_raw)
        };

        // Read payload length (4 bytes)
        let mut len_buf = [0u8; 4];
        reader
            .read_exact(&mut len_buf)
            .await
            .map_err(MessageError::Io)?;

        let len = u32::from_le_bytes(len_buf) as usize;

        // Read payload
        let mut payload = vec![0u8; len];
        reader
            .read_exact(&mut payload)
            .await
            .map_err(MessageError::Io)?;

        Ok(BusMessage {
            request_id,
            event_type,
            source: None,
            correlation_id,
            target: None,
            payload,
        })
    }

    async fn write_message(&self, msg: &BusMessage) -> Result<(), MessageError> {
        let mut writer = self.writer.lock().await;
        let mut data = Vec::new();
        data.push(msg.event_type as u8);
        data.extend_from_slice(msg.request_id.as_bytes());

        // Write correlation_id (16 bytes)
        let correlation_bytes = msg.correlation_id.unwrap_or(Uuid::nil()).into_bytes();
        data.extend_from_slice(&correlation_bytes);

        data.extend_from_slice(&(msg.payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&msg.payload);

        writer.write_all(&data).await.map_err(MessageError::Io)?;
        Ok(())
    }

    async fn close(&self) -> Result<(), MessageError> {
        // Dropping the Arc references will eventually close the stream
        Ok(())
    }
}

/// TLS Transport Implementation
#[derive(Debug, Clone)]
pub struct TlsTransport {
    reader: Arc<Mutex<tokio::io::ReadHalf<TlsStream<TcpStream>>>>,
    writer: Arc<Mutex<tokio::io::WriteHalf<TlsStream<TcpStream>>>>,
}

impl TlsTransport {
    pub async fn connect(
        addr: &str,
        domain: &str,
        config: ClientConfig,
    ) -> Result<Self, MessageError> {
        let connector = TlsConnector::from(Arc::new(config));
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| MessageError::Connection(e.to_string()))?;

        let domain = ServerName::try_from(domain)
            .map_err(|e| MessageError::Connection(format!("Invalid domain: {}", e)))?
            .to_owned();

        let stream = connector
            .connect(domain, stream)
            .await
            .map_err(|e| MessageError::Connection(format!("TLS handshake failed: {}", e)))?;

        let (reader, writer) = tokio::io::split(stream);

        Ok(Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
        })
    }
}

#[async_trait]
impl Transport for TlsTransport {
    async fn read_message(&self) -> Result<BusMessage, MessageError> {
        let mut reader = self.reader.lock().await;

        // Read event type (1 byte)
        let mut type_buf = [0u8; 1];
        reader
            .read_exact(&mut type_buf)
            .await
            .map_err(MessageError::Io)?;

        let event_type = EventType::try_from(type_buf[0])
            .map_err(|_| MessageError::InvalidMessage("Invalid event type".into()))?;

        // Read Request ID (16 bytes)
        let mut uuid_buf = [0u8; 16];
        reader
            .read_exact(&mut uuid_buf)
            .await
            .map_err(MessageError::Io)?;
        let request_id = Uuid::from_bytes(uuid_buf);

        // Read Correlation ID (16 bytes)
        let mut correlation_buf = [0u8; 16];
        reader
            .read_exact(&mut correlation_buf)
            .await
            .map_err(MessageError::Io)?;
        let correlation_id_raw = Uuid::from_bytes(correlation_buf);
        let correlation_id = if correlation_id_raw.is_nil() {
            None
        } else {
            Some(correlation_id_raw)
        };

        // Read payload length (4 bytes)
        let mut len_buf = [0u8; 4];
        reader
            .read_exact(&mut len_buf)
            .await
            .map_err(MessageError::Io)?;

        let len = u32::from_le_bytes(len_buf) as usize;

        // Read payload
        let mut payload = vec![0u8; len];
        reader
            .read_exact(&mut payload)
            .await
            .map_err(MessageError::Io)?;

        Ok(BusMessage {
            request_id,
            event_type,
            source: None,
            correlation_id,
            target: None,
            payload,
        })
    }

    async fn write_message(&self, msg: &BusMessage) -> Result<(), MessageError> {
        let mut writer = self.writer.lock().await;
        let mut data = Vec::new();
        data.push(msg.event_type as u8);
        data.extend_from_slice(msg.request_id.as_bytes());

        // Write correlation_id (16 bytes)
        let correlation_bytes = msg.correlation_id.unwrap_or(Uuid::nil()).into_bytes();
        data.extend_from_slice(&correlation_bytes);

        data.extend_from_slice(&(msg.payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&msg.payload);

        writer.write_all(&data).await.map_err(MessageError::Io)?;
        Ok(())
    }

    async fn close(&self) -> Result<(), MessageError> {
        Ok(())
    }
}

/// Memory Transport Implementation (for In-Process communication)
#[derive(Debug, Clone)]
pub struct MemoryTransport {
    /// Receiver for messages FROM server (broadcasts)
    rx: Arc<Mutex<broadcast::Receiver<BusMessage>>>,
    /// Sender for messages TO server
    tx: broadcast::Sender<BusMessage>,
}

impl MemoryTransport {
    /// Create a new memory transport
    ///
    /// # Arguments
    /// * `server_broadcast_tx` - The server's broadcast sender (to subscribe to updates)
    /// * `client_to_server_tx` - The channel to send messages TO the server
    pub fn new(
        server_broadcast_tx: &broadcast::Sender<BusMessage>,
        client_to_server_tx: &broadcast::Sender<BusMessage>,
    ) -> Self {
        Self {
            rx: Arc::new(Mutex::new(server_broadcast_tx.subscribe())),
            tx: client_to_server_tx.clone(),
        }
    }
}

#[async_trait]
impl Transport for MemoryTransport {
    async fn read_message(&self) -> Result<BusMessage, MessageError> {
        let mut rx = self.rx.lock().await;
        rx.recv()
            .await
            .map_err(|e| MessageError::Connection(format!("Memory channel error: {}", e)))
    }

    async fn write_message(&self, msg: &BusMessage) -> Result<(), MessageError> {
        self.tx
            .send(msg.clone())
            .map_err(|e| MessageError::Connection(format!("Failed to send to server: {}", e)))?;
        Ok(())
    }

    async fn close(&self) -> Result<(), MessageError> {
        Ok(())
    }
}
