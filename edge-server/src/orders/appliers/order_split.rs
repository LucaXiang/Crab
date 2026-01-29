//! Split event appliers
//!
//! Five appliers for the split payment system:
//! - `ItemSplitApplier` — 菜品分单
//! - `AmountSplitApplier` — 金额分单
//! - `AaSplitStartedApplier` — AA 开始（锁人数）
//! - `AaSplitPaidApplier` — AA 支付（进度）
//! - `AaSplitCancelledApplier` — AA 取消（解锁）

use crate::orders::money::{self, to_decimal, to_f64, MONEY_TOLERANCE};
use crate::orders::traits::EventApplier;
use shared::order::{
    CartItemSnapshot, EventPayload, OrderEvent, OrderSnapshot, PaymentRecord, SplitType,
};

// ============================================================================
// ItemSplit applier
// ============================================================================

pub struct ItemSplitApplier;

impl EventApplier for ItemSplitApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::ItemSplit {
            payment_id,
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
            snapshot.paid_amount =
                to_f64(to_decimal(snapshot.paid_amount) + to_decimal(*split_amount));

            // Create payment record note
            let item_names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
            let note = if items.is_empty() {
                None
            } else {
                Some(format!("Split: {}", item_names.join(", ")))
            };

            // Build split_items snapshot for restoration on cancel
            let split_items: Vec<CartItemSnapshot> = items
                .iter()
                .filter_map(|split_item| {
                    snapshot
                        .items
                        .iter()
                        .find(|item| item.instance_id == split_item.instance_id)
                        .map(|item| {
                            let mut item_snapshot = item.clone();
                            item_snapshot.quantity = split_item.quantity;
                            item_snapshot.unpaid_quantity = 0;
                            item_snapshot
                        })
                })
                .collect();

            let payment = PaymentRecord {
                payment_id: payment_id.clone(),
                method: payment_method.clone(),
                amount: *split_amount,
                tendered: None,
                change: None,
                note,
                timestamp: event.timestamp,
                cancelled: false,
                cancel_reason: None,
                split_items: Some(split_items),
                aa_shares: None,
                split_type: Some(SplitType::ItemSplit),
            };
            snapshot.payments.push(payment);

            // When fully paid after item-based split, mark all items as paid
            if !items.is_empty()
                && to_decimal(snapshot.paid_amount) >= to_decimal(snapshot.total) - MONEY_TOLERANCE
            {
                let item_quantities: Vec<(String, i32)> = snapshot
                    .items
                    .iter()
                    .map(|item| (item.instance_id.clone(), item.quantity))
                    .collect();
                for (instance_id, quantity) in item_quantities {
                    snapshot.paid_item_quantities.insert(instance_id, quantity);
                }
            }

            // Recalculate totals
            money::recalculate_totals(snapshot);

            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;
            snapshot.update_checksum();
        }
    }
}

// ============================================================================
// AmountSplit applier
// ============================================================================

pub struct AmountSplitApplier;

impl EventApplier for AmountSplitApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::AmountSplit {
            payment_id,
            split_amount,
            payment_method,
        } = &event.payload
        {
            // Update paid amount
            snapshot.paid_amount =
                to_f64(to_decimal(snapshot.paid_amount) + to_decimal(*split_amount));

            // Set amount split flag
            snapshot.has_amount_split = true;

            let payment = PaymentRecord {
                payment_id: payment_id.clone(),
                method: payment_method.clone(),
                amount: *split_amount,
                tendered: None,
                change: None,
                note: None,
                timestamp: event.timestamp,
                cancelled: false,
                cancel_reason: None,
                split_items: Some(vec![]), // Empty vec signals amount-based split for rollback detection
                aa_shares: None,
                split_type: Some(SplitType::AmountSplit),
            };
            snapshot.payments.push(payment);

            money::recalculate_totals(snapshot);

            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;
            snapshot.update_checksum();
        }
    }
}

// ============================================================================
// AaSplitStarted applier
// ============================================================================

pub struct AaSplitStartedApplier;

impl EventApplier for AaSplitStartedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::AaSplitStarted { total_shares, .. } = &event.payload {
            // Lock total shares
            snapshot.aa_total_shares = Some(*total_shares);

            // No PaymentRecord — this event only locks the headcount

            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;
            snapshot.update_checksum();
        }
    }
}

// ============================================================================
// AaSplitPaid applier
// ============================================================================

pub struct AaSplitPaidApplier;

impl EventApplier for AaSplitPaidApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::AaSplitPaid {
            payment_id,
            shares,
            amount,
            payment_method,
            ..
        } = &event.payload
        {
            // Update paid amount
            snapshot.paid_amount =
                to_f64(to_decimal(snapshot.paid_amount) + to_decimal(*amount));

            // Update AA paid shares
            snapshot.aa_paid_shares += shares;

            let payment = PaymentRecord {
                payment_id: payment_id.clone(),
                method: payment_method.clone(),
                amount: *amount,
                tendered: None,
                change: None,
                note: None,
                timestamp: event.timestamp,
                cancelled: false,
                cancel_reason: None,
                split_items: None,
                aa_shares: Some(*shares),
                split_type: Some(SplitType::AaSplit),
            };
            snapshot.payments.push(payment);

            money::recalculate_totals(snapshot);

            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;
            snapshot.update_checksum();
        }
    }
}

