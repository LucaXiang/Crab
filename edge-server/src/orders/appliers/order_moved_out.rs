//! OrderMovedOut event applier
//!
//! Marks the source order as Moved status when items have been moved to another table.

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot, OrderStatus};

/// OrderMovedOut applier - applies to the source order
///
/// Marks the source order as Moved status when items have been transferred out.
pub struct OrderMovedOutApplier;

impl EventApplier for OrderMovedOutApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::OrderMovedOut { .. } = &event.payload {
            // Mark order as moved
            snapshot.status = OrderStatus::Moved;

            // Update sequence and timestamp
            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;

            // Update checksum
            snapshot.update_checksum();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::types::ServiceType;
    use shared::order::OrderEventType;

    fn create_test_snapshot(order_id: &str) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.table_id = Some("dining_table:t1".to_string());
        snapshot.table_name = Some("Table 1".to_string());
        snapshot
    }

    fn create_order_moved_out_event(
        order_id: &str,
        seq: u64,
        target_table_id: &str,
        target_table_name: &str,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::OrderMovedOut,
            EventPayload::OrderMovedOut {
                target_table_id: target_table_id.to_string(),
                target_table_name: target_table_name.to_string(),
                reason: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        )
    }

    #[test]
    fn test_order_moved_out_sets_status() {
        let mut snapshot = create_test_snapshot("source-1");
        assert_eq!(snapshot.status, OrderStatus::Active);

        let event = create_order_moved_out_event("source-1", 2, "dining_table:t2", "Table 2");

        let applier = OrderMovedOutApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.status, OrderStatus::Moved);
    }

    #[test]
    fn test_order_moved_out_updates_sequence() {
        let mut snapshot = create_test_snapshot("source-1");
        snapshot.last_sequence = 5;

        let event = create_order_moved_out_event("source-1", 10, "dining_table:t2", "Table 2");

        let applier = OrderMovedOutApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 10);
    }

    #[test]
    fn test_order_moved_out_updates_timestamp() {
        let mut snapshot = create_test_snapshot("source-1");
        snapshot.updated_at = 1000000000;

        let event = create_order_moved_out_event("source-1", 2, "dining_table:t2", "Table 2");
        let expected_timestamp = event.timestamp;

        let applier = OrderMovedOutApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.updated_at, expected_timestamp);
    }

    #[test]
    fn test_order_moved_out_updates_checksum() {
        let mut snapshot = create_test_snapshot("source-1");
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_order_moved_out_event("source-1", 2, "dining_table:t2", "Table 2");

        let applier = OrderMovedOutApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_order_moved_out_preserves_table_info() {
        let mut snapshot = create_test_snapshot("source-1");

        let event = create_order_moved_out_event("source-1", 2, "dining_table:t2", "Table 2");

        let applier = OrderMovedOutApplier;
        applier.apply(&mut snapshot, &event);

        // Source table info should be preserved
        assert_eq!(snapshot.table_id, Some("dining_table:t1".to_string()));
        assert_eq!(snapshot.table_name, Some("Table 1".to_string()));
    }

    #[test]
    fn test_order_moved_out_wrong_event_type_is_noop() {
        let mut snapshot = create_test_snapshot("source-1");
        let original_status = snapshot.status;
        let original_sequence = snapshot.last_sequence;

        // Create an event with wrong payload type
        let event = OrderEvent::new(
            2,
            "source-1".to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::OrderCompleted,
            EventPayload::OrderCompleted {
                receipt_number: "R-001".to_string(),
                service_type: Some(ServiceType::DineIn),
                final_total: 100.0,
                payment_summary: vec![],
            },
        );

        let applier = OrderMovedOutApplier;
        applier.apply(&mut snapshot, &event);

        // Nothing should change
        assert_eq!(snapshot.status, original_status);
        assert_eq!(snapshot.last_sequence, original_sequence);
    }
}
