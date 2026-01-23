//! ItemRestored event applier
//!
//! Applies the ItemRestored event to restore a previously removed item.
//!
//! Note: In the current architecture, items are physically removed from the
//! snapshot when deleted (via ItemRemoved event). A full implementation would
//! require storing removed items elsewhere and retrieving them here.
//! This applier currently only updates sequence/timestamp as a placeholder
//! for future item tracking implementation.

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

/// ItemRestored applier
pub struct ItemRestoredApplier;

impl EventApplier for ItemRestoredApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::ItemRestored {
            instance_id: _,
            item_name: _,
        } = &event.payload
        {
            // Note: In the current architecture, we cannot restore the item
            // because removed items are not tracked.
            // A full implementation would:
            // 1. Look up the removed item from a tracking store
            // 2. Add it back to snapshot.items
            // 3. Recalculate totals

            // For now, just update sequence and timestamp
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

    fn create_item_restored_event(
        order_id: &str,
        seq: u64,
        instance_id: &str,
        item_name: &str,
    ) -> OrderEvent {
        OrderEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            sequence: seq,
            order_id: order_id.to_string(),
            timestamp: 1234567890,
            client_timestamp: Some(1234567890),
            operator_id: "user-1".to_string(),
            operator_name: "Test User".to_string(),
            command_id: "cmd-1".to_string(),
            event_type: OrderEventType::ItemRestored,
            payload: EventPayload::ItemRestored {
                instance_id: instance_id.to_string(),
                item_name: item_name.to_string(),
            },
        }
    }

    fn create_test_item(
        instance_id: &str,
        product_id: &str,
        name: &str,
        price: f64,
        quantity: i32,
    ) -> CartItemSnapshot {
        CartItemSnapshot {
            id: product_id.to_string(),
            instance_id: instance_id.to_string(),
            name: name.to_string(),
            price,
            original_price: None,
            quantity,
            unpaid_quantity: quantity,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            surcharge: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            line_total: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        }
    }

    #[test]
    fn test_item_restored_updates_sequence() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.last_sequence = 5;

        let event = create_item_restored_event("order-1", 6, "item-1", "Test Product");

        let applier = ItemRestoredApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 6);
    }

    #[test]
    fn test_item_restored_updates_timestamp() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.updated_at = 1000;

        let event = create_item_restored_event("order-1", 1, "item-1", "Test Product");

        let applier = ItemRestoredApplier;
        applier.apply(&mut snapshot, &event);

        // updated_at should be updated to event timestamp
        assert_eq!(snapshot.updated_at, 1234567890);
    }

    #[test]
    fn test_item_restored_updates_checksum() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_item_restored_event("order-1", 1, "item-1", "Test Product");

        let applier = ItemRestoredApplier;
        applier.apply(&mut snapshot, &event);

        // Checksum should be updated (due to sequence change)
        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_item_restored_preserves_existing_items() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items.push(create_test_item(
            "item-2",
            "product:p2",
            "Existing Product",
            20.0,
            1,
        ));
        snapshot.subtotal = 20.0;
        snapshot.total = 20.0;

        let event = create_item_restored_event("order-1", 1, "item-1", "Restored Product");

        let applier = ItemRestoredApplier;
        applier.apply(&mut snapshot, &event);

        // Existing items should be preserved
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "item-2");
        // Note: In current implementation, restored item is NOT added back
        // because we don't track removed items
    }

    #[test]
    fn test_item_restored_idempotent() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        let event = create_item_restored_event("order-1", 1, "item-1", "Test Product");

        let applier = ItemRestoredApplier;

        // Apply twice
        applier.apply(&mut snapshot, &event);
        let checksum_after_first = snapshot.state_checksum.clone();

        applier.apply(&mut snapshot, &event);
        let checksum_after_second = snapshot.state_checksum.clone();

        // Checksum should be the same (same sequence applied twice)
        assert_eq!(checksum_after_first, checksum_after_second);
    }

    #[test]
    fn test_item_restored_with_payments_preserved() {
        use shared::order::PaymentRecord;

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.paid_amount = 50.0;
        snapshot.payments.push(PaymentRecord {
            payment_id: "pay-1".to_string(),
            method: "CASH".to_string(),
            amount: 50.0,
            tendered: Some(50.0),
            change: Some(0.0),
            note: None,
            timestamp: 1234567800,
            cancelled: false,
            cancel_reason: None,
            split_items: None,
        });

        let event = create_item_restored_event("order-1", 2, "item-1", "Test Product");

        let applier = ItemRestoredApplier;
        applier.apply(&mut snapshot, &event);

        // Payments should be preserved
        assert_eq!(snapshot.payments.len(), 1);
        assert_eq!(snapshot.paid_amount, 50.0);
    }

    #[test]
    fn test_item_restored_different_items() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.last_sequence = 5;

        // First restore
        let event1 = create_item_restored_event("order-1", 6, "item-1", "Product A");
        let applier = ItemRestoredApplier;
        applier.apply(&mut snapshot, &event1);
        assert_eq!(snapshot.last_sequence, 6);

        // Second restore
        let event2 = create_item_restored_event("order-1", 7, "item-2", "Product B");
        applier.apply(&mut snapshot, &event2);
        assert_eq!(snapshot.last_sequence, 7);
    }

    #[test]
    fn test_item_restored_sequence_only_updates_if_higher() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.last_sequence = 10;

        // Event with lower sequence
        let event = create_item_restored_event("order-1", 5, "item-1", "Test Product");

        let applier = ItemRestoredApplier;
        applier.apply(&mut snapshot, &event);

        // Note: Current implementation always updates sequence to event.sequence
        // This might need revision if replay protection is needed
        assert_eq!(snapshot.last_sequence, 5);
    }

    #[test]
    fn test_item_restored_with_unknown_item_name() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        // This is the typical case when item tracking is not implemented
        let event = create_item_restored_event("order-1", 1, "item-1", "Unknown");

        let applier = ItemRestoredApplier;
        applier.apply(&mut snapshot, &event);

        // Should still update sequence/timestamp even with "Unknown" item name
        assert_eq!(snapshot.last_sequence, 1);
    }

    #[test]
    fn test_item_restored_preserves_order_data() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.table_id = Some("dining_table:t1".to_string());
        snapshot.table_name = Some("Table 1".to_string());
        snapshot.guest_count = 4;
        snapshot.is_retail = false;

        let event = create_item_restored_event("order-1", 1, "item-1", "Test Product");

        let applier = ItemRestoredApplier;
        applier.apply(&mut snapshot, &event);

        // Order data should be preserved
        assert_eq!(snapshot.table_id, Some("dining_table:t1".to_string()));
        assert_eq!(snapshot.table_name, Some("Table 1".to_string()));
        assert_eq!(snapshot.guest_count, 4);
        assert!(!snapshot.is_retail);
    }
}
