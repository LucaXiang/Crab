//! Tauri Event definitions for server message forwarding
//!
//! This module defines the event types used to forward Message Bus messages
//! from the Rust backend to the frontend via Tauri Events.

use serde::{Deserialize, Serialize};
use shared::message::BusMessage;

/// Server message event for Tauri
///
/// This is a frontend-friendly representation of BusMessage that uses
/// string event types and JSON values instead of raw bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMessageEvent {
    /// Event type as string (e.g., "notification", "sync", "server_command")
    pub event_type: String,
    /// Payload as JSON value for easy frontend consumption
    pub payload: serde_json::Value,
    /// Correlation ID for RPC response matching (if applicable)
    pub correlation_id: Option<String>,
}

impl From<BusMessage> for ServerMessageEvent {
    fn from(msg: BusMessage) -> Self {
        // Parse payload bytes to JSON value, fallback to null on error
        let payload = serde_json::from_slice(&msg.payload).unwrap_or(serde_json::Value::Null);

        Self {
            event_type: msg.event_type.to_string(),
            payload,
            correlation_id: msg.correlation_id.map(|id| id.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::message::NotificationPayload;

    #[test]
    fn test_server_message_event_from_bus_message() {
        let notification = NotificationPayload::info("Test", "Hello World");
        let bus_msg = BusMessage::notification(&notification);

        let event: ServerMessageEvent = bus_msg.into();

        assert_eq!(event.event_type, "notification");
        assert!(event.correlation_id.is_none());

        // Verify payload contains expected fields
        let payload_obj = event.payload.as_object().unwrap();
        assert_eq!(payload_obj.get("title").unwrap().as_str().unwrap(), "Test");
        assert_eq!(
            payload_obj.get("message").unwrap().as_str().unwrap(),
            "Hello World"
        );
    }
}
