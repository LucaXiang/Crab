//! Simple message client for Event Bus communication
//!
//! Provides a clean `send`/`recv` interface, hiding transport details.
//! Supports both TCP transport and in-memory transport.
//!
//! # Examples
//!
//! ## TCP client (network)
//!
//! ```ignore
//! use edge_server::MessageClient;
//!
//! let client = MessageClient::connect("127.0.0.1:8081").await?;
//! let payload = shared::message::NotificationPayload::info("Test", "Hello");
//! let msg = BusMessage::notification(&payload);
//! client.send(&msg).await?;
//! let received = client.recv().await?;
//! ```
//!
//! ## Memory client (in-process testing)
//!
//! ```ignore
//! use edge_server::{MessageClient, ServerState};
//!
//! let state = ServerState::initialize(&config).await;
//! state.start_background_tasks().await; // Start message processing
//! let client = MessageClient::memory(&state.get_message_bus());
//! let payload = shared::message::NotificationPayload::info("Test", "Hello");
//! let msg = BusMessage::notification(&payload);
//! client.send(&msg).await?;
//! let received = client.recv().await?;
//! ```

use crate::common::AppError;
use crate::message::{BusMessage, MemoryTransport, TcpTransport};

/// Simple message client with send/recv interface
///
/// Wraps transport to provide a clean API for message bus communication.
#[derive(Debug, Clone)]
pub struct MessageClient {
    transport: MessageClientTransport,
}

#[derive(Debug, Clone)]
enum MessageClientTransport {
    Tcp(TcpTransport),
    Memory(MemoryTransport),
}

impl MessageClient {
    /// Connect to message bus via TCP
    pub async fn connect(addr: &str) -> Result<Self, AppError> {
        let transport = TcpTransport::connect(addr).await?;
        Ok(Self {
            transport: MessageClientTransport::Tcp(transport),
        })
    }

    /// Create in-memory client for same-process communication
    pub fn memory(bus: &crate::message::MessageBus) -> Self {
        Self {
            transport: MessageClientTransport::Memory(bus.client_memory_transport()),
        }
    }

    /// Send a message to the server
    pub async fn send(&self, msg: &BusMessage) -> Result<(), AppError> {
        match &self.transport {
            MessageClientTransport::Tcp(t) => t.write_message(msg).await,
            MessageClientTransport::Memory(m) => m.write_message(msg).await,
        }
    }

    /// Receive a message from the server
    pub async fn recv(&self) -> Result<BusMessage, AppError> {
        match &self.transport {
            MessageClientTransport::Tcp(t) => t.read_message().await,
            MessageClientTransport::Memory(m) => m.read_message().await,
        }
    }

    /// Close the client connection
    pub async fn close(&self) -> Result<(), AppError> {
        match &self.transport {
            MessageClientTransport::Tcp(t) => t.close().await,
            MessageClientTransport::Memory(_) => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageBus;

    #[tokio::test]
    async fn test_message_client_tcp() {
        // This would require a running TCP server
        // Skip for now as we don't have a test server
    }

    #[tokio::test]
    async fn test_message_client_memory() {
        let bus = MessageBus::new();
        let mut server_rx = bus.subscribe_to_clients(); // Simulate server listening
        let client = MessageClient::memory(&bus);

        // Send a message
        let payload = shared::message::NotificationPayload::info("Test", "Hello");
        let msg = BusMessage::notification(&payload);
        client.send(&msg).await.unwrap();

        // Verify server received it
        let received_by_server = server_rx.recv().await.unwrap();
        assert_eq!(
            received_by_server.event_type,
            crate::EventType::Notification
        );

        // Simulate server broadcasting the message back
        bus.publish(msg).await.unwrap();

        // Receive it
        let received = client.recv().await.unwrap();
        assert_eq!(received.event_type, crate::EventType::Notification);
    }

    #[tokio::test]
    async fn test_message_client_close() {
        let bus = MessageBus::new();
        let client = MessageClient::memory(&bus);
        client.close().await.unwrap();
    }
}
