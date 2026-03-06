//! PaymentAdded event applier
//!
//! Applies the PaymentAdded event to add a payment to the snapshot.

use crate::order_money::{self, MONEY_TOLERANCE, to_decimal, to_f64};
use crate::orders::traits::EventApplier;
use rust_decimal::Decimal;
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
                payment_id: *payment_id,
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

            // Sync remaining_amount after paid_amount changes (must always be updated)
            let paid = to_decimal(snapshot.paid_amount);
            let total = to_decimal(snapshot.total);
            snapshot.remaining_amount = to_f64((total - paid).max(Decimal::ZERO));

            // When fully paid, mark all items as paid for reliable tracking
            // 金额分单不跟踪商品数量，跳过填充 paid_item_quantities
            if paid >= total - MONEY_TOLERANCE && !snapshot.has_amount_split {
                let item_quantities: Vec<(String, i32)> = snapshot
                    .items
                    .iter()
                    .filter(|item| !item.is_comped)
                    .map(|item| (item.instance_id.clone(), item.quantity))
                    .collect();
                for (instance_id, quantity) in item_quantities {
                    snapshot.paid_item_quantities.insert(instance_id, quantity);
                }
            }

            // Always recalculate to update unpaid_quantity per item
            order_money::recalculate_totals(snapshot);

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

    /// Create a snapshot with a single item of given price (so recalculate_totals produces correct total)
    fn snapshot_with_total(order_id: i64, total: f64) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id);
        snapshot.items.push(CartItemSnapshot {
            id: 1,
            instance_id: "test-item".to_string(),
            name: "Item".to_string(),
            price: total,
            original_price: total,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: 0.0,
            rule_surcharge_amount: 0.0,
            applied_rules: vec![],
            applied_mg_rules: vec![],
            mg_discount_amount: 0.0,
            unit_price: total,
            line_total: total,
            tax: 0.0,
            tax_rate: 0,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_id: None,
            category_name: None,
            is_comped: false,
        });
        order_money::recalculate_totals(&mut snapshot);
        snapshot
    }

    fn create_payment_added_event(
        order_id: i64,
        seq: u64,
        payment_id: i64,
        method: &str,
        amount: f64,
        tendered: Option<f64>,
        change: Option<f64>,
        note: Option<String>,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id,
            1,
            "Test User".to_string(),
            shared::util::snowflake_id(),
            Some(1234567890),
            OrderEventType::PaymentAdded,
            EventPayload::PaymentAdded {
                payment_id: payment_id,
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
        let mut snapshot = snapshot_with_total(1001, 100.0);
        snapshot.last_sequence = 0;

        let event = create_payment_added_event(1001, 1, 4001, "CARD", 50.0, None, None, None);

        let applier = PaymentAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.payments.len(), 1);
        assert_eq!(snapshot.payments[0].payment_id, 4001);
        assert_eq!(snapshot.payments[0].method, "CARD");
        assert_eq!(snapshot.payments[0].amount, 50.0);
        assert!(!snapshot.payments[0].cancelled);
        assert_eq!(snapshot.paid_amount, 50.0);
        assert_eq!(snapshot.last_sequence, 1);
    }

    #[test]
    fn test_payment_added_applier_cash_with_change() {
        let mut snapshot = snapshot_with_total(1001, 85.0);
        snapshot.last_sequence = 0;

        let event =
            create_payment_added_event(1001, 1, 4001, "CASH", 85.0, Some(100.0), Some(15.0), None);

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
        let mut snapshot = OrderSnapshot::new(1001);

        let event = create_payment_added_event(
            1001,
            1,
            4001,
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
        let mut snapshot = snapshot_with_total(1001, 100.0);

        // First payment
        let event1 = create_payment_added_event(1001, 1, 4001, "CARD", 30.0, None, None, None);

        let applier = PaymentAddedApplier;
        applier.apply(&mut snapshot, &event1);

        assert_eq!(snapshot.payments.len(), 1);
        assert_eq!(snapshot.paid_amount, 30.0);

        // Second payment
        let event2 =
            create_payment_added_event(1001, 2, 4002, "CASH", 70.0, Some(100.0), Some(30.0), None);

        applier.apply(&mut snapshot, &event2);

        assert_eq!(snapshot.payments.len(), 2);
        assert_eq!(snapshot.paid_amount, 100.0);
        assert_eq!(snapshot.last_sequence, 2);

        // Verify payment details
        assert_eq!(snapshot.payments[0].payment_id, 4001);
        assert_eq!(snapshot.payments[1].payment_id, 4002);
    }

    #[test]
    fn test_payment_added_applier_updates_sequence() {
        let mut snapshot = OrderSnapshot::new(1001);
        snapshot.last_sequence = 5;

        let event = create_payment_added_event(1001, 6, 4001, "CASH", 50.0, None, None, None);

        let applier = PaymentAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 6);
    }

    #[test]
    fn test_payment_added_applier_updates_checksum() {
        let mut snapshot = OrderSnapshot::new(1001);
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_payment_added_event(1001, 1, 4001, "CASH", 50.0, None, None, None);

        let applier = PaymentAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_payment_added_applier_sets_timestamp() {
        let mut snapshot = OrderSnapshot::new(1001);

        let event = create_payment_added_event(1001, 1, 4001, "CASH", 50.0, None, None, None);

        let applier = PaymentAddedApplier;
        applier.apply(&mut snapshot, &event);

        // Payment timestamp should match event's server timestamp
        // (event.timestamp is set by OrderEvent::new to current server time)
        assert_eq!(snapshot.payments[0].timestamp, event.timestamp);
        assert_eq!(snapshot.updated_at, event.timestamp);
    }

    #[test]
    fn test_payment_added_applier_partial_payment() {
        let mut snapshot = snapshot_with_total(1001, 100.0);

        // Partial payment
        let event = create_payment_added_event(1001, 1, 4001, "CARD", 40.0, None, None, None);

        let applier = PaymentAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.paid_amount, 40.0);
        assert_eq!(snapshot.remaining_amount(), 60.0);
        assert!(!snapshot.is_fully_paid());
    }

    #[test]
    fn test_payment_added_applier_full_payment() {
        let mut snapshot = snapshot_with_total(1001, 100.0);

        let event = create_payment_added_event(1001, 1, 4001, "CARD", 100.0, None, None, None);

        let applier = PaymentAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.paid_amount, 100.0);
        assert_eq!(snapshot.remaining_amount(), 0.0);
        assert!(snapshot.is_fully_paid());
    }
}
