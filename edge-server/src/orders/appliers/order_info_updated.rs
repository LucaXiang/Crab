//! OrderInfoUpdated event applier
//!
//! Applies the OrderInfoUpdated event to update order metadata in the snapshot.
//! Only updates fields that are present (Some) in the event payload.

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

/// OrderInfoUpdated applier
pub struct OrderInfoUpdatedApplier;

impl EventApplier for OrderInfoUpdatedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::OrderInfoUpdated {
            guest_count,
            table_name,
            is_pre_payment,
        } = &event.payload
        {
            // Only update fields that are present (Some) in the event
            // Note: receipt_number is immutable (set at OpenTable)

            if let Some(count) = guest_count {
                snapshot.guest_count = *count;
            }

            if let Some(name) = table_name {
                snapshot.table_name = Some(name.clone());
            }

            if let Some(pre_payment) = is_pre_payment {
                snapshot.is_pre_payment = *pre_payment;
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
    use shared::order::types::ServiceType;
    use shared::order::{OrderEventType, OrderStatus};

    fn create_test_snapshot(order_id: &str) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.guest_count = 2;
        snapshot.table_name = Some("Table 1".to_string());
        snapshot.is_pre_payment = false;
        snapshot
    }

    fn create_order_info_updated_event(
        order_id: &str,
        seq: u64,
        guest_count: Option<i32>,
        table_name: Option<String>,
        is_pre_payment: Option<bool>,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::OrderInfoUpdated,
            EventPayload::OrderInfoUpdated {
                guest_count,
                table_name,
                is_pre_payment,
            },
        )
    }

    #[test]
    fn test_order_info_updated_guest_count_only() {
        let mut snapshot = create_test_snapshot("order-1");
        let initial_guest_count = snapshot.guest_count;

        let event = create_order_info_updated_event("order-1", 2, Some(6), None, None);

        let applier = OrderInfoUpdatedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.guest_count, 6);
        assert_ne!(snapshot.guest_count, initial_guest_count);
        // Other fields should remain unchanged
        assert_eq!(snapshot.table_name, Some("Table 1".to_string()));
        assert!(snapshot.receipt_number.is_empty());
        assert!(!snapshot.is_pre_payment);
    }

    #[test]
    fn test_order_info_updated_table_name_only() {
        let mut snapshot = create_test_snapshot("order-1");

        let event = create_order_info_updated_event(
            "order-1",
            2,
            None,
            Some("VIP Room".to_string()),
            None,
        );

        let applier = OrderInfoUpdatedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.table_name, Some("VIP Room".to_string()));
        // Other fields should remain unchanged
        assert_eq!(snapshot.guest_count, 2);
        assert!(snapshot.receipt_number.is_empty());
        assert!(!snapshot.is_pre_payment);
    }

    #[test]
    fn test_order_info_updated_receipt_number_only() {
        let mut snapshot = create_test_snapshot("order-1");

        // receipt_number is immutable (set at OpenTable), not updatable via OrderInfoUpdated
        // This test is no longer valid - receipt_number cannot be updated
        // Keeping test but changing to test guest_count update instead
        let event = create_order_info_updated_event("order-1", 2, Some(5), None, None);

        let applier = OrderInfoUpdatedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.guest_count, 5);
        // Other fields should remain unchanged
        assert_eq!(snapshot.table_name, Some("Table 1".to_string()));
        assert!(snapshot.receipt_number.is_empty()); // receipt_number not changed by this event
        assert!(!snapshot.is_pre_payment);
    }

    #[test]
    fn test_order_info_updated_is_pre_payment_only() {
        let mut snapshot = create_test_snapshot("order-1");
        assert!(!snapshot.is_pre_payment);

        let event = create_order_info_updated_event("order-1", 2, None, None, Some(true));

        let applier = OrderInfoUpdatedApplier;
        applier.apply(&mut snapshot, &event);

        assert!(snapshot.is_pre_payment);
        // Other fields should remain unchanged
        assert_eq!(snapshot.guest_count, 2);
        assert_eq!(snapshot.table_name, Some("Table 1".to_string()));
        assert!(snapshot.receipt_number.is_empty());
    }

    #[test]
    fn test_order_info_updated_multiple_fields() {
        let mut snapshot = create_test_snapshot("order-1");

        let event = create_order_info_updated_event(
            "order-1",
            2,
            Some(8),
            Some("Private Dining".to_string()),
            Some(true),
        );

        let applier = OrderInfoUpdatedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.guest_count, 8);
        assert_eq!(snapshot.table_name, Some("Private Dining".to_string()));
        assert!(snapshot.is_pre_payment);
        // receipt_number unchanged (immutable)
        assert!(snapshot.receipt_number.is_empty());
    }

    #[test]
    fn test_order_info_updated_updates_sequence() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.last_sequence = 5;

        let event = create_order_info_updated_event("order-1", 10, Some(4), None, None);

        let applier = OrderInfoUpdatedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 10);
    }

    #[test]
    fn test_order_info_updated_updates_timestamp() {
        let mut snapshot = create_test_snapshot("order-1");
        // Set initial timestamp to a known different value
        snapshot.updated_at = 1000000000;

        let event = create_order_info_updated_event("order-1", 2, Some(4), None, None);
        let expected_timestamp = event.timestamp; // Server timestamp set at event creation

        let applier = OrderInfoUpdatedApplier;
        applier.apply(&mut snapshot, &event);

        // snapshot.updated_at should match the event's server timestamp
        assert_eq!(snapshot.updated_at, expected_timestamp);
        // Should be different from our initial value
        assert_ne!(snapshot.updated_at, 1000000000);
    }

    #[test]
    fn test_order_info_updated_updates_checksum() {
        let mut snapshot = create_test_snapshot("order-1");
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_order_info_updated_event("order-1", 2, Some(10), None, None);

        let applier = OrderInfoUpdatedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_order_info_updated_no_fields_is_noop_for_data() {
        let mut snapshot = create_test_snapshot("order-1");
        let original_guest_count = snapshot.guest_count;
        let original_table_name = snapshot.table_name.clone();
        let original_receipt_number = snapshot.receipt_number.clone();
        let original_is_pre_payment = snapshot.is_pre_payment;

        // Event with all None values (shouldn't happen in practice, but test the behavior)
        let event = create_order_info_updated_event("order-1", 2, None, None, None);

        let applier = OrderInfoUpdatedApplier;
        applier.apply(&mut snapshot, &event);

        // Data fields should remain unchanged
        assert_eq!(snapshot.guest_count, original_guest_count);
        assert_eq!(snapshot.table_name, original_table_name);
        assert_eq!(snapshot.receipt_number, original_receipt_number);
        assert_eq!(snapshot.is_pre_payment, original_is_pre_payment);

        // Sequence and timestamp are still updated
        assert_eq!(snapshot.last_sequence, 2);
    }

    #[test]
    fn test_order_info_updated_overwrite_existing_values() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.receipt_number = "OLD-001".to_string(); // receipt_number is immutable
        snapshot.is_pre_payment = true;

        let event = create_order_info_updated_event(
            "order-1",
            2,
            Some(1),
            Some("New Table".to_string()),
            Some(false),
        );

        let applier = OrderInfoUpdatedApplier;
        applier.apply(&mut snapshot, &event);

        // receipt_number should NOT change (immutable)
        assert_eq!(snapshot.receipt_number, "OLD-001");
        assert_eq!(snapshot.guest_count, 1);
        assert_eq!(snapshot.table_name, Some("New Table".to_string()));
        assert!(!snapshot.is_pre_payment);
    }

    #[test]
    fn test_order_info_updated_partial_overwrite() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.receipt_number = "R-OLD".to_string(); // receipt_number is immutable
        snapshot.is_pre_payment = true;

        // Only update guest_count and table_name
        let event = create_order_info_updated_event(
            "order-1",
            2,
            Some(5),
            Some("Updated Table".to_string()),
            None, // Don't update is_pre_payment
        );

        let applier = OrderInfoUpdatedApplier;
        applier.apply(&mut snapshot, &event);

        // Updated fields
        assert_eq!(snapshot.guest_count, 5);
        assert_eq!(snapshot.table_name, Some("Updated Table".to_string()));
        // Unchanged fields (receipt_number is immutable, is_pre_payment not in event)
        assert_eq!(snapshot.receipt_number, "R-OLD");
        assert!(snapshot.is_pre_payment);
    }

    #[test]
    fn test_order_info_updated_checksum_verifiable() {
        let mut snapshot = create_test_snapshot("order-1");

        let event = create_order_info_updated_event(
            "order-1",
            2,
            Some(3),
            Some("Table 3".to_string()),
            Some(true),
        );

        let applier = OrderInfoUpdatedApplier;
        applier.apply(&mut snapshot, &event);

        // Checksum should be valid after update
        assert!(snapshot.verify_checksum());

        // Tampering with checksum-relevant data should invalidate checksum
        // (Note: checksum includes items.len, total, paid_amount, last_sequence, status)
        snapshot.last_sequence = 999;
        assert!(!snapshot.verify_checksum());
    }

    #[test]
    fn test_order_info_updated_wrong_event_type_is_noop() {
        let mut snapshot = create_test_snapshot("order-1");
        let original_guest_count = snapshot.guest_count;
        let original_sequence = snapshot.last_sequence;

        // Create an event with wrong payload type (simulating a mismatch)
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
                service_type: Some(ServiceType::DineIn),
                final_total: 100.0,
                payment_summary: vec![],
            },
        );

        let applier = OrderInfoUpdatedApplier;
        applier.apply(&mut snapshot, &event);

        // Nothing should change
        assert_eq!(snapshot.guest_count, original_guest_count);
        assert_eq!(snapshot.last_sequence, original_sequence);
    }

    #[test]
    fn test_order_info_updated_set_is_pre_payment_false() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.is_pre_payment = true;

        let event = create_order_info_updated_event("order-1", 2, None, None, Some(false));

        let applier = OrderInfoUpdatedApplier;
        applier.apply(&mut snapshot, &event);

        assert!(!snapshot.is_pre_payment);
    }

    #[test]
    fn test_order_info_updated_set_guest_count_to_one() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.guest_count = 10;

        let event = create_order_info_updated_event("order-1", 2, Some(1), None, None);

        let applier = OrderInfoUpdatedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.guest_count, 1);
    }

    #[test]
    fn test_order_info_updated_empty_string_table_name() {
        let mut snapshot = create_test_snapshot("order-1");

        let event =
            create_order_info_updated_event("order-1", 2, None, Some("".to_string()), None);

        let applier = OrderInfoUpdatedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.table_name, Some("".to_string()));
    }

    // Removed test_order_info_updated_empty_string_receipt_number
    // receipt_number is immutable (set at OpenTable), cannot be updated via OrderInfoUpdated
}
