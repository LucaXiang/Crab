//! Message filtering for client-server architecture
//!
//! In pub-sub, publishers receive their own messages (standard behavior).
//! But for restaurant POS, we want:
//! 1. Client sends → ONLY server receives (not other clients)
//! 2. Server processes → broadcasts to ALL clients
//! 3. Clients DON'T see other clients' messages
//!
//! This requires the server to act as a FILTER.

use crate::message::{BusMessage, EventType};

/// Message source - to filter what gets broadcast
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageSource {
    /// Message from a client (should NOT be broadcast to all)
    Client(String),  // client_id
    /// Message from server (SHOULD be broadcast to all)
    Server,
}

impl BusMessage {
    /// Check if this message should be broadcast to all subscribers
    pub fn should_broadcast(&self) -> bool {
        // Messages from server should be broadcast
        // Messages from clients should NOT be broadcast
        match self.event_type {
            // Server-initiated messages → broadcast to all
            EventType::TableSync | EventType::DataSync | EventType::Notification | EventType::ServerCommand => true,
            // Client-initiated messages → do NOT broadcast
            EventType::TableIntent => false,
        }
    }
}

/// Message bus with filtering
pub struct FilteredMessageBus {
    // In real implementation, this would filter based on source
}

impl FilteredMessageBus {
    /// Publish a message with source information
    pub async fn publish_with_source(&self, msg: BusMessage, source: MessageSource) -> Result<(), String> {
        // If message should be broadcast based on source
        if msg.should_broadcast() {
            // Broadcast to all subscribers
            self.broadcast_to_all(msg).await?;
        } else {
            // Only process locally (server-side)
            self.process_locally(msg).await?;
        }
        Ok(())
    }

    async fn broadcast_to_all(&self, msg: BusMessage) -> Result<(), String> {
        // Broadcast to all clients
        println!("📡 Broadcasting to all clients: {:?}", msg.event_type);
        Ok(())
    }

    async fn process_locally(&self, msg: BusMessage) -> Result<(), String> {
        // Process server-side, don't broadcast
        println!("🔒 Processing locally (not broadcasting): {:?}", msg.event_type);
        Ok(())
    }
}

/// Example usage:
/// ```rust
/// use crate::message::{BusMessage, EventType};
/// use MessageSource::*;
///
/// // Waiter sends OrderIntent
/// let waiter_msg = BusMessage::order_intent(&data);
/// bus.publish_with_source(waiter_msg, Client("waiter_001".to_string())).await;
/// // Output: "Processing locally (not broadcasting)" - Kitchen receives it via server
/// //        Clients DON'T see this message
///
/// // Server broadcasts status update
/// let status_msg = BusMessage::order_sync(&status_data);
/// bus.publish_with_source(status_msg, Server).await;
/// // Output: "Broadcasting to all clients" - All clients receive it
/// ```
