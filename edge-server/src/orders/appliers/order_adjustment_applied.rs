//! OrderDiscountApplied + OrderSurchargeApplied event appliers
//!
//! 纯函数：将订单级手动折扣/附加费事件应用到快照。

use crate::orders::money::recalculate_totals;
use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

/// OrderDiscountApplied applier
pub struct OrderDiscountAppliedApplier;

impl EventApplier for OrderDiscountAppliedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::OrderDiscountApplied {
            discount_percent,
            discount_fixed,
            ..
        } = &event.payload
        {
            // 1. Update manual discount fields
            snapshot.order_manual_discount_percent = *discount_percent;
            snapshot.order_manual_discount_fixed = *discount_fixed;

            // 2. Recalculate all totals
            recalculate_totals(snapshot);

            // 3. Update sequence and timestamp
            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;

            // 4. Update checksum
            snapshot.update_checksum();
        }
    }
}

/// OrderSurchargeApplied applier
pub struct OrderSurchargeAppliedApplier;

impl EventApplier for OrderSurchargeAppliedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::OrderSurchargeApplied {
            surcharge_percent,
            surcharge_amount,
            ..
        } = &event.payload
        {
            // 1. Update manual surcharge fields
            snapshot.order_manual_surcharge_percent = *surcharge_percent;
            snapshot.order_manual_surcharge_fixed = *surcharge_amount;

            // 2. Recalculate all totals
            recalculate_totals(snapshot);

            // 3. Update sequence and timestamp
            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;

            // 4. Update checksum
            snapshot.update_checksum();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::{CartItemSnapshot, EventPayload, OrderEventType, OrderStatus};

    fn create_test_item(price: f64, quantity: i32) -> CartItemSnapshot {
        CartItemSnapshot {
            id: 1,
            instance_id: "inst-1".to_string(),
            name: "Test Product".to_string(),
            price,
            original_price: price,
            quantity,
            unpaid_quantity: quantity,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: 0.0,
            rule_surcharge_amount: 0.0,
            applied_rules: vec![],
            unit_price: 0.0,
            line_total: 0.0,
            tax: 0.0,
            tax_rate: 0,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
        is_comped: false,
        }
    }

    fn create_test_snapshot(order_id: &str, items: Vec<CartItemSnapshot>) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items = items;
        recalculate_totals(&mut snapshot);
        snapshot
    }

    fn create_discount_event(
        order_id: &str,
        seq: u64,
        discount_percent: Option<f64>,
        discount_fixed: Option<f64>,
        previous_discount_percent: Option<f64>,
        previous_discount_fixed: Option<f64>,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            1,
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::OrderDiscountApplied,
            EventPayload::OrderDiscountApplied {
                discount_percent,
                discount_fixed,
                previous_discount_percent,
                previous_discount_fixed,
                authorizer_id: None,
                authorizer_name: None,
                subtotal: 0.0,   // applier recalculates
                discount: 0.0,   // applier recalculates
                total: 0.0,      // applier recalculates
            },
        )
    }

    fn create_surcharge_event(
        order_id: &str,
        seq: u64,
        surcharge_amount: Option<f64>,
        previous_surcharge_amount: Option<f64>,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            1,
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::OrderSurchargeApplied,
            EventPayload::OrderSurchargeApplied {
                surcharge_percent: None,
                surcharge_amount,
                previous_surcharge_percent: None,
                previous_surcharge_amount,
                authorizer_id: None,
                authorizer_name: None,
                subtotal: 0.0,
                surcharge: 0.0,
                total: 0.0,
            },
        )
    }

    // ==========================================================
    // OrderDiscountApplied tests
    // ==========================================================

    #[test]
    fn test_apply_percentage_discount() {
        let mut snapshot = create_test_snapshot("order-1", vec![create_test_item(100.0, 1)]);
        assert_eq!(snapshot.total, 100.0);
        assert_eq!(snapshot.order_manual_discount_percent, None);

        let event = create_discount_event("order-1", 2, Some(10.0), None, None, None);

        let applier = OrderDiscountAppliedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.order_manual_discount_percent, Some(10.0));
        assert_eq!(snapshot.order_manual_discount_fixed, None);
        assert_eq!(snapshot.subtotal, 100.0);
        assert_eq!(snapshot.discount, 10.0);
        assert_eq!(snapshot.total, 90.0);
        assert_eq!(snapshot.remaining_amount, 90.0);
        assert_eq!(snapshot.last_sequence, 2);
    }

    #[test]
    fn test_apply_fixed_discount() {
        let mut snapshot = create_test_snapshot("order-1", vec![create_test_item(100.0, 1)]);

        let event = create_discount_event("order-1", 2, None, Some(25.0), None, None);

        let applier = OrderDiscountAppliedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.order_manual_discount_percent, None);
        assert_eq!(snapshot.order_manual_discount_fixed, Some(25.0));
        assert_eq!(snapshot.subtotal, 100.0);
        assert_eq!(snapshot.discount, 25.0);
        assert_eq!(snapshot.total, 75.0);
    }

    #[test]
    fn test_clear_discount() {
        let mut snapshot = create_test_snapshot("order-1", vec![create_test_item(100.0, 1)]);
        snapshot.order_manual_discount_percent = Some(10.0);
        recalculate_totals(&mut snapshot);
        assert_eq!(snapshot.total, 90.0);

        let event = create_discount_event("order-1", 2, None, None, Some(10.0), None);

        let applier = OrderDiscountAppliedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.order_manual_discount_percent, None);
        assert_eq!(snapshot.order_manual_discount_fixed, None);
        assert_eq!(snapshot.discount, 0.0);
        assert_eq!(snapshot.total, 100.0);
    }

    #[test]
    fn test_replace_percent_with_fixed() {
        let mut snapshot = create_test_snapshot("order-1", vec![create_test_item(200.0, 1)]);
        snapshot.order_manual_discount_percent = Some(10.0);
        recalculate_totals(&mut snapshot);
        assert_eq!(snapshot.total, 180.0);

        // Replace 10% with fixed 50
        let event = create_discount_event("order-1", 2, None, Some(50.0), Some(10.0), None);

        let applier = OrderDiscountAppliedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.order_manual_discount_percent, None);
        assert_eq!(snapshot.order_manual_discount_fixed, Some(50.0));
        assert_eq!(snapshot.discount, 50.0);
        assert_eq!(snapshot.total, 150.0);
    }

    #[test]
    fn test_discount_updates_checksum() {
        let mut snapshot = create_test_snapshot("order-1", vec![create_test_item(100.0, 1)]);
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_discount_event("order-1", 2, Some(10.0), None, None, None);

        let applier = OrderDiscountAppliedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_discount_with_multiple_items() {
        let mut snapshot = create_test_snapshot(
            "order-1",
            vec![create_test_item(50.0, 2), create_test_item(30.0, 3)],
        );
        // subtotal = 50*2 + 30*3 = 100 + 90 = 190
        assert_eq!(snapshot.subtotal, 190.0);
        assert_eq!(snapshot.total, 190.0);

        // 10% discount
        let event = create_discount_event("order-1", 2, Some(10.0), None, None, None);

        let applier = OrderDiscountAppliedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.subtotal, 190.0);
        assert_eq!(snapshot.discount, 19.0);
        assert_eq!(snapshot.total, 171.0);
    }

    #[test]
    fn test_discount_with_paid_amount() {
        let mut snapshot = create_test_snapshot("order-1", vec![create_test_item(100.0, 1)]);
        snapshot.paid_amount = 30.0;
        recalculate_totals(&mut snapshot);
        assert_eq!(snapshot.remaining_amount, 70.0);

        // Apply 10% discount: total = 90, remaining = 90 - 30 = 60
        let event = create_discount_event("order-1", 2, Some(10.0), None, None, None);

        let applier = OrderDiscountAppliedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.total, 90.0);
        assert_eq!(snapshot.remaining_amount, 60.0);
    }

    // ==========================================================
    // OrderSurchargeApplied tests
    // ==========================================================

    #[test]
    fn test_apply_surcharge() {
        let mut snapshot = create_test_snapshot("order-1", vec![create_test_item(100.0, 1)]);
        assert_eq!(snapshot.total, 100.0);

        let event = create_surcharge_event("order-1", 2, Some(15.0), None);

        let applier = OrderSurchargeAppliedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.order_manual_surcharge_fixed, Some(15.0));
        assert_eq!(snapshot.subtotal, 100.0);
        assert_eq!(snapshot.total, 115.0);
        assert_eq!(snapshot.remaining_amount, 115.0);
        assert_eq!(snapshot.last_sequence, 2);
    }

    #[test]
    fn test_clear_surcharge() {
        let mut snapshot = create_test_snapshot("order-1", vec![create_test_item(100.0, 1)]);
        snapshot.order_manual_surcharge_fixed = Some(15.0);
        recalculate_totals(&mut snapshot);
        assert_eq!(snapshot.total, 115.0);

        let event = create_surcharge_event("order-1", 2, None, Some(15.0));

        let applier = OrderSurchargeAppliedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.order_manual_surcharge_fixed, None);
        assert_eq!(snapshot.total, 100.0);
    }

    #[test]
    fn test_surcharge_updates_checksum() {
        let mut snapshot = create_test_snapshot("order-1", vec![create_test_item(100.0, 1)]);
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_surcharge_event("order-1", 2, Some(10.0), None);

        let applier = OrderSurchargeAppliedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_surcharge_with_paid_amount() {
        let mut snapshot = create_test_snapshot("order-1", vec![create_test_item(100.0, 1)]);
        snapshot.paid_amount = 50.0;
        recalculate_totals(&mut snapshot);

        let event = create_surcharge_event("order-1", 2, Some(20.0), None);

        let applier = OrderSurchargeAppliedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.total, 120.0);
        assert_eq!(snapshot.remaining_amount, 70.0); // 120 - 50
    }

    // ==========================================================
    // Discount + Surcharge coexistence tests
    // ==========================================================

    #[test]
    fn test_discount_and_surcharge_coexist() {
        let mut snapshot = create_test_snapshot("order-1", vec![create_test_item(100.0, 2)]);
        // subtotal = 200

        // 先加附加费
        let surcharge_event = create_surcharge_event("order-1", 2, Some(20.0), None);
        let surcharge_applier = OrderSurchargeAppliedApplier;
        surcharge_applier.apply(&mut snapshot, &surcharge_event);

        assert_eq!(snapshot.total, 220.0); // 200 + 20

        // 再加 10% 折扣
        let discount_event = create_discount_event("order-1", 3, Some(10.0), None, None, None);
        let discount_applier = OrderDiscountAppliedApplier;
        discount_applier.apply(&mut snapshot, &discount_event);

        // total = subtotal(200) - discount(10% of 200 = 20) + surcharge(20) = 200
        assert_eq!(snapshot.subtotal, 200.0);
        assert_eq!(snapshot.discount, 20.0);
        assert_eq!(snapshot.total, 200.0);
    }

    #[test]
    fn test_surcharge_with_existing_discount() {
        let mut snapshot = create_test_snapshot("order-1", vec![create_test_item(100.0, 1)]);
        snapshot.order_manual_discount_fixed = Some(10.0);
        recalculate_totals(&mut snapshot);
        assert_eq!(snapshot.total, 90.0); // 100 - 10

        // 加附加费
        let event = create_surcharge_event("order-1", 2, Some(25.0), None);
        let applier = OrderSurchargeAppliedApplier;
        applier.apply(&mut snapshot, &event);

        // total = 100 - 10 + 25 = 115
        assert_eq!(snapshot.total, 115.0);
    }

    #[test]
    fn test_replace_surcharge_amount() {
        let mut snapshot = create_test_snapshot("order-1", vec![create_test_item(100.0, 1)]);
        snapshot.order_manual_surcharge_fixed = Some(10.0);
        recalculate_totals(&mut snapshot);
        assert_eq!(snapshot.total, 110.0);

        // 替换为更大的附加费
        let event = create_surcharge_event("order-1", 2, Some(30.0), Some(10.0));
        let applier = OrderSurchargeAppliedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.order_manual_surcharge_fixed, Some(30.0));
        assert_eq!(snapshot.total, 130.0);
    }

    #[test]
    fn test_discount_with_rule_surcharge() {
        let mut snapshot = create_test_snapshot("order-1", vec![create_test_item(100.0, 1)]);
        // 模拟规则附加费
        snapshot.order_rule_surcharge_amount = 8.0;
        recalculate_totals(&mut snapshot);
        assert_eq!(snapshot.total, 108.0); // 100 + 8

        // 加 10% 手动折扣
        let event = create_discount_event("order-1", 2, Some(10.0), None, None, None);
        let applier = OrderDiscountAppliedApplier;
        applier.apply(&mut snapshot, &event);

        // total = 100 - 10 + 8 = 98
        assert_eq!(snapshot.discount, 10.0);
        assert_eq!(snapshot.total, 98.0);
    }
}
