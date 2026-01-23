//! PaymentCancelled event applier
//!
//! Applies the PaymentCancelled event to mark a payment as cancelled
//! and update the paid_amount.
//!
//! For split payments with `split_items`, this applier also restores items
//! using "add items" logic - merging with existing items or creating new ones.

use crate::orders::traits::EventApplier;
use shared::order::{CartItemSnapshot, EventPayload, OrderEvent, OrderSnapshot};

/// PaymentCancelled applier
pub struct PaymentCancelledApplier;

impl EventApplier for PaymentCancelledApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::PaymentCancelled {
            payment_id, reason, ..
        } = &event.payload
        {
            // Find the payment and mark it as cancelled
            // We need to find and clone split_items before mutating payment
            let (amount, split_items) = {
                if let Some(payment) = snapshot
                    .payments
                    .iter()
                    .find(|p| p.payment_id == *payment_id && !p.cancelled)
                {
                    (payment.amount, payment.split_items.clone())
                } else {
                    return; // Payment not found or already cancelled
                }
            };

            // Now mutate the payment
            if let Some(payment) = snapshot
                .payments
                .iter_mut()
                .find(|p| p.payment_id == *payment_id && !p.cancelled)
            {
                // Set cancelled flag
                payment.cancelled = true;
                payment.cancel_reason = reason.clone();
            }

            // Subtract from paid_amount
            snapshot.paid_amount -= amount;

            // If this was a split payment, restore items using "add items" logic
            if let Some(items_to_restore) = split_items {
                restore_split_items(snapshot, &items_to_restore);
            }

            // Update sequence and timestamp
            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;

            // Update checksum
            snapshot.update_checksum();
        }
    }
}

