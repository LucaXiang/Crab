//! OrderCompleted event applier
//!
//! Applies the OrderCompleted event to mark the order as completed.

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot, OrderStatus};

/// OrderCompleted applier
pub struct OrderCompletedApplier;

impl EventApplier for OrderCompletedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::OrderCompleted {
            receipt_number,
            final_total: _,
            payment_summary: _,
        } = &event.payload
        {
            // Set status to Completed
            snapshot.status = OrderStatus::Completed;

            // Set receipt number (overwrite with event value for audit trail)
            snapshot.receipt_number = receipt_number.clone();

            // Set end time
            snapshot.end_time = Some(event.timestamp);

            // Safety net: mark all items as fully paid on completion
            for item in &snapshot.items {
                snapshot
                    .paid_item_quantities
                    .insert(item.instance_id.clone(), item.quantity);
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
    use shared::order::{OrderEventType, PaymentSummaryItem};

    fn create_order_completed_event(
        order_id: &str,
        seq: u64,
        receipt_number: &str,
        final_total: f64,
        payment_summary: Vec<PaymentSummaryItem>,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::OrderCompleted,
            EventPayload::OrderCompleted {
                receipt_number: receipt_number.to_string(),
                final_total,
                payment_summary,
            },
        )
    }

    #[test]
    fn test_order_completed_sets_status() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.last_sequence = 5;

        let event = create_order_completed_event("order-1", 6, "RCP-001", 100.0, vec![]);

        let applier = OrderCompletedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.status, OrderStatus::Completed);
    }

    #[test]
    fn test_order_completed_sets_receipt_number() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        assert!(snapshot.receipt_number.is_empty());

        let event = create_order_completed_event("order-1", 1, "RCP-12345", 100.0, vec![]);

        let applier = OrderCompletedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.receipt_number, "RCP-12345");
    }

    #[test]
    fn test_order_completed_sets_end_time() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        assert!(snapshot.end_time.is_none());

        let event = create_order_completed_event("order-1", 1, "RCP-001", 100.0, vec![]);

        let applier = OrderCompletedApplier;
        applier.apply(&mut snapshot, &event);

        // end_time should be set to event.timestamp
        assert_eq!(snapshot.end_time, Some(event.timestamp));
    }

    #[test]
    fn test_order_completed_updates_sequence() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.last_sequence = 10;

        let event = create_order_completed_event("order-1", 11, "RCP-001", 100.0, vec![]);

        let applier = OrderCompletedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 11);
    }

    #[test]
    fn test_order_completed_updates_timestamp() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        let old_updated_at = snapshot.updated_at;

        let event = create_order_completed_event("order-1", 1, "RCP-001", 100.0, vec![]);

        let applier = OrderCompletedApplier;
        applier.apply(&mut snapshot, &event);

        // updated_at should be set to event.timestamp
        assert_eq!(snapshot.updated_at, event.timestamp);
        // The event timestamp is server-generated, so it will be different from old_updated_at
        // (both are current time, but snapshot was created slightly before event)
        assert!(snapshot.updated_at >= old_updated_at);
    }

    #[test]
    fn test_order_completed_updates_checksum() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_order_completed_event("order-1", 1, "RCP-001", 100.0, vec![]);

        let applier = OrderCompletedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_order_completed_with_payment_summary() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;

        let payment_summary = vec![
            PaymentSummaryItem {
                method: "CASH".to_string(),
                amount: 70.0,
            },
            PaymentSummaryItem {
                method: "CARD".to_string(),
                amount: 30.0,
            },
        ];

        let event = create_order_completed_event("order-1", 1, "RCP-001", 100.0, payment_summary);

        let applier = OrderCompletedApplier;
        applier.apply(&mut snapshot, &event);

        // Applier should still work - payment_summary is in event, not snapshot
        assert_eq!(snapshot.status, OrderStatus::Completed);
        assert_eq!(snapshot.receipt_number, "RCP-001");
    }

    #[test]
    fn test_order_completed_preserves_existing_data() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.table_id = Some("dining_table:t1".to_string());
        snapshot.table_name = Some("Table 1".to_string());
        snapshot.total = 150.0;
        snapshot.subtotal = 150.0;
        snapshot.guest_count = 4;

        let event = create_order_completed_event("order-1", 1, "RCP-001", 150.0, vec![]);

        let applier = OrderCompletedApplier;
        applier.apply(&mut snapshot, &event);

        // Existing data should be preserved
        assert_eq!(snapshot.table_id, Some("dining_table:t1".to_string()));
        assert_eq!(snapshot.table_name, Some("Table 1".to_string()));
        assert_eq!(snapshot.total, 150.0);
        assert_eq!(snapshot.subtotal, 150.0);
        assert_eq!(snapshot.guest_count, 4);

        // But status and receipt should be updated
        assert_eq!(snapshot.status, OrderStatus::Completed);
        assert_eq!(snapshot.receipt_number, "RCP-001");
    }

    #[test]
    fn test_order_completed_idempotent() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        let event = create_order_completed_event("order-1", 1, "RCP-001", 100.0, vec![]);

        let applier = OrderCompletedApplier;

        // Apply twice
        applier.apply(&mut snapshot, &event);
        let checksum_after_first = snapshot.state_checksum.clone();

        applier.apply(&mut snapshot, &event);
        let checksum_after_second = snapshot.state_checksum.clone();

        // State should be the same
        assert_eq!(snapshot.status, OrderStatus::Completed);
        assert_eq!(snapshot.receipt_number, "RCP-001");
        // Checksum should be recalculated but the same (since state is same)
        assert_eq!(checksum_after_first, checksum_after_second);
    }

    #[test]
    fn test_order_completed_different_receipt_numbers() {
        let mut snapshot1 = OrderSnapshot::new("order-1".to_string());
        snapshot1.status = OrderStatus::Active;

        let mut snapshot2 = OrderSnapshot::new("order-2".to_string());
        snapshot2.status = OrderStatus::Active;

        let event1 = create_order_completed_event("order-1", 1, "RCP-001", 100.0, vec![]);
        let event2 = create_order_completed_event("order-2", 1, "RCP-002", 200.0, vec![]);

        let applier = OrderCompletedApplier;
        applier.apply(&mut snapshot1, &event1);
        applier.apply(&mut snapshot2, &event2);

        assert_eq!(snapshot1.receipt_number, "RCP-001".to_string());
        assert_eq!(snapshot2.receipt_number, "RCP-002".to_string());
    }
}
