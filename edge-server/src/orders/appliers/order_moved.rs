//! OrderMoved event applier
//!
//! Applies the OrderMoved event to update table and zone information in the snapshot.
//! Updates table_id, table_name, zone_id, and zone_name from the event's target fields.

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

/// OrderMoved applier
pub struct OrderMovedApplier;

impl EventApplier for OrderMovedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::OrderMoved {
            target_table_id,
            target_table_name,
            target_zone_id,
            target_zone_name,
            ..
        } = &event.payload
        {
            // Update table information to the target
            snapshot.table_id = Some(target_table_id.clone());
            snapshot.table_name = Some(target_table_name.clone());

            // Update zone information
            snapshot.zone_id = target_zone_id.clone();
            snapshot.zone_name = target_zone_name.clone();

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
    use shared::order::{CartItemSnapshot, OrderEventType, OrderStatus};

    fn create_test_snapshot(order_id: &str) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.table_id = Some("dining_table:t1".to_string());
        snapshot.table_name = Some("Table 1".to_string());
        snapshot.zone_id = Some("zone:z1".to_string());
        snapshot.zone_name = Some("Zone A".to_string());
        snapshot
    }

    fn create_order_moved_event(
        order_id: &str,
        seq: u64,
        source_table_id: &str,
        source_table_name: &str,
        target_table_id: &str,
        target_table_name: &str,
        items: Vec<CartItemSnapshot>,
    ) -> OrderEvent {
        create_order_moved_event_with_zone(
            order_id,
            seq,
            source_table_id,
            source_table_name,
            target_table_id,
            target_table_name,
            None,
            None,
            items,
        )
    }

    fn create_order_moved_event_with_zone(
        order_id: &str,
        seq: u64,
        source_table_id: &str,
        source_table_name: &str,
        target_table_id: &str,
        target_table_name: &str,
        target_zone_id: Option<String>,
        target_zone_name: Option<String>,
        items: Vec<CartItemSnapshot>,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::OrderMoved,
            EventPayload::OrderMoved {
                source_table_id: source_table_id.to_string(),
                source_table_name: source_table_name.to_string(),
                target_table_id: target_table_id.to_string(),
                target_table_name: target_table_name.to_string(),
                target_zone_id,
                target_zone_name,
                items,
                authorizer_id: None,
                authorizer_name: None,
            },
        )
    }

    #[test]
    fn test_order_moved_updates_table_info() {
        let mut snapshot = create_test_snapshot("order-1");
        assert_eq!(snapshot.table_id, Some("dining_table:t1".to_string()));
        assert_eq!(snapshot.table_name, Some("Table 1".to_string()));

        let event = create_order_moved_event(
            "order-1",
            2,
            "dining_table:t1",
            "Table 1",
            "dining_table:t2",
            "Table 2",
            vec![],
        );

        let applier = OrderMovedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.table_id, Some("dining_table:t2".to_string()));
        assert_eq!(snapshot.table_name, Some("Table 2".to_string()));
    }

    #[test]
    fn test_order_moved_updates_zone_info() {
        let mut snapshot = create_test_snapshot("order-1");
        assert_eq!(snapshot.zone_id, Some("zone:z1".to_string()));
        assert_eq!(snapshot.zone_name, Some("Zone A".to_string()));

        let event = create_order_moved_event_with_zone(
            "order-1",
            2,
            "dining_table:t1",
            "Table 1",
            "dining_table:t3",
            "Table 3",
            Some("zone:z2".to_string()),
            Some("Zone B".to_string()),
            vec![],
        );

        let applier = OrderMovedApplier;
        applier.apply(&mut snapshot, &event);

        // Zone info should be updated to the target zone
        assert_eq!(snapshot.zone_id, Some("zone:z2".to_string()));
        assert_eq!(snapshot.zone_name, Some("Zone B".to_string()));
    }

    #[test]
    fn test_order_moved_clears_zone_when_none() {
        let mut snapshot = create_test_snapshot("order-1");
        assert_eq!(snapshot.zone_id, Some("zone:z1".to_string()));

        // Move without zone info → zone should be cleared
        let event = create_order_moved_event(
            "order-1",
            2,
            "dining_table:t1",
            "Table 1",
            "dining_table:t3",
            "Table 3",
            vec![],
        );

        let applier = OrderMovedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.zone_id, None);
        assert_eq!(snapshot.zone_name, None);
    }

    #[test]
    fn test_order_moved_updates_sequence() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.last_sequence = 5;

        let event = create_order_moved_event(
            "order-1",
            10,
            "dining_table:t1",
            "Table 1",
            "dining_table:t2",
            "Table 2",
            vec![],
        );

        let applier = OrderMovedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 10);
    }

    #[test]
    fn test_order_moved_updates_timestamp() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.updated_at = 1000000000;

        let event = create_order_moved_event(
            "order-1",
            2,
            "dining_table:t1",
            "Table 1",
            "dining_table:t2",
            "Table 2",
            vec![],
        );
        let expected_timestamp = event.timestamp;

        let applier = OrderMovedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.updated_at, expected_timestamp);
        assert_ne!(snapshot.updated_at, 1000000000);
    }

    #[test]
    fn test_order_moved_updates_checksum() {
        let mut snapshot = create_test_snapshot("order-1");
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_order_moved_event(
            "order-1",
            2,
            "dining_table:t1",
            "Table 1",
            "dining_table:t2",
            "Table 2",
            vec![],
        );

        let applier = OrderMovedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_order_moved_with_empty_source_table() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.table_id = None;
        snapshot.table_name = None;

        let event = create_order_moved_event("order-1", 2, "", "", "dining_table:t2", "Table 2", vec![]);

        let applier = OrderMovedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.table_id, Some("dining_table:t2".to_string()));
        assert_eq!(snapshot.table_name, Some("Table 2".to_string()));
    }

    #[test]
    fn test_order_moved_preserves_items() {
        let mut snapshot = create_test_snapshot("order-1");
        let item = CartItemSnapshot {
            id: "product:1".to_string(),
            instance_id: "item-1".to_string(),
            name: "Coffee".to_string(),
            price: 10.0,
            original_price: None,
            quantity: 2,
            unpaid_quantity: 2,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            surcharge: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
        };
        snapshot.items.push(item.clone());

        let event = create_order_moved_event(
            "order-1",
            2,
            "dining_table:t1",
            "Table 1",
            "dining_table:t2",
            "Table 2",
            vec![item],
        );

        let applier = OrderMovedApplier;
        applier.apply(&mut snapshot, &event);

        // Items in snapshot should remain unchanged (applier doesn't modify items)
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "item-1");
    }

    #[test]
    fn test_order_moved_preserves_status() {
        let mut snapshot = create_test_snapshot("order-1");
        assert_eq!(snapshot.status, OrderStatus::Active);

        let event = create_order_moved_event(
            "order-1",
            2,
            "dining_table:t1",
            "Table 1",
            "dining_table:t2",
            "Table 2",
            vec![],
        );

        let applier = OrderMovedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.status, OrderStatus::Active);
    }

    #[test]
    fn test_order_moved_wrong_event_type_is_noop() {
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

        let applier = OrderMovedApplier;
        applier.apply(&mut snapshot, &event);

        // Nothing should change
        assert_eq!(snapshot.table_id, original_table_id);
        assert_eq!(snapshot.last_sequence, original_sequence);
    }

    #[test]
    fn test_order_moved_checksum_verifiable() {
        let mut snapshot = create_test_snapshot("order-1");

        let event = create_order_moved_event(
            "order-1",
            2,
            "dining_table:t1",
            "Table 1",
            "dining_table:t2",
            "Table 2",
            vec![],
        );

        let applier = OrderMovedApplier;
        applier.apply(&mut snapshot, &event);

        assert!(snapshot.verify_checksum());

        // Tampering should invalidate checksum
        snapshot.last_sequence = 999;
        assert!(!snapshot.verify_checksum());
    }

    #[test]
    fn test_order_moved_to_same_table() {
        let mut snapshot = create_test_snapshot("order-1");

        let event = create_order_moved_event(
            "order-1",
            2,
            "dining_table:t1",
            "Table 1",
            "dining_table:t1", // Same table
            "Table 1",
            vec![],
        );

        let applier = OrderMovedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.table_id, Some("dining_table:t1".to_string()));
        assert_eq!(snapshot.table_name, Some("Table 1".to_string()));
        assert_eq!(snapshot.last_sequence, 2);
    }

    #[test]
    fn test_order_moved_empty_target_table_name() {
        let mut snapshot = create_test_snapshot("order-1");

        let event =
            create_order_moved_event("order-1", 2, "dining_table:t1", "Table 1", "dining_table:t2", "", vec![]);

        let applier = OrderMovedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.table_id, Some("dining_table:t2".to_string()));
        assert_eq!(snapshot.table_name, Some("".to_string()));
    }
}