/// Restore split payment items using "add items" logic
///
/// For each item in split_items:
/// - If an item with the same instance_id exists in the order, merge quantities
/// - Otherwise, create a new item entry
///
/// This handles the case where the original items may have been modified
/// (e.g., discounts added to remaining quantity, causing instance_id change)
fn restore_split_items(snapshot: &mut OrderSnapshot, items_to_restore: &[CartItemSnapshot]) {
    for restore_item in items_to_restore {
        // Try to find an existing item with the same instance_id
        if let Some(existing) = snapshot
            .items
            .iter_mut()
            .find(|i| i.instance_id == restore_item.instance_id)
        {
            // Merge: add quantity back
            existing.quantity += restore_item.quantity;
            existing.unpaid_quantity += restore_item.quantity;
        } else {
            // No matching item found - add as new item
            let mut new_item = restore_item.clone();
            // Set unpaid_quantity to quantity (all restored items are unpaid)
            new_item.unpaid_quantity = new_item.quantity;
            snapshot.items.push(new_item);
        }

        // Restore paid_item_quantities
        if let Some(paid_qty) = snapshot
            .paid_item_quantities
            .get_mut(&restore_item.instance_id)
        {
            *paid_qty = (*paid_qty - restore_item.quantity).max(0);
            if *paid_qty == 0 {
                snapshot
                    .paid_item_quantities
                    .remove(&restore_item.instance_id);
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
            split_items: None,
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

    // ==================== Split Payment Cancellation Tests ====================

    #[test]
    fn test_cancel_split_payment_restores_items_to_existing() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.total = 100.0;
        snapshot.paid_amount = 50.0;

        // Add an item with 5 quantity (3 remain unpaid)
        let item = CartItemSnapshot {
            id: "product:1".to_string(),
            instance_id: "inst-1".to_string(),
            name: "Coffee".to_string(),
            price: 10.0,
            original_price: None,
            quantity: 3,
            unpaid_quantity: 3,
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
        };
        snapshot.items.push(item.clone());

        // Create split payment with items
        let mut split_item = item.clone();
        split_item.quantity = 2;
        split_item.unpaid_quantity = 0;

        let mut payment = create_payment_record("split-pay-1", "cash", 20.0);
        payment.split_items = Some(vec![split_item]);
        snapshot.payments.push(payment);
        snapshot
            .paid_item_quantities
            .insert("inst-1".to_string(), 2);

        // Cancel the split payment
        let event = create_payment_cancelled_event(
            "order-1",
            1,
            "split-pay-1",
            "cash",
            20.0,
            None,
            None,
            None,
        );

        let applier = PaymentCancelledApplier;
        applier.apply(&mut snapshot, &event);

        // Check: paid_amount reduced
        assert_eq!(snapshot.paid_amount, 30.0); // 50 - 20 = 30

        // Check: item quantity restored (merged with existing)
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].quantity, 5); // 3 + 2 = 5
        assert_eq!(snapshot.items[0].unpaid_quantity, 5); // 3 + 2 = 5

        // Check: paid_item_quantities updated
        assert!(snapshot.paid_item_quantities.get("inst-1").is_none() 
                || *snapshot.paid_item_quantities.get("inst-1").unwrap() == 0);
    }

    #[test]
    fn test_cancel_split_payment_creates_new_item_when_not_found() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.total = 100.0;
        snapshot.paid_amount = 30.0;

        // Order has a different item (different instance_id due to discount)
        let modified_item = CartItemSnapshot {
            id: "product:1".to_string(),
            instance_id: "inst-2".to_string(), // Different instance_id after modification
            name: "Coffee (10% off)".to_string(),
            price: 9.0,
            original_price: Some(10.0),
            quantity: 3,
            unpaid_quantity: 3,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: Some(10.0),
            surcharge: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            line_total: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        };
        snapshot.items.push(modified_item);

        // Split payment was for original items (inst-1) before modification
        let original_item = CartItemSnapshot {
            id: "product:1".to_string(),
            instance_id: "inst-1".to_string(), // Original instance_id
            name: "Coffee".to_string(),
            price: 10.0,
            original_price: None,
            quantity: 2,
            unpaid_quantity: 0,
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
        };

        let mut payment = create_payment_record("split-pay-1", "cash", 20.0);
        payment.split_items = Some(vec![original_item]);
        snapshot.payments.push(payment);
        snapshot
            .paid_item_quantities
            .insert("inst-1".to_string(), 2);

        // Cancel the split payment
        let event = create_payment_cancelled_event(
            "order-1",
            1,
            "split-pay-1",
            "cash",
            20.0,
            None,
            None,
            None,
        );

        let applier = PaymentCancelledApplier;
        applier.apply(&mut snapshot, &event);

        // Check: paid_amount reduced
        assert_eq!(snapshot.paid_amount, 10.0); // 30 - 20 = 10

        // Check: new item created (original item added back)
        assert_eq!(snapshot.items.len(), 2);

        // Find the restored item
        let restored = snapshot
            .items
            .iter()
            .find(|i| i.instance_id == "inst-1")
            .expect("Restored item not found");
        assert_eq!(restored.quantity, 2);
        assert_eq!(restored.unpaid_quantity, 2);
        assert_eq!(restored.price, 10.0);
        assert!(restored.manual_discount_percent.is_none());

        // Modified item unchanged
        let modified = snapshot
            .items
            .iter()
            .find(|i| i.instance_id == "inst-2")
            .expect("Modified item should remain");
        assert_eq!(modified.quantity, 3);
    }

    #[test]
    fn test_cancel_normal_payment_no_item_restoration() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.total = 100.0;
        snapshot.paid_amount = 50.0;

        let item = CartItemSnapshot {
            id: "product:1".to_string(),
            instance_id: "inst-1".to_string(),
            name: "Coffee".to_string(),
            price: 10.0,
            original_price: None,
            quantity: 5,
            unpaid_quantity: 5,
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
        };
        snapshot.items.push(item);

        // Normal payment (no split_items)
        snapshot
            .payments
            .push(create_payment_record("pay-1", "cash", 50.0));

        // Cancel normal payment
        let event = create_payment_cancelled_event(
            "order-1",
            1,
            "pay-1",
            "cash",
            50.0,
            None,
            None,
            None,
        );

        let applier = PaymentCancelledApplier;
        applier.apply(&mut snapshot, &event);

        // Check: paid_amount reduced
        assert_eq!(snapshot.paid_amount, 0.0);

        // Check: items unchanged (no split_items to restore)
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].quantity, 5);
        assert_eq!(snapshot.items[0].unpaid_quantity, 5);
    }
}
