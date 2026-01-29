//! PaymentAdded event applier
//!
//! Applies the PaymentAdded event to add a payment to the snapshot.

use crate::orders::money::{self, to_decimal, to_f64, MONEY_TOLERANCE};
use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot, PaymentRecord};

/// PaymentAdded applier
pub struct PaymentAddedApplier;

impl EventApplier for PaymentAddedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::PaymentAdded {
            payment_id,
            method,
            amount,
            tendered,
            change,
            note,
        } = &event.payload
        {
            // Create payment record
            let payment = PaymentRecord {
                payment_id: payment_id.clone(),
                method: method.clone(),
                amount: *amount,
                tendered: *tendered,
                change: *change,
                note: note.clone(),
                timestamp: event.timestamp,
                cancelled: false,
                cancel_reason: None,
                split_items: None,
                aa_shares: None,
                split_type: None,
            };

            // Add payment to snapshot
            snapshot.payments.push(payment);

            // Update paid_amount using Decimal for precision
            snapshot.paid_amount = to_f64(to_decimal(snapshot.paid_amount) + to_decimal(*amount));

            // When fully paid, mark all items as paid for reliable tracking
            // 金额分单不跟踪商品数量，跳过填充 paid_item_quantities
            if to_decimal(snapshot.paid_amount) >= to_decimal(snapshot.total) - MONEY_TOLERANCE {
                if !snapshot.has_amount_split {
                    let item_quantities: Vec<(String, i32)> = snapshot
                        .items
                        .iter()
                        .map(|item| (item.instance_id.clone(), item.quantity))
                        .collect();
                    for (instance_id, quantity) in item_quantities {
                        snapshot.paid_item_quantities.insert(instance_id, quantity);
                    }
                }
                // Recalculate to update unpaid_quantity per item
                money::recalculate_totals(snapshot);
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
    use shared::order::OrderEventType;

    fn create_payment_added_event(
        order_id: &str,
        seq: u64,
        payment_id: &str,
        method: &str,
        amount: f64,
        tendered: Option<f64>,
        change: Option<f64>,
        note: Option<String>,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::PaymentAdded,
            EventPayload::PaymentAdded {
                payment_id: payment_id.to_string(),
                method: method.to_string(),
                amount,
                tendered,
                change,
                note,
            },
        )
    }

    #[test]
    fn test_payment_added_applier_basic() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.total = 100.0;
        snapshot.last_sequence = 0;

        let event = create_payment_added_event(
            "order-1",
            1,
            "payment-1",
            "CARD",
            50.0,
            None,
            None,
            None,
        );

        let applier = PaymentAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.payments.len(), 1);
        assert_eq!(snapshot.payments[0].payment_id, "payment-1");
        assert_eq!(snapshot.payments[0].method, "CARD");
        assert_eq!(snapshot.payments[0].amount, 50.0);
        assert!(!snapshot.payments[0].cancelled);
        assert_eq!(snapshot.paid_amount, 50.0);
        assert_eq!(snapshot.last_sequence, 1);
    }

    #[test]
    fn test_payment_added_applier_cash_with_change() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.total = 85.0;
        snapshot.last_sequence = 0;

        let event = create_payment_added_event(
            "order-1",
            1,
            "payment-1",
            "CASH",
            85.0,
            Some(100.0),
            Some(15.0),
            None,
        );

        let applier = PaymentAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.payments.len(), 1);
        assert_eq!(snapshot.payments[0].method, "CASH");
        assert_eq!(snapshot.payments[0].amount, 85.0);
        assert_eq!(snapshot.payments[0].tendered, Some(100.0));
        assert_eq!(snapshot.payments[0].change, Some(15.0));
        assert_eq!(snapshot.paid_amount, 85.0);
    }

    #[test]
    fn test_payment_added_applier_with_note() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());

        let event = create_payment_added_event(
            "order-1",
            1,
            "payment-1",
            "CARD",
            50.0,
            None,
            None,
            Some("Visa ending in 1234".to_string()),
        );

        let applier = PaymentAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(
            snapshot.payments[0].note,
            Some("Visa ending in 1234".to_string())
        );
    }

    #[test]
    fn test_payment_added_applier_multiple_payments() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.total = 100.0;

        // First payment
        let event1 = create_payment_added_event(
            "order-1",
            1,
            "payment-1",
            "CARD",
            30.0,
            None,
            None,
            None,
        );

        let applier = PaymentAddedApplier;
        applier.apply(&mut snapshot, &event1);

        assert_eq!(snapshot.payments.len(), 1);
        assert_eq!(snapshot.paid_amount, 30.0);

        // Second payment
        let event2 = create_payment_added_event(
            "order-1",
            2,
            "payment-2",
            "CASH",
            70.0,
            Some(100.0),
            Some(30.0),
            None,
        );

        applier.apply(&mut snapshot, &event2);

        assert_eq!(snapshot.payments.len(), 2);
        assert_eq!(snapshot.paid_amount, 100.0);
        assert_eq!(snapshot.last_sequence, 2);

        // Verify payment details
        assert_eq!(snapshot.payments[0].payment_id, "payment-1");
        assert_eq!(snapshot.payments[1].payment_id, "payment-2");
    }

    #[test]
    fn test_payment_added_applier_updates_sequence() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.last_sequence = 5;

        let event =
            create_payment_added_event("order-1", 6, "payment-1", "CASH", 50.0, None, None, None);

        let applier = PaymentAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 6);
    }

    #[test]
    fn test_payment_added_applier_updates_checksum() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let initial_checksum = snapshot.state_checksum.clone();

        let event =
            create_payment_added_event("order-1", 1, "payment-1", "CASH", 50.0, None, None, None);

        let applier = PaymentAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_payment_added_applier_sets_timestamp() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());

        let event =
            create_payment_added_event("order-1", 1, "payment-1", "CASH", 50.0, None, None, None);

        let applier = PaymentAddedApplier;
        applier.apply(&mut snapshot, &event);

        // Payment timestamp should match event's server timestamp
        // (event.timestamp is set by OrderEvent::new to current server time)
        assert_eq!(snapshot.payments[0].timestamp, event.timestamp);
        assert_eq!(snapshot.updated_at, event.timestamp);
    }

    #[test]
    fn test_payment_added_applier_partial_payment() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.total = 100.0;

        // Partial payment
        let event = create_payment_added_event(
            "order-1",
            1,
            "payment-1",
            "CARD",
            40.0,
            None,
            None,
            None,
        );

        let applier = PaymentAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.paid_amount, 40.0);
        assert_eq!(snapshot.remaining_amount(), 60.0);
        assert!(!snapshot.is_fully_paid());
    }

    #[test]
    fn test_payment_added_applier_full_payment() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.total = 100.0;

        let event = create_payment_added_event(
            "order-1",
            1,
            "payment-1",
            "CARD",
            100.0,
            None,
            None,
            None,
        );

        let applier = PaymentAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.paid_amount, 100.0);
        assert_eq!(snapshot.remaining_amount(), 0.0);
        assert!(snapshot.is_fully_paid());
    }
}
