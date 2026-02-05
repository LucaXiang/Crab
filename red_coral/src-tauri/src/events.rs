//! Tauri Event definitions for server message forwarding
//!
//! This module defines the event types used to forward Message Bus messages
//! from the Rust backend to the frontend via Tauri Events.

use serde::{Deserialize, Serialize};
use shared::message::{BusMessage, EventType};
use shared::order::{OrderEvent, OrderSnapshot};

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

/// Order sync payload containing event and snapshot (Server Authority Model)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSyncPayload {
    /// The order event (for timeline display)
    pub event: OrderEvent,
    /// Server-computed snapshot (Server Authority - no local computation)
    pub snapshot: OrderSnapshot,
}

/// Routing information for a BusMessage
///
/// Used to determine how to emit the message to the frontend.
pub enum MessageRoute {
    /// Order sync - should be emitted as "order-sync" with event + snapshot
    /// Server Authority Model: frontend uses snapshot directly, no local computation
    OrderSync(Box<OrderSyncPayload>),
    /// General server message - should be emitted as "server-message"
    ServerMessage(ServerMessageEvent),
}

impl MessageRoute {
    /// Analyze a BusMessage and determine how it should be routed
    ///
    /// Order sync messages (resource="order_sync") are extracted with event + snapshot
    /// and routed to the "order-sync" channel for the order store.
    /// All other messages go to "server-message".
    ///
    /// Server Authority Model: snapshot is server-computed, frontend uses it directly.
    pub fn from_bus_message(msg: BusMessage) -> Self {
        // Check if this is a Sync message for order sync
        if msg.event_type == EventType::Sync {
            // Try to parse as SyncPayload and check resource
            if let Ok(sync_payload) = msg.parse_payload::<shared::message::SyncPayload>() {
                if sync_payload.resource == "order_sync" {
                    // Try to extract OrderSyncPayload (event + snapshot) from data field
                    if let Some(data) = sync_payload.data {
                        if let Ok(order_sync) = serde_json::from_value::<OrderSyncPayload>(data) {
                            return MessageRoute::OrderSync(Box::new(order_sync));
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
    fn test_message_route_order_sync() {
        use shared::order::{EventPayload, OrderEventType, OrderStatus};

        // Create an OrderEvent with all required fields
        let order_event = OrderEvent {
            event_id: "evt-001".to_string(),
            sequence: 1,
            order_id: "order-123".to_string(),
            timestamp: 1705900000000,
            client_timestamp: None,
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
                queue_number: None,
                receipt_number: "RCP-001".to_string(),
            },
        };

        // Create a minimal OrderSnapshot
        let order_snapshot = OrderSnapshot {
            order_id: "order-123".to_string(),
            table_id: Some("t-001".to_string()),
            table_name: Some("A1".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 2,
            is_retail: false,
            service_type: None,
            queue_number: None,
            status: OrderStatus::Active,
            items: vec![],
            payments: vec![],
            paid_item_quantities: std::collections::BTreeMap::new(),
            original_total: 0.0,
            subtotal: 0.0,
            total_discount: 0.0,
            total_surcharge: 0.0,
            tax: 0.0,
            discount: 0.0,
            comp_total_amount: 0.0,
            order_manual_discount_amount: 0.0,
            order_manual_surcharge_amount: 0.0,
            total: 0.0,
            paid_amount: 0.0,
            remaining_amount: 0.0,
            receipt_number: "RCP-001".to_string(),
            is_pre_payment: false,
            order_rule_discount_amount: None,
            order_rule_surcharge_amount: None,
            order_applied_rules: None,
            order_manual_discount_percent: None,
            order_manual_discount_fixed: None,
            order_manual_surcharge_percent: None,
            order_manual_surcharge_fixed: None,
            comps: vec![],
            note: None,
            start_time: 1705900000000,
            end_time: None,
            created_at: 1705900000000,
            updated_at: 1705900000000,
            last_sequence: 1,
            state_checksum: String::new(),
            void_type: None,
            loss_reason: None,
            loss_amount: None,
            void_note: None,
            has_amount_split: false,
            aa_total_shares: None,
            aa_paid_shares: 0,
        };

        // Create a Sync message with resource="order_sync" (like edge-server does)
        let sync_payload = SyncPayload {
            resource: "order_sync".to_string(),
            version: order_event.sequence,
            action: order_event.event_type.to_string(),
            id: order_event.order_id.clone(),
            data: Some(serde_json::json!({
                "event": order_event,
                "snapshot": order_snapshot
            })),
        };
        let bus_msg = BusMessage::sync(&sync_payload);

        // Route the message
        let route = MessageRoute::from_bus_message(bus_msg);

        // Should be routed as OrderSync with event + snapshot
        match route {
            MessageRoute::OrderSync(sync) => {
                assert_eq!(sync.event.order_id, "order-123");
                assert_eq!(sync.event.sequence, 1);
                assert_eq!(sync.snapshot.order_id, "order-123");
                assert_eq!(sync.snapshot.guest_count, 2);
            }
            MessageRoute::ServerMessage(_) => {
                panic!("Expected OrderSync, got ServerMessage");
            }
        }
    }

    #[test]
    fn test_message_route_other_sync() {
        // Create a Sync message with different resource (not order_sync)
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
            MessageRoute::OrderSync(_) => {
                panic!("Expected ServerMessage, got OrderSync");
            }
        }
    }
}
