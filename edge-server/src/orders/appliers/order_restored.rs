//! OrderRestored event applier
//!
//! Applies the OrderRestored event to restore a voided order back to Active status.

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot, OrderStatus};

/// OrderRestored applier
pub struct OrderRestoredApplier;

impl EventApplier for OrderRestoredApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::OrderRestored {} = &event.payload {
            // Set status back to Active
            snapshot.status = OrderStatus::Active;

            // Clear end time (order is active again)
            snapshot.end_time = None;

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
    use shared::order::OrderEventType;

    fn create_order_restored_event(order_id: &str, seq: u64) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::OrderRestored,
            EventPayload::OrderRestored {},
        )
    }

    #[test]
    fn test_order_restored_sets_status_active() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        snapshot.last_sequence = 5;

        let event = create_order_restored_event("order-1", 6);

        let applier = OrderRestoredApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.status, OrderStatus::Active);
    }

    #[test]
    fn test_order_restored_clears_end_time() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        snapshot.end_time = Some(1234567800); // Was set when voided

        let event = create_order_restored_event("order-1", 1);

        let applier = OrderRestoredApplier;
        applier.apply(&mut snapshot, &event);

        // end_time should be cleared (order is active again)
        assert!(snapshot.end_time.is_none());
    }

    #[test]
    fn test_order_restored_updates_sequence() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        snapshot.last_sequence = 10;

        let event = create_order_restored_event("order-1", 11);

        let applier = OrderRestoredApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 11);
    }

    #[test]
    fn test_order_restored_updates_timestamp() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        let old_updated_at = snapshot.updated_at;

        let event = create_order_restored_event("order-1", 1);

        let applier = OrderRestoredApplier;
        applier.apply(&mut snapshot, &event);

        // updated_at should be set to event.timestamp
        assert_eq!(snapshot.updated_at, event.timestamp);
        assert!(snapshot.updated_at >= old_updated_at);
    }

    #[test]
    fn test_order_restored_updates_checksum() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_order_restored_event("order-1", 1);

        let applier = OrderRestoredApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_order_restored_preserves_existing_data() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        snapshot.table_id = Some("dining_table:t1".to_string());
        snapshot.table_name = Some("Table 1".to_string());
        snapshot.total = 150.0;
        snapshot.subtotal = 150.0;
        snapshot.paid_amount = 50.0;
        snapshot.guest_count = 4;

        let event = create_order_restored_event("order-1", 1);

        let applier = OrderRestoredApplier;
        applier.apply(&mut snapshot, &event);

        // Existing data should be preserved
        assert_eq!(snapshot.table_id, Some("dining_table:t1".to_string()));
        assert_eq!(snapshot.table_name, Some("Table 1".to_string()));
        assert_eq!(snapshot.total, 150.0);
        assert_eq!(snapshot.subtotal, 150.0);
        assert_eq!(snapshot.paid_amount, 50.0);
        assert_eq!(snapshot.guest_count, 4);

        // Status should be updated to Active
        assert_eq!(snapshot.status, OrderStatus::Active);
        // end_time should be cleared
        assert!(snapshot.end_time.is_none());
    }

    #[test]
    fn test_order_restored_idempotent() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;

        let event = create_order_restored_event("order-1", 1);

        let applier = OrderRestoredApplier;

        // Apply twice
        applier.apply(&mut snapshot, &event);
        let checksum_after_first = snapshot.state_checksum.clone();

        applier.apply(&mut snapshot, &event);
        let checksum_after_second = snapshot.state_checksum.clone();

        // State should be the same
        assert_eq!(snapshot.status, OrderStatus::Active);
        // Checksum should be recalculated but the same (since state is same)
        assert_eq!(checksum_after_first, checksum_after_second);
    }

    #[test]
    fn test_order_restored_with_payments_preserved() {
        use shared::order::PaymentRecord;

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        snapshot.total = 100.0;
        snapshot.paid_amount = 100.0;
        snapshot.payments.push(PaymentRecord {
            payment_id: "pay-1".to_string(),
            method: "CASH".to_string(),
            amount: 100.0,
            tendered: Some(100.0),
            change: Some(0.0),
            note: None,
            timestamp: 1234567800,
            cancelled: false,
            cancel_reason: None,
            split_items: None,
        });

        let event = create_order_restored_event("order-1", 2);

        let applier = OrderRestoredApplier;
        applier.apply(&mut snapshot, &event);

        // Payments should be preserved
        assert_eq!(snapshot.payments.len(), 1);
        assert_eq!(snapshot.paid_amount, 100.0);
        assert_eq!(snapshot.status, OrderStatus::Active);
    }

    #[test]
    fn test_order_restored_with_items_preserved() {
        use shared::order::CartItemSnapshot;

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        snapshot.items.push(CartItemSnapshot {
            id: "product:p1".to_string(),
            instance_id: "item-1".to_string(),
            name: "Test Product".to_string(),
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
        });
        snapshot.subtotal = 20.0;
        snapshot.total = 20.0;

        let event = create_order_restored_event("order-1", 1);

        let applier = OrderRestoredApplier;
        applier.apply(&mut snapshot, &event);

        // Items should be preserved
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "item-1");
        assert_eq!(snapshot.subtotal, 20.0);
        assert_eq!(snapshot.total, 20.0);
        assert_eq!(snapshot.status, OrderStatus::Active);
    }

    #[test]
    fn test_order_restored_sequence_increments() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        snapshot.last_sequence = 5;

        let event = create_order_restored_event("order-1", 10);

        let applier = OrderRestoredApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 10);
    }

    #[test]
    fn test_order_restored_receipt_number_preserved() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        snapshot.receipt_number = "RC-001".to_string();

        let event = create_order_restored_event("order-1", 1);

        let applier = OrderRestoredApplier;
        applier.apply(&mut snapshot, &event);

        // Receipt number should be preserved (if it was assigned before voiding)
        assert_eq!(snapshot.receipt_number, "RC-001");
        assert_eq!(snapshot.status, OrderStatus::Active);
    }
}
