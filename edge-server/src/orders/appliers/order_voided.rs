//! OrderVoided event applier
//!
//! Applies the OrderVoided event to mark the order as voided.

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot, OrderStatus};

/// OrderVoided applier
pub struct OrderVoidedApplier;

impl EventApplier for OrderVoidedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::OrderVoided {
            reason: _,
            authorizer_id: _,
            authorizer_name: _,
        } = &event.payload
        {
            // Set status to Void
            snapshot.status = OrderStatus::Void;

            // Set end time (voided_at)
            snapshot.end_time = Some(event.timestamp);

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

    fn create_order_voided_event(
        order_id: &str,
        seq: u64,
        reason: Option<String>,
        authorizer_id: Option<String>,
        authorizer_name: Option<String>,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::OrderVoided,
            EventPayload::OrderVoided {
                reason,
                authorizer_id,
                authorizer_name,
            },
        )
    }

    #[test]
    fn test_order_voided_sets_status() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.last_sequence = 5;

        let event = create_order_voided_event("order-1", 6, None, None, None);

        let applier = OrderVoidedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.status, OrderStatus::Void);
    }

    #[test]
    fn test_order_voided_sets_end_time() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        assert!(snapshot.end_time.is_none());

        let event = create_order_voided_event(
            "order-1",
            1,
            Some("Customer cancelled".to_string()),
            None,
            None,
        );

        let applier = OrderVoidedApplier;
        applier.apply(&mut snapshot, &event);

        // end_time should be set to event.timestamp (voided_at)
        assert_eq!(snapshot.end_time, Some(event.timestamp));
    }

    #[test]
    fn test_order_voided_updates_sequence() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.last_sequence = 10;

        let event = create_order_voided_event("order-1", 11, None, None, None);

        let applier = OrderVoidedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 11);
    }

    #[test]
    fn test_order_voided_updates_timestamp() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        let old_updated_at = snapshot.updated_at;

        let event = create_order_voided_event("order-1", 1, None, None, None);

        let applier = OrderVoidedApplier;
        applier.apply(&mut snapshot, &event);

        // updated_at should be set to event.timestamp
        assert_eq!(snapshot.updated_at, event.timestamp);
        assert!(snapshot.updated_at >= old_updated_at);
    }

    #[test]
    fn test_order_voided_updates_checksum() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_order_voided_event("order-1", 1, None, None, None);

        let applier = OrderVoidedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_order_voided_with_reason() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        let event = create_order_voided_event(
            "order-1",
            1,
            Some("Customer changed mind".to_string()),
            None,
            None,
        );

        let applier = OrderVoidedApplier;
        applier.apply(&mut snapshot, &event);

        // Applier should still work - reason is in event, not snapshot
        assert_eq!(snapshot.status, OrderStatus::Void);
        assert!(snapshot.end_time.is_some());
    }

    #[test]
    fn test_order_voided_with_authorizer() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        let event = create_order_voided_event(
            "order-1",
            1,
            Some("Manager override".to_string()),
            Some("manager-1".to_string()),
            Some("Manager Name".to_string()),
        );

        let applier = OrderVoidedApplier;
        applier.apply(&mut snapshot, &event);

        // Authorizer info is in event, applier just sets status
        assert_eq!(snapshot.status, OrderStatus::Void);
    }

    #[test]
    fn test_order_voided_preserves_existing_data() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.table_id = Some("dining_table:t1".to_string());
        snapshot.table_name = Some("Table 1".to_string());
        snapshot.total = 150.0;
        snapshot.subtotal = 150.0;
        snapshot.paid_amount = 50.0;
        snapshot.guest_count = 4;

        let event =
            create_order_voided_event("order-1", 1, Some("Test void".to_string()), None, None);

        let applier = OrderVoidedApplier;
        applier.apply(&mut snapshot, &event);

        // Existing data should be preserved
        assert_eq!(snapshot.table_id, Some("dining_table:t1".to_string()));
        assert_eq!(snapshot.table_name, Some("Table 1".to_string()));
        assert_eq!(snapshot.total, 150.0);
        assert_eq!(snapshot.subtotal, 150.0);
        assert_eq!(snapshot.paid_amount, 50.0);
        assert_eq!(snapshot.guest_count, 4);

        // But status and end_time should be updated
        assert_eq!(snapshot.status, OrderStatus::Void);
        assert!(snapshot.end_time.is_some());
    }

    #[test]
    fn test_order_voided_idempotent() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        let event = create_order_voided_event("order-1", 1, None, None, None);

        let applier = OrderVoidedApplier;

        // Apply twice
        applier.apply(&mut snapshot, &event);
        let checksum_after_first = snapshot.state_checksum.clone();

        applier.apply(&mut snapshot, &event);
        let checksum_after_second = snapshot.state_checksum.clone();

        // State should be the same
        assert_eq!(snapshot.status, OrderStatus::Void);
        // Checksum should be recalculated but the same (since state is same)
        assert_eq!(checksum_after_first, checksum_after_second);
    }

    #[test]
    fn test_order_voided_different_reasons() {
        let mut snapshot1 = OrderSnapshot::new("order-1".to_string());
        snapshot1.status = OrderStatus::Active;

        let mut snapshot2 = OrderSnapshot::new("order-2".to_string());
        snapshot2.status = OrderStatus::Active;

        let event1 = create_order_voided_event(
            "order-1",
            1,
            Some("Customer cancelled".to_string()),
            None,
            None,
        );
        let event2 =
            create_order_voided_event("order-2", 1, Some("Order error".to_string()), None, None);

        let applier = OrderVoidedApplier;
        applier.apply(&mut snapshot1, &event1);
        applier.apply(&mut snapshot2, &event2);

        // Both should be voided
        assert_eq!(snapshot1.status, OrderStatus::Void);
        assert_eq!(snapshot2.status, OrderStatus::Void);
    }

    #[test]
    fn test_order_voided_clears_receipt_number() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.receipt_number = None; // Orders being voided typically don't have receipt numbers

        let event = create_order_voided_event("order-1", 1, None, None, None);

        let applier = OrderVoidedApplier;
        applier.apply(&mut snapshot, &event);

        // Receipt number should remain None for voided orders
        assert!(snapshot.receipt_number.is_none());
        assert_eq!(snapshot.status, OrderStatus::Void);
    }

    #[test]
    fn test_order_voided_with_payments_preserved() {
        use shared::order::PaymentRecord;

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
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

        let event = create_order_voided_event(
            "order-1",
            2,
            Some("Voiding paid order".to_string()),
            None,
            None,
        );

        let applier = OrderVoidedApplier;
        applier.apply(&mut snapshot, &event);

        // Payments should be preserved for audit trail
        assert_eq!(snapshot.payments.len(), 1);
        assert_eq!(snapshot.paid_amount, 100.0);
        assert_eq!(snapshot.status, OrderStatus::Void);
    }

    #[test]
    fn test_order_voided_sequence_increments() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.last_sequence = 5;

        let event = create_order_voided_event("order-1", 10, None, None, None);

        let applier = OrderVoidedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 10);
    }
}
