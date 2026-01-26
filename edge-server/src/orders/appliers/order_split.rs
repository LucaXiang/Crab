//! OrderSplit event applier
//!
//! Applies the OrderSplit event to track paid item quantities and update paid amount.
//! This is used for split bill payments where specific items are paid for.

use crate::orders::money::{self, to_decimal, to_f64};
use crate::orders::traits::EventApplier;
use shared::order::{CartItemSnapshot, EventPayload, OrderEvent, OrderSnapshot, PaymentRecord};

/// OrderSplit applier
pub struct OrderSplitApplier;

impl EventApplier for OrderSplitApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::OrderSplit {
            split_amount,
            payment_method,
            items,
        } = &event.payload
        {
            // Track paid quantities for each item
            for split_item in items {
                *snapshot
                    .paid_item_quantities
                    .entry(split_item.instance_id.clone())
                    .or_insert(0) += split_item.quantity;
            }

            // Update paid amount using Decimal for precision
            snapshot.paid_amount = to_f64(to_decimal(snapshot.paid_amount) + to_decimal(*split_amount));

            // Create PaymentRecord for audit trail
            let item_names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
            let note = if items.is_empty() {
                None
            } else {
                Some(format!("Split: {}", item_names.join(", ")))
            };

            // Build split_items snapshot for restoration on cancel
            // Find each item in snapshot and create a snapshot with the split quantity
            let split_items: Vec<CartItemSnapshot> = items
                .iter()
                .filter_map(|split_item| {
                    snapshot
                        .items
                        .iter()
                        .find(|item| item.instance_id == split_item.instance_id)
                        .map(|item| {
                            let mut item_snapshot = item.clone();
                            // Set quantity to the split quantity (not the full item quantity)
                            item_snapshot.quantity = split_item.quantity;
                            item_snapshot.unpaid_quantity = 0; // These items are paid
                            item_snapshot
                        })
                })
                .collect();

            let payment = PaymentRecord {
                payment_id: format!("split-{}", event.event_id),
                method: payment_method.clone(),
                amount: *split_amount,
                tendered: None,
                change: None,
                note,
                timestamp: event.timestamp,
                cancelled: false,
                cancel_reason: None,
                split_items: if split_items.is_empty() {
                    None
                } else {
                    Some(split_items)
                },
            };
            snapshot.payments.push(payment);

            // Recalculate totals to update unpaid_quantity for each item
            money::recalculate_totals(snapshot);

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
    use shared::order::{CartItemSnapshot, OrderEventType, OrderStatus, SplitItem};

    fn create_test_snapshot(order_id: &str) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.table_id = Some("dining_table:t1".to_string());
        snapshot.table_name = Some("Table 1".to_string());

        // Add items
        let item1 = CartItemSnapshot {
            id: "product:1".to_string(),
            instance_id: "item-1".to_string(),
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
            unit_price: None,
            line_total: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            tax: None,
        };
        let item2 = CartItemSnapshot {
            id: "product:2".to_string(),
            instance_id: "item-2".to_string(),
            name: "Tea".to_string(),
            price: 8.0,
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
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            tax: None,
        };
        snapshot.items.push(item1);
        snapshot.items.push(item2);
        snapshot.subtotal = 46.0;
        snapshot.total = 46.0;

        snapshot
    }

    fn create_order_split_event(
        order_id: &str,
        seq: u64,
        split_amount: f64,
        payment_method: &str,
        items: Vec<SplitItem>,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::OrderSplit,
            EventPayload::OrderSplit {
                split_amount,
                payment_method: payment_method.to_string(),
                items,
            },
        )
    }

    #[test]
    fn test_order_split_updates_paid_quantities() {
        let mut snapshot = create_test_snapshot("order-1");
        assert!(snapshot.paid_item_quantities.is_empty());

        let event = create_order_split_event(
            "order-1",
            2,
            20.0,
            "CASH",
            vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 2,
                unit_price: 10.0,
            }],
        );

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.paid_item_quantities.get("item-1"), Some(&2));
    }

    #[test]
    fn test_order_split_updates_unpaid_quantity() {
        let mut snapshot = create_test_snapshot("order-1");
        // Items have unpaid_quantity = quantity initially
        assert_eq!(snapshot.items[0].unpaid_quantity, 3); // Coffee: qty=3
        assert_eq!(snapshot.items[1].unpaid_quantity, 2); // Tea: qty=2

        let event = create_order_split_event(
            "order-1",
            2,
            20.0,
            "CASH",
            vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 2,
                unit_price: 10.0,
            }],
        );

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        // unpaid_quantity should be updated: 3 - 2 = 1
        assert_eq!(snapshot.items[0].unpaid_quantity, 1);
        // item-2 unchanged
        assert_eq!(snapshot.items[1].unpaid_quantity, 2);
    }

    #[test]
    fn test_order_split_creates_payment_record() {
        let mut snapshot = create_test_snapshot("order-1");
        assert!(snapshot.payments.is_empty());

        let event = create_order_split_event(
            "order-1",
            2,
            20.0,
            "CASH",
            vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 2,
                unit_price: 10.0,
            }],
        );

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        // PaymentRecord should be created
        assert_eq!(snapshot.payments.len(), 1);
        let payment = &snapshot.payments[0];
        assert!(payment.payment_id.starts_with("split-"));
        assert_eq!(payment.method, "CASH");
        assert_eq!(payment.amount, 20.0);
        assert_eq!(payment.note, Some("Split: Coffee".to_string()));
        assert!(!payment.cancelled);
    }

    #[test]
    fn test_order_split_payment_record_multiple_items() {
        let mut snapshot = create_test_snapshot("order-1");

        let event = create_order_split_event(
            "order-1",
            2,
            28.0,
            "CARD",
            vec![
                SplitItem {
                    instance_id: "item-1".to_string(),
                    name: "Coffee".to_string(),
                    quantity: 2,
                    unit_price: 10.0,
                },
                SplitItem {
                    instance_id: "item-2".to_string(),
                    name: "Tea".to_string(),
                    quantity: 1,
                    unit_price: 8.0,
                },
            ],
        );

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.payments.len(), 1);
        let payment = &snapshot.payments[0];
        assert_eq!(payment.method, "CARD");
        assert_eq!(payment.amount, 28.0);
        assert_eq!(payment.note, Some("Split: Coffee, Tea".to_string()));
    }

    #[test]
    fn test_order_split_multiple_items() {
        let mut snapshot = create_test_snapshot("order-1");

        let event = create_order_split_event(
            "order-1",
            2,
            28.0,
            "CARD",
            vec![
                SplitItem {
                    instance_id: "item-1".to_string(),
                    name: "Coffee".to_string(),
                    quantity: 2,
                    unit_price: 10.0,
                },
                SplitItem {
                    instance_id: "item-2".to_string(),
                    name: "Tea".to_string(),
                    quantity: 1,
                    unit_price: 8.0,
                },
            ],
        );

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.paid_item_quantities.get("item-1"), Some(&2));
        assert_eq!(snapshot.paid_item_quantities.get("item-2"), Some(&1));
    }

    #[test]
    fn test_order_split_updates_paid_amount() {
        let mut snapshot = create_test_snapshot("order-1");
        assert_eq!(snapshot.paid_amount, 0.0);

        let event = create_order_split_event(
            "order-1",
            2,
            20.0,
            "CASH",
            vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 2,
                unit_price: 10.0,
            }],
        );

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.paid_amount, 20.0);
    }

    #[test]
    fn test_order_split_accumulates_paid_quantities() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot
            .paid_item_quantities
            .insert("item-1".to_string(), 1);
        snapshot.paid_amount = 10.0;

        let event = create_order_split_event(
            "order-1",
            2,
            20.0,
            "CASH",
            vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 2,
                unit_price: 10.0,
            }],
        );

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        // 1 + 2 = 3
        assert_eq!(snapshot.paid_item_quantities.get("item-1"), Some(&3));
        // 10 + 20 = 30
        assert_eq!(snapshot.paid_amount, 30.0);
    }

    #[test]
    fn test_order_split_updates_sequence() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.last_sequence = 5;

        let event = create_order_split_event("order-1", 10, 20.0, "CASH", vec![]);

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 10);
    }

    #[test]
    fn test_order_split_updates_timestamp() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.updated_at = 1000000000;

        let event = create_order_split_event("order-1", 2, 20.0, "CASH", vec![]);
        let expected_timestamp = event.timestamp;

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.updated_at, expected_timestamp);
        assert_ne!(snapshot.updated_at, 1000000000);
    }

    #[test]
    fn test_order_split_updates_checksum() {
        let mut snapshot = create_test_snapshot("order-1");
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_order_split_event(
            "order-1",
            2,
            20.0,
            "CASH",
            vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 1,
                unit_price: 10.0,
            }],
        );

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_order_split_empty_items() {
        let mut snapshot = create_test_snapshot("order-1");
        let initial_paid_quantities = snapshot.paid_item_quantities.clone();

        let event = create_order_split_event(
            "order-1",
            2,
            20.0,
            "CASH",
            vec![], // Empty items
        );

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        // paid_item_quantities should remain unchanged
        assert_eq!(snapshot.paid_item_quantities, initial_paid_quantities);
        // But paid_amount should still be updated
        assert_eq!(snapshot.paid_amount, 20.0);
    }

    #[test]
    fn test_order_split_preserves_items() {
        let mut snapshot = create_test_snapshot("order-1");
        let original_items_len = snapshot.items.len();

        let event = create_order_split_event(
            "order-1",
            2,
            20.0,
            "CASH",
            vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 2,
                unit_price: 10.0,
            }],
        );

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        // Items in snapshot should remain unchanged
        assert_eq!(snapshot.items.len(), original_items_len);
        assert_eq!(snapshot.items[0].instance_id, "item-1");
        assert_eq!(snapshot.items[0].quantity, 3); // Original quantity preserved
    }

    #[test]
    fn test_order_split_preserves_status() {
        let mut snapshot = create_test_snapshot("order-1");
        assert_eq!(snapshot.status, OrderStatus::Active);

        let event = create_order_split_event(
            "order-1",
            2,
            20.0,
            "CASH",
            vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 2,
                unit_price: 10.0,
            }],
        );

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.status, OrderStatus::Active);
    }

    #[test]
    fn test_order_split_wrong_event_type_is_noop() {
        let mut snapshot = create_test_snapshot("order-1");
        let original_paid_amount = snapshot.paid_amount;
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

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        // Nothing should change
        assert_eq!(snapshot.paid_amount, original_paid_amount);
        assert_eq!(snapshot.last_sequence, original_sequence);
    }

    #[test]
    fn test_order_split_checksum_verifiable() {
        let mut snapshot = create_test_snapshot("order-1");

        let event = create_order_split_event(
            "order-1",
            2,
            20.0,
            "CASH",
            vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 1,
                unit_price: 10.0,
            }],
        );

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        assert!(snapshot.verify_checksum());

        // Tampering should invalidate checksum
        snapshot.last_sequence = 999;
        assert!(!snapshot.verify_checksum());
    }

    #[test]
    fn test_order_split_different_payment_methods() {
        let mut snapshot = create_test_snapshot("order-1");

        let event = create_order_split_event(
            "order-1",
            2,
            20.0,
            "wechat", // Different payment method
            vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 1,
                unit_price: 10.0,
            }],
        );

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        // Payment method doesn't affect the snapshot directly
        // It's stored in the event for audit purposes
        assert_eq!(snapshot.paid_amount, 20.0);
        assert_eq!(snapshot.paid_item_quantities.get("item-1"), Some(&1));
    }

    #[test]
    fn test_order_split_zero_quantity_item() {
        let mut snapshot = create_test_snapshot("order-1");

        let event = create_order_split_event(
            "order-1",
            2,
            0.0,
            "CASH",
            vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 0, // Zero quantity
                unit_price: 10.0,
            }],
        );

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        // Zero quantity is added (no-op in terms of value change)
        assert_eq!(snapshot.paid_item_quantities.get("item-1"), Some(&0));
    }

    #[test]
    fn test_order_split_preserves_table_info() {
        let mut snapshot = create_test_snapshot("order-1");
        let original_table_id = snapshot.table_id.clone();
        let original_table_name = snapshot.table_name.clone();

        let event = create_order_split_event(
            "order-1",
            2,
            20.0,
            "CASH",
            vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 1,
                unit_price: 10.0,
            }],
        );

        let applier = OrderSplitApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.table_id, original_table_id);
        assert_eq!(snapshot.table_name, original_table_name);
    }
}