// ============================================================================
// AaSplitCancelled applier
// ============================================================================

pub struct AaSplitCancelledApplier;

impl EventApplier for AaSplitCancelledApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::AaSplitCancelled { .. } = &event.payload {
            // Unlock AA mode
            snapshot.aa_total_shares = None;
            // aa_paid_shares should already be 0 at this point (all AA payments cancelled)

            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;
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
            tax_rate: None,
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
            tax_rate: None,
        };
        snapshot.items.push(item1);
        snapshot.items.push(item2);
        snapshot.subtotal = 46.0;
        snapshot.total = 46.0;

        snapshot
    }

    // ========== ItemSplit tests ==========

    #[test]
    fn test_item_split_updates_paid_quantities() {
        let mut snapshot = create_test_snapshot("order-1");
        assert!(snapshot.paid_item_quantities.is_empty());

        let event = OrderEvent::new(
            2,
            "order-1".to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::ItemSplit,
            EventPayload::ItemSplit {
                payment_id: uuid::Uuid::new_v4().to_string(),
                split_amount: 20.0,
                payment_method: "CASH".to_string(),
                items: vec![SplitItem {
                    instance_id: "item-1".to_string(),
                    name: "Coffee".to_string(),
                    quantity: 2,
                    unit_price: 10.0,
                }],
            },
        );

        let applier = ItemSplitApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.paid_item_quantities.get("item-1"), Some(&2));
        assert_eq!(snapshot.paid_amount, 20.0);
        assert_eq!(snapshot.payments.len(), 1);
        assert_eq!(
            snapshot.payments[0].split_type,
            Some(SplitType::ItemSplit)
        );
    }

    #[test]
    fn test_item_split_creates_payment_with_note() {
        let mut snapshot = create_test_snapshot("order-1");

        let event = OrderEvent::new(
            2,
            "order-1".to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::ItemSplit,
            EventPayload::ItemSplit {
                payment_id: "pay-1".to_string(),
                split_amount: 28.0,
                payment_method: "CARD".to_string(),
                items: vec![
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
            },
        );

        let applier = ItemSplitApplier;
        applier.apply(&mut snapshot, &event);

        let payment = &snapshot.payments[0];
        assert_eq!(payment.note, Some("Split: Coffee, Tea".to_string()));
        assert_eq!(payment.method, "CARD");
    }

    // ========== AmountSplit tests ==========

    #[test]
    fn test_amount_split_updates_paid_amount() {
        let mut snapshot = create_test_snapshot("order-1");
        assert_eq!(snapshot.paid_amount, 0.0);
        assert!(!snapshot.has_amount_split);

        let event = OrderEvent::new(
            2,
            "order-1".to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::AmountSplit,
            EventPayload::AmountSplit {
                payment_id: "pay-1".to_string(),
                split_amount: 20.0,
                payment_method: "CASH".to_string(),
            },
        );

        let applier = AmountSplitApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.paid_amount, 20.0);
        assert!(snapshot.has_amount_split);
        assert_eq!(snapshot.payments.len(), 1);
        assert_eq!(
            snapshot.payments[0].split_type,
            Some(SplitType::AmountSplit)
        );
        assert!(snapshot.paid_item_quantities.is_empty());
    }

    // ========== AaSplitStarted tests ==========

    #[test]
    fn test_aa_split_started_locks_shares() {
        let mut snapshot = create_test_snapshot("order-1");
        assert!(snapshot.aa_total_shares.is_none());

        let event = OrderEvent::new(
            2,
            "order-1".to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::AaSplitStarted,
            EventPayload::AaSplitStarted {
                total_shares: 3,
                per_share_amount: 15.33,
                order_total: 46.0,
            },
        );

        let applier = AaSplitStartedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.aa_total_shares, Some(3));
        // No payment record created
        assert!(snapshot.payments.is_empty());
        assert_eq!(snapshot.paid_amount, 0.0);
    }

    // ========== AaSplitPaid tests ==========

    #[test]
    fn test_aa_split_paid_creates_payment() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.aa_total_shares = Some(3);

        let event = OrderEvent::new(
            2,
            "order-1".to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::AaSplitPaid,
            EventPayload::AaSplitPaid {
                payment_id: "pay-1".to_string(),
                shares: 1,
                amount: 15.33,
                payment_method: "CASH".to_string(),
                progress_paid: 1,
                progress_total: 3,
            },
        );

        let applier = AaSplitPaidApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.paid_amount, 15.33);
        assert_eq!(snapshot.aa_paid_shares, 1);
        assert_eq!(snapshot.payments.len(), 1);
        assert_eq!(snapshot.payments[0].aa_shares, Some(1));
        assert_eq!(snapshot.payments[0].split_type, Some(SplitType::AaSplit));
    }

    // ========== AaSplitCancelled tests ==========

    #[test]
    fn test_aa_split_cancelled_unlocks() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.aa_total_shares = Some(3);
        snapshot.aa_paid_shares = 0;

        let event = OrderEvent::new(
            2,
            "order-1".to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::AaSplitCancelled,
            EventPayload::AaSplitCancelled { total_shares: 3 },
        );

        let applier = AaSplitCancelledApplier;
        applier.apply(&mut snapshot, &event);

        assert!(snapshot.aa_total_shares.is_none());
    }
}
