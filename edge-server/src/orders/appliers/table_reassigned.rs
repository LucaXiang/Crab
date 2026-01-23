//! TableReassigned event applier
//!
//! Updates table and zone information when an order is reassigned to a different table.

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

/// TableReassigned applier
///
/// Updates table_id, table_name, and optionally zone_name when the order
/// is reassigned to a different table.
pub struct TableReassignedApplier;

impl EventApplier for TableReassignedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::TableReassigned {
            target_table_id,
            target_table_name,
            target_zone_name,
            ..
        } = &event.payload
        {
            // Update table information to the target
            snapshot.table_id = Some(target_table_id.clone());
            snapshot.table_name = Some(target_table_name.clone());

            // Update zone name if provided
            if let Some(zone_name) = target_zone_name {
                snapshot.zone_name = Some(zone_name.clone());
            }

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
    use shared::order::{OrderEventType, OrderStatus};

    fn create_test_snapshot(order_id: &str) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.table_id = Some("table-1".to_string());
        snapshot.table_name = Some("Table 1".to_string());
        snapshot.zone_id = Some("zone-1".to_string());
        snapshot.zone_name = Some("Zone A".to_string());
        snapshot
    }

    fn create_table_reassigned_event(
        order_id: &str,
        seq: u64,
        source_table_id: &str,
        source_table_name: &str,
        target_table_id: &str,
        target_table_name: &str,
        target_zone_name: Option<&str>,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::TableReassigned,
            EventPayload::TableReassigned {
                source_table_id: source_table_id.to_string(),
                source_table_name: source_table_name.to_string(),
                target_table_id: target_table_id.to_string(),
                target_table_name: target_table_name.to_string(),
                target_zone_name: target_zone_name.map(|s| s.to_string()),
                original_start_time: 1234567890,
                items: vec![],
            },
        )
    }

    #[test]
    fn test_table_reassigned_updates_table_info() {
        let mut snapshot = create_test_snapshot("order-1");
        assert_eq!(snapshot.table_id, Some("table-1".to_string()));
        assert_eq!(snapshot.table_name, Some("Table 1".to_string()));

        let event = create_table_reassigned_event(
            "order-1",
            2,
            "table-1",
            "Table 1",
            "table-2",
            "Table 2",
            None,
        );

        let applier = TableReassignedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.table_id, Some("table-2".to_string()));
        assert_eq!(snapshot.table_name, Some("Table 2".to_string()));
    }

    #[test]
    fn test_table_reassigned_updates_zone_name() {
        let mut snapshot = create_test_snapshot("order-1");
        assert_eq!(snapshot.zone_name, Some("Zone A".to_string()));

        let event = create_table_reassigned_event(
            "order-1",
            2,
            "table-1",
            "Table 1",
            "table-2",
            "Table 2",
            Some("Zone B"),
        );

        let applier = TableReassignedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.zone_name, Some("Zone B".to_string()));
    }

    #[test]
    fn test_table_reassigned_preserves_zone_when_not_provided() {
        let mut snapshot = create_test_snapshot("order-1");
        let original_zone_name = snapshot.zone_name.clone();

        let event = create_table_reassigned_event(
            "order-1",
            2,
            "table-1",
            "Table 1",
            "table-2",
            "Table 2",
            None, // No zone change
        );

        let applier = TableReassignedApplier;
        applier.apply(&mut snapshot, &event);

        // Zone name should remain unchanged
        assert_eq!(snapshot.zone_name, original_zone_name);
    }

    #[test]
    fn test_table_reassigned_updates_sequence() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.last_sequence = 5;

        let event = create_table_reassigned_event(
            "order-1",
            10,
            "table-1",
            "Table 1",
            "table-2",
            "Table 2",
            None,
        );

        let applier = TableReassignedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 10);
    }

    #[test]
    fn test_table_reassigned_updates_timestamp() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.updated_at = 1000000000;

        let event = create_table_reassigned_event(
            "order-1",
            2,
            "table-1",
            "Table 1",
            "table-2",
            "Table 2",
            None,
        );
        let expected_timestamp = event.timestamp;

        let applier = TableReassignedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.updated_at, expected_timestamp);
    }

    #[test]
    fn test_table_reassigned_updates_checksum() {
        let mut snapshot = create_test_snapshot("order-1");
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_table_reassigned_event(
            "order-1",
            2,
            "table-1",
            "Table 1",
            "table-2",
            "Table 2",
            None,
        );

        let applier = TableReassignedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_table_reassigned_preserves_status() {
        let mut snapshot = create_test_snapshot("order-1");
        assert_eq!(snapshot.status, OrderStatus::Active);

        let event = create_table_reassigned_event(
            "order-1",
            2,
            "table-1",
            "Table 1",
            "table-2",
            "Table 2",
            None,
        );

        let applier = TableReassignedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.status, OrderStatus::Active);
    }

    #[test]
    fn test_table_reassigned_wrong_event_type_is_noop() {
        let mut snapshot = create_test_snapshot("order-1");
        let original_table_id = snapshot.table_id.clone();
        let original_sequence = snapshot.last_sequence;

        // Create an event with wrong payload type
        let event = OrderEvent::new(
            2,
            "order-1".to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::OrderCompleted,
            EventPayload::OrderCompleted {
                receipt_number: "R-001".to_string(),
                final_total: 100.0,
                payment_summary: vec![],
            },
        );

        let applier = TableReassignedApplier;
        applier.apply(&mut snapshot, &event);

        // Nothing should change
        assert_eq!(snapshot.table_id, original_table_id);
        assert_eq!(snapshot.last_sequence, original_sequence);
    }
}
