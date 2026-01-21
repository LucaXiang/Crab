//! Tauri Event definitions for server message forwarding
//!
//! This module defines the event types used to forward Message Bus messages
//! from the Rust backend to the frontend via Tauri Events.

use serde::{Deserialize, Serialize};
use shared::message::{BusMessage, EventType};
use shared::order::OrderEvent;

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

/// Routing information for a BusMessage
///
/// Used to determine how to emit the message to the frontend.
pub enum MessageRoute {
    /// Order event - should be emitted as "order-event"
    OrderEvent(Box<OrderEvent>),
    /// General server message - should be emitted as "server-message"
    ServerMessage(ServerMessageEvent),
}

impl MessageRoute {
    /// Analyze a BusMessage and determine how it should be routed
    ///
    /// Order events (Sync messages with resource="order_event") are extracted
    /// and routed to the "order-event" channel for the order store.
    /// All other messages go to "server-message".
    pub fn from_bus_message(msg: BusMessage) -> Self {
        // Check if this is a Sync message for order events
        if msg.event_type == EventType::Sync {
            // Try to parse as SyncPayload and check resource
            if let Ok(sync_payload) = msg.parse_payload::<shared::message::SyncPayload>() {
                if sync_payload.resource == "order_event" {
                    // Try to extract OrderEvent from data field
                    if let Some(data) = sync_payload.data {
                        if let Ok(order_event) = serde_json::from_value::<OrderEvent>(data) {
                            return MessageRoute::OrderEvent(Box::new(order_event));
                        }
                    }
                }
            }
        }

        // Default: route as server message
        MessageRoute::ServerMessage(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::message::{NotificationPayload, SyncPayload};

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

    #[test]
    fn test_message_route_order_event() {
        use shared::order::{EventPayload, OrderEventType};

        // Create an OrderEvent with all required fields
        let order_event = OrderEvent {
            event_id: "evt-001".to_string(),
            sequence: 1,
            order_id: "order-123".to_string(),
            timestamp: 1705900000000, // Unix milliseconds (server time)
            client_timestamp: None,   // Optional client timestamp for clock skew debugging
            operator_id: "op-001".to_string(),
            operator_name: "Test Operator".to_string(),
            command_id: "cmd-001".to_string(),
            event_type: OrderEventType::TableOpened,
            payload: EventPayload::TableOpened {
                table_id: Some("t-001".to_string()),
                table_name: Some("A1".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 2,
                is_retail: false,
                receipt_number: None,
            },
        };

        // Create a Sync message with resource="order_event" (like edge-server does)
        let sync_payload = SyncPayload {
            resource: "order_event".to_string(),
            version: order_event.sequence,
            action: order_event.event_type.to_string(),
            id: order_event.order_id.clone(),
            data: serde_json::to_value(&order_event).ok(),
        };
        let bus_msg = BusMessage::sync(&sync_payload);

        // Route the message
        let route = MessageRoute::from_bus_message(bus_msg);

        // Should be routed as OrderEvent
        match route {
            MessageRoute::OrderEvent(event) => {
                assert_eq!(event.order_id, "order-123");
                assert_eq!(event.sequence, 1);
            }
            MessageRoute::ServerMessage(_) => {
                panic!("Expected OrderEvent, got ServerMessage");
            }
        }
    }

    #[test]
    fn test_message_route_other_sync() {
        // Create a Sync message with different resource (not order_event)
        let sync_payload = SyncPayload {
            resource: "product".to_string(),
            version: 1,
            action: "updated".to_string(),
            id: "prod-1".to_string(),
            data: Some(serde_json::json!({"name": "Coffee"})),
        };
        let bus_msg = BusMessage::sync(&sync_payload);

        // Route the message
        let route = MessageRoute::from_bus_message(bus_msg);

        // Should be routed as ServerMessage
        match route {
            MessageRoute::ServerMessage(event) => {
                assert_eq!(event.event_type, "sync");
            }
            MessageRoute::OrderEvent(_) => {
                panic!("Expected ServerMessage, got OrderEvent");
            }
        }
    }
}
