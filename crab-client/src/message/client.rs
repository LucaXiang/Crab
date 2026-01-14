use crate::message::MessageError;
use crate::message::transport::{MemoryTransport, TcpTransport, TlsTransport, Transport};
use shared::message::BusMessage;
use tokio::sync::broadcast;
use rustls::ClientConfig;

/// Message Client
#[derive(Debug, Clone)]
pub struct MessageClient {
    transport: ClientTransport,
}

#[derive(Debug, Clone)]
enum ClientTransport {
    Tcp(TcpTransport),
    Tls(TlsTransport),
    Memory(MemoryTransport),
}

impl MessageClient {
    /// Connect via TCP
    pub async fn connect(addr: &str) -> Result<Self, MessageError> {
        let transport = TcpTransport::connect(addr).await?;
        Ok(Self {
            transport: ClientTransport::Tcp(transport),
        })
    }

    /// Connect via TLS
    pub async fn connect_tls(addr: &str, domain: &str, config: ClientConfig) -> Result<Self, MessageError> {
        let transport = TlsTransport::connect(addr, domain, config).await?;
        Ok(Self {
            transport: ClientTransport::Tls(transport),
        })
    }

    /// Create in-memory client
    pub fn memory(
        server_broadcast_tx: &broadcast::Sender<BusMessage>,
        client_to_server_tx: &broadcast::Sender<BusMessage>,
    ) -> Self {
        let transport = MemoryTransport::new(server_broadcast_tx, client_to_server_tx);
        Self {
            transport: ClientTransport::Memory(transport),
        }
    }

    /// Receive a message
    pub async fn recv(&self) -> Result<BusMessage, MessageError> {
        match &self.transport {
            ClientTransport::Tcp(t) => t.read_message().await,
            ClientTransport::Tls(t) => t.read_message().await,
            ClientTransport::Memory(t) => t.read_message().await,
        }
    }

    /// Send a message
    pub async fn send(&self, msg: &BusMessage) -> Result<(), MessageError> {
        match &self.transport {
            ClientTransport::Tcp(t) => t.write_message(msg).await,
            ClientTransport::Tls(t) => t.write_message(msg).await,
            ClientTransport::Memory(t) => t.write_message(msg).await,
        }
    }

    /// Close connection
    pub async fn close(&self) -> Result<(), MessageError> {
        match &self.transport {
            ClientTransport::Tcp(t) => t.close().await,
            ClientTransport::Tls(t) => t.close().await,
            ClientTransport::Memory(t) => t.close().await,
        }
    }
}
