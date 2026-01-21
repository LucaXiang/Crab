//! PaymentCancelled event applier
//!
//! Applies the PaymentCancelled event to mark a payment as cancelled
//! and update the paid_amount.

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

/// PaymentCancelled applier
pub struct PaymentCancelledApplier;

impl EventApplier for PaymentCancelledApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::PaymentCancelled {
            payment_id, reason, ..
        } = &event.payload
        {
            // Find the payment and mark it as cancelled
            if let Some(payment) = snapshot
                .payments
                .iter_mut()
                .find(|p| p.payment_id == *payment_id && !p.cancelled)
            {
                // Set cancelled flag
                payment.cancelled = true;
                payment.cancel_reason = reason.clone();

                // Subtract from paid_amount
                snapshot.paid_amount -= payment.amount;

                // Update sequence and timestamp
                snapshot.last_sequence = event.sequence;
                snapshot.updated_at = event.timestamp;

                // Update checksum
                snapshot.update_checksum();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::{OrderEventType, PaymentRecord};

    fn create_payment_cancelled_event(
        order_id: &str,
        seq: u64,
        payment_id: &str,
        method: &str,
        amount: f64,
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
            OrderEventType::PaymentCancelled,
            EventPayload::PaymentCancelled {
                payment_id: payment_id.to_string(),
                method: method.to_string(),
                amount,
                reason,
                authorizer_id,
                authorizer_name,
            },
        )
    }

    fn create_payment_record(payment_id: &str, method: &str, amount: f64) -> PaymentRecord {
        PaymentRecord {
            payment_id: payment_id.to_string(),
            method: method.to_string(),
            amount,
            tendered: None,
            change: None,
            note: None,
            timestamp: 1234567800,
            cancelled: false,
            cancel_reason: None,
        }
    }

    #[test]
    fn test_payment_cancelled_applier_basic() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.total = 100.0;
        snapshot.paid_amount = 50.0;
        snapshot.last_sequence = 0;
        snapshot
            .payments
            .push(create_payment_record("payment-1", "credit_card", 50.0));

        let event = create_payment_cancelled_event(
            "order-1",
            1,
            "payment-1",
            "credit_card",
            50.0,
            Some("Refund requested".to_string()),
            None,
            None,
        );

        let applier = PaymentCancelledApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.payments.len(), 1);
        assert!(snapshot.payments[0].cancelled);
        assert_eq!(
            snapshot.payments[0].cancel_reason,
            Some("Refund requested".to_string())
        );
        assert_eq!(snapshot.paid_amount, 0.0);
        assert_eq!(snapshot.last_sequence, 1);
    }

    #[test]
    fn test_payment_cancelled_without_reason() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.total = 100.0;
        snapshot.paid_amount = 50.0;
        snapshot
            .payments
            .push(create_payment_record("payment-1", "cash", 50.0));

        let event = create_payment_cancelled_event(
            "order-1",
            1,
            "payment-1",
            "cash",
            50.0,
            None,
            None,
            None,
        );

        let applier = PaymentCancelledApplier;
        applier.apply(&mut snapshot, &event);

        assert!(snapshot.payments[0].cancelled);
        assert!(snapshot.payments[0].cancel_reason.is_none());
        assert_eq!(snapshot.paid_amount, 0.0);
    }

    #[test]
    fn test_payment_cancelled_partial_payment() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.total = 100.0;
        snapshot.paid_amount = 80.0;
        snapshot
            .payments
            .push(create_payment_record("payment-1", "credit_card", 30.0));
        snapshot
            .payments
            .push(create_payment_record("payment-2", "cash", 50.0));

        // Cancel only the first payment
        let event = create_payment_cancelled_event(
            "order-1",
            1,
            "payment-1",
            "credit_card",
            30.0,
            None,
            None,
            None,
        );

        let applier = PaymentCancelledApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.payments.len(), 2);
        assert!(snapshot.payments[0].cancelled);
        assert!(!snapshot.payments[1].cancelled);
        assert_eq!(snapshot.paid_amount, 50.0); // 80 - 30 = 50
    }

    #[test]
    fn test_payment_cancelled_does_not_affect_other_payments() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.total = 100.0;
        snapshot.paid_amount = 80.0;
        snapshot
            .payments
            .push(create_payment_record("payment-1", "credit_card", 30.0));
        snapshot
            .payments
            .push(create_payment_record("payment-2", "cash", 50.0));

        // Cancel the second payment
        let event = create_payment_cancelled_event(
            "order-1",
            1,
            "payment-2",
            "cash",
            50.0,
            Some("Wrong amount".to_string()),
            None,
            None,
        );

        let applier = PaymentCancelledApplier;
        applier.apply(&mut snapshot, &event);

        assert!(!snapshot.payments[0].cancelled);
        assert!(snapshot.payments[1].cancelled);
        assert_eq!(
            snapshot.payments[1].cancel_reason,
            Some("Wrong amount".to_string())
        );
        assert_eq!(snapshot.paid_amount, 30.0); // 80 - 50 = 30
    }

    #[test]
    fn test_payment_cancelled_idempotent_on_already_cancelled() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.total = 100.0;
        snapshot.paid_amount = 0.0; // Already subtracted
        snapshot.last_sequence = 5;
        let mut payment = create_payment_record("payment-1", "cash", 50.0);
        payment.cancelled = true;
        payment.cancel_reason = Some("Previous cancellation".to_string());
        snapshot.payments.push(payment);

        // Try to cancel again
        let event = create_payment_cancelled_event(
            "order-1",
            6,
            "payment-1",
            "cash",
            50.0,
            Some("New reason".to_string()),
            None,
            None,
        );

        let applier = PaymentCancelledApplier;
        applier.apply(&mut snapshot, &event);

        // Should remain unchanged
        assert!(snapshot.payments[0].cancelled);
        assert_eq!(
            snapshot.payments[0].cancel_reason,
            Some("Previous cancellation".to_string())
        );
        assert_eq!(snapshot.paid_amount, 0.0); // Should not go negative
        assert_eq!(snapshot.last_sequence, 5); // Should not update
    }

    #[test]
    fn test_payment_cancelled_nonexistent_payment_no_effect() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.total = 100.0;
        snapshot.paid_amount = 50.0;
        snapshot.last_sequence = 0;
        snapshot
            .payments
            .push(create_payment_record("payment-1", "cash", 50.0));

        // Try to cancel a non-existent payment
        let event = create_payment_cancelled_event(
            "order-1",
            1,
            "nonexistent",
            "cash",
            50.0,
            None,
            None,
            None,
        );

        let applier = PaymentCancelledApplier;
        applier.apply(&mut snapshot, &event);

        // Should have no effect
        assert!(!snapshot.payments[0].cancelled);
        assert_eq!(snapshot.paid_amount, 50.0);
        assert_eq!(snapshot.last_sequence, 0);
    }

    #[test]
    fn test_payment_cancelled_updates_checksum() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.total = 100.0;
        snapshot.paid_amount = 50.0;
        snapshot
            .payments
            .push(create_payment_record("payment-1", "cash", 50.0));
        snapshot.update_checksum();
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_payment_cancelled_event(
            "order-1",
            1,
            "payment-1",
            "cash",
            50.0,
            None,
            None,
            None,
        );

        let applier = PaymentCancelledApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_payment_cancelled_updates_timestamp() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.total = 100.0;
        snapshot.paid_amount = 50.0;
        snapshot.updated_at = 1000000000;
        snapshot
            .payments
            .push(create_payment_record("payment-1", "cash", 50.0));

        let event = create_payment_cancelled_event(
            "order-1",
            1,
            "payment-1",
            "cash",
            50.0,
            None,
            None,
            None,
        );

        let applier = PaymentCancelledApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.updated_at, event.timestamp);
    }

    #[test]
    fn test_payment_cancelled_remaining_amount_calculation() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.total = 100.0;
        snapshot.paid_amount = 100.0;
        snapshot
            .payments
            .push(create_payment_record("payment-1", "credit_card", 60.0));
        snapshot
            .payments
            .push(create_payment_record("payment-2", "cash", 40.0));

        assert!(snapshot.is_fully_paid());
        assert_eq!(snapshot.remaining_amount(), 0.0);

        // Cancel one payment
        let event = create_payment_cancelled_event(
            "order-1",
            1,
            "payment-1",
            "credit_card",
            60.0,
            None,
            None,
            None,
        );

        let applier = PaymentCancelledApplier;
        applier.apply(&mut snapshot, &event);

        assert!(!snapshot.is_fully_paid());
        assert_eq!(snapshot.remaining_amount(), 60.0);
        assert_eq!(snapshot.paid_amount, 40.0);
    }
}
