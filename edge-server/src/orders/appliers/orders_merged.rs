//! OrderMerged and OrderMergedOut event appliers
//!
//! Handles the merge operation for both orders:
//! - OrderMerged: Target order receives items from source order
//! - OrderMergedOut: Source order is marked as Merged status

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot, OrderStatus};

/// OrderMerged applier - applies to the target order
///
/// Adds items from the source order to the target order's items list.
pub struct OrderMergedApplier;

impl EventApplier for OrderMergedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::OrderMerged { items, .. } = &event.payload {
            // Add merged items to the target order
            for item in items {
                snapshot.items.push(item.clone());
            }

            // Update sequence and timestamp
            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;

            // Update checksum
            snapshot.update_checksum();
        }
    }
}

/// OrderMergedOut applier - applies to the source order
///
/// Marks the source order as Merged status.
pub struct OrderMergedOutApplier;

impl EventApplier for OrderMergedOutApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::OrderMergedOut { .. } = &event.payload {
            // Mark order as merged
            snapshot.status = OrderStatus::Merged;

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
    use shared::order::{CartItemSnapshot, OrderEventType};

    fn create_test_snapshot(order_id: &str) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.table_id = Some("table-1".to_string());
        snapshot.table_name = Some("Table 1".to_string());
        snapshot
    }

    fn create_test_item(instance_id: &str, name: &str) -> CartItemSnapshot {
        CartItemSnapshot {
            id: "product-1".to_string(),
            instance_id: instance_id.to_string(),
            name: name.to_string(),
            price: 10.0,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            discount_percent: None,
            surcharge: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        }
    }

    fn create_order_merged_event(
        order_id: &str,
        seq: u64,
        source_table_id: &str,
        source_table_name: &str,
        items: Vec<CartItemSnapshot>,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::OrderMerged,
            EventPayload::OrderMerged {
                source_table_id: source_table_id.to_string(),
                source_table_name: source_table_name.to_string(),
                items,
            },
        )
    }

    fn create_order_merged_out_event(
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
            OrderEventType::OrderMergedOut,
            EventPayload::OrderMergedOut {
                target_table_id: target_table_id.to_string(),
                target_table_name: target_table_name.to_string(),
                reason: None,
            },
        )
    }

    // ==================== OrderMergedApplier Tests ====================

    #[test]
    fn test_order_merged_adds_items() {
        let mut snapshot = create_test_snapshot("target-1");
        assert!(snapshot.items.is_empty());

        let items = vec![
            create_test_item("item-1", "Coffee"),
            create_test_item("item-2", "Tea"),
        ];

        let event = create_order_merged_event("target-1", 2, "table-2", "Table 2", items);

        let applier = OrderMergedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items.len(), 2);
        assert_eq!(snapshot.items[0].instance_id, "item-1");
        assert_eq!(snapshot.items[0].name, "Coffee");
        assert_eq!(snapshot.items[1].instance_id, "item-2");
        assert_eq!(snapshot.items[1].name, "Tea");
    }

    #[test]
    fn test_order_merged_appends_to_existing_items() {
        let mut snapshot = create_test_snapshot("target-1");
        snapshot
            .items
            .push(create_test_item("existing-1", "Existing Item"));

        let items = vec![create_test_item("merged-1", "Merged Item")];

        let event = create_order_merged_event("target-1", 2, "table-2", "Table 2", items);

        let applier = OrderMergedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items.len(), 2);
        assert_eq!(snapshot.items[0].instance_id, "existing-1");
        assert_eq!(snapshot.items[1].instance_id, "merged-1");
    }

    #[test]
    fn test_order_merged_empty_items() {
        let mut snapshot = create_test_snapshot("target-1");

        let event = create_order_merged_event("target-1", 2, "table-2", "Table 2", vec![]);

        let applier = OrderMergedApplier;
        applier.apply(&mut snapshot, &event);

        assert!(snapshot.items.is_empty());
    }

    #[test]
    fn test_order_merged_updates_sequence() {
        let mut snapshot = create_test_snapshot("target-1");
        snapshot.last_sequence = 5;

        let event = create_order_merged_event("target-1", 10, "table-2", "Table 2", vec![]);

        let applier = OrderMergedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 10);
    }

    #[test]
    fn test_order_merged_updates_timestamp() {
        let mut snapshot = create_test_snapshot("target-1");
        snapshot.updated_at = 1000000000;

        let event = create_order_merged_event("target-1", 2, "table-2", "Table 2", vec![]);
        let expected_timestamp = event.timestamp;

        let applier = OrderMergedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.updated_at, expected_timestamp);
    }

    #[test]
    fn test_order_merged_updates_checksum() {
        let mut snapshot = create_test_snapshot("target-1");
        let initial_checksum = snapshot.state_checksum.clone();

        let items = vec![create_test_item("item-1", "Coffee")];
        let event = create_order_merged_event("target-1", 2, "table-2", "Table 2", items);

        let applier = OrderMergedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_order_merged_preserves_status() {
        let mut snapshot = create_test_snapshot("target-1");
        assert_eq!(snapshot.status, OrderStatus::Active);

        let event = create_order_merged_event("target-1", 2, "table-2", "Table 2", vec![]);

        let applier = OrderMergedApplier;
        applier.apply(&mut snapshot, &event);

        // Status should remain Active (target order is still active)
        assert_eq!(snapshot.status, OrderStatus::Active);
    }

    #[test]
    fn test_order_merged_preserves_table_info() {
        let mut snapshot = create_test_snapshot("target-1");
        snapshot.table_id = Some("target-table".to_string());
        snapshot.table_name = Some("Target Table".to_string());

        let event =
            create_order_merged_event("target-1", 2, "source-table", "Source Table", vec![]);

        let applier = OrderMergedApplier;
        applier.apply(&mut snapshot, &event);

        // Target table info should be preserved
        assert_eq!(snapshot.table_id, Some("target-table".to_string()));
        assert_eq!(snapshot.table_name, Some("Target Table".to_string()));
    }

    #[test]
    fn test_order_merged_wrong_event_type_is_noop() {
        let mut snapshot = create_test_snapshot("target-1");
        let original_items_len = snapshot.items.len();
        let original_sequence = snapshot.last_sequence;

        // Create an event with wrong payload type
        let event = OrderEvent::new(
            2,
            "target-1".to_string(),
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

        let applier = OrderMergedApplier;
        applier.apply(&mut snapshot, &event);

        // Nothing should change
        assert_eq!(snapshot.items.len(), original_items_len);
        assert_eq!(snapshot.last_sequence, original_sequence);
    }

    // ==================== OrderMergedOutApplier Tests ====================

    #[test]
    fn test_order_merged_out_sets_status() {
        let mut snapshot = create_test_snapshot("source-1");
        assert_eq!(snapshot.status, OrderStatus::Active);

        let event = create_order_merged_out_event("source-1", 2, "table-2", "Table 2");

        let applier = OrderMergedOutApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.status, OrderStatus::Merged);
    }

    #[test]
    fn test_order_merged_out_updates_sequence() {
        let mut snapshot = create_test_snapshot("source-1");
        snapshot.last_sequence = 5;

        let event = create_order_merged_out_event("source-1", 10, "table-2", "Table 2");

        let applier = OrderMergedOutApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 10);
    }

    #[test]
    fn test_order_merged_out_updates_timestamp() {
        let mut snapshot = create_test_snapshot("source-1");
        snapshot.updated_at = 1000000000;

        let event = create_order_merged_out_event("source-1", 2, "table-2", "Table 2");
        let expected_timestamp = event.timestamp;

        let applier = OrderMergedOutApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.updated_at, expected_timestamp);
    }

    #[test]
    fn test_order_merged_out_updates_checksum() {
        let mut snapshot = create_test_snapshot("source-1");
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_order_merged_out_event("source-1", 2, "table-2", "Table 2");

        let applier = OrderMergedOutApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_order_merged_out_preserves_items() {
        let mut snapshot = create_test_snapshot("source-1");
        snapshot.items.push(create_test_item("item-1", "Coffee"));

        let event = create_order_merged_out_event("source-1", 2, "table-2", "Table 2");

        let applier = OrderMergedOutApplier;
        applier.apply(&mut snapshot, &event);

        // Items should be preserved (not cleared)
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "item-1");
    }

    #[test]
    fn test_order_merged_out_preserves_table_info() {
        let mut snapshot = create_test_snapshot("source-1");

        let event = create_order_merged_out_event("source-1", 2, "table-2", "Table 2");

        let applier = OrderMergedOutApplier;
        applier.apply(&mut snapshot, &event);

        // Source table info should be preserved
        assert_eq!(snapshot.table_id, Some("table-1".to_string()));
        assert_eq!(snapshot.table_name, Some("Table 1".to_string()));
    }

    #[test]
    fn test_order_merged_out_wrong_event_type_is_noop() {
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
                final_total: 100.0,
                payment_summary: vec![],
            },
        );

        let applier = OrderMergedOutApplier;
        applier.apply(&mut snapshot, &event);

        // Nothing should change
        assert_eq!(snapshot.status, original_status);
        assert_eq!(snapshot.last_sequence, original_sequence);
    }

    #[test]
    fn test_order_merged_out_checksum_verifiable() {
        let mut snapshot = create_test_snapshot("source-1");

        let event = create_order_merged_out_event("source-1", 2, "table-2", "Table 2");

        let applier = OrderMergedOutApplier;
        applier.apply(&mut snapshot, &event);

        assert!(snapshot.verify_checksum());

        // Tampering should invalidate checksum
        snapshot.last_sequence = 999;
        assert!(!snapshot.verify_checksum());
    }

    #[test]
    fn test_order_merged_checksum_verifiable() {
        let mut snapshot = create_test_snapshot("target-1");

        let items = vec![create_test_item("item-1", "Coffee")];
        let event = create_order_merged_event("target-1", 2, "table-2", "Table 2", items);

        let applier = OrderMergedApplier;
        applier.apply(&mut snapshot, &event);

        assert!(snapshot.verify_checksum());

        // Tampering should invalidate checksum
        snapshot.last_sequence = 999;
        assert!(!snapshot.verify_checksum());
    }
}
