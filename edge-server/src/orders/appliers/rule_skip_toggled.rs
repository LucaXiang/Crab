//! RuleSkipToggled event applier
//!
//! Applies the RuleSkipToggled event to toggle a rule's skipped status
//! and recalculate order totals using precise decimal arithmetic.

use crate::orders::money;
use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

/// RuleSkipToggled applier
pub struct RuleSkipToggledApplier;

impl EventApplier for RuleSkipToggledApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::RuleSkipToggled {
            rule_id,
            skipped,
        } = &event.payload
        {
            // 1. Update the skipped status on all items' applied_rules with matching rule_id
            for item in &mut snapshot.items {
                if let Some(ref mut applied_rules) = item.applied_rules {
                    for rule in applied_rules.iter_mut() {
                        if rule.rule_id == *rule_id {
                            rule.skipped = *skipped;
                        }
                    }
                }
            }

            // 2. Update order-level applied rules with matching rule_id
            if let Some(ref mut order_rules) = snapshot.order_applied_rules {
                for rule in order_rules.iter_mut() {
                    if rule.rule_id == *rule_id {
                        rule.skipped = *skipped;
                    }
                }
            }

            // 3. Update sequence and timestamp
            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;

            // 4. Recalculate totals using precise decimal arithmetic
            money::recalculate_totals(snapshot);

            // 5. Update checksum
            snapshot.update_checksum();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::{AppliedRule, CartItemSnapshot, OrderEventType, OrderStatus};

    fn create_test_item_with_rule(
        instance_id: &str,
        price: f64,
        original_price: f64,
        quantity: i32,
        rule_id: &str,
        rule_type: shared::models::price_rule::RuleType,
        adjustment_value: f64,
        calculated_amount: f64,
    ) -> CartItemSnapshot {
        CartItemSnapshot {
            id: "product:1".to_string(),
            instance_id: instance_id.to_string(),
            name: "Test Product".to_string(),
            price,
            original_price: Some(original_price),
            quantity,
            unpaid_quantity: quantity,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: if rule_type == shared::models::price_rule::RuleType::Discount {
                Some(calculated_amount)
            } else {
                None
            },
            rule_surcharge_amount: if rule_type == shared::models::price_rule::RuleType::Surcharge {
                Some(calculated_amount)
            } else {
                None
            },
            applied_rules: Some(vec![AppliedRule {
                rule_id: rule_id.to_string(),
                name: "test_rule".to_string(),
                display_name: "Test Rule".to_string(),
                receipt_name: "TEST".to_string(),
                rule_type,
                adjustment_type: shared::models::price_rule::AdjustmentType::Percentage,
                product_scope: shared::models::price_rule::ProductScope::Global,
                zone_scope: "zone:all".to_string(),
                adjustment_value,
                calculated_amount,
                is_stackable: true,
                is_exclusive: false,
                skipped: false,
            }]),
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
        }
    }

    fn create_rule_skip_toggled_event(
        order_id: &str,
        seq: u64,
        rule_id: &str,
        skipped: bool,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::RuleSkipToggled,
            EventPayload::RuleSkipToggled {
                rule_id: rule_id.to_string(),
                skipped,
            },
        )
    }

    #[test]
    fn test_rule_skip_toggled_skip_discount_rule() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        // Item: original_price 100, rule_discount 10 → price 90, subtotal 90
        snapshot.items.push(create_test_item_with_rule(
            "inst-1", 90.0, 100.0, 1,
            "rule-1", shared::models::price_rule::RuleType::Discount, 10.0, 10.0,
        ));
        snapshot.subtotal = 90.0;
        snapshot.discount = 10.0;
        snapshot.total = 90.0;

        // Skip the rule
        let event = create_rule_skip_toggled_event("order-1", 2, "rule-1", true);
        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        // Rule should be marked as skipped
        assert!(snapshot.items[0].applied_rules.as_ref().unwrap()[0].skipped);

        // Totals should be recalculated: discount rule skipped → full price
        // recalculate_totals uses calculate_unit_price which checks skipped
        assert_eq!(snapshot.subtotal, 100.0);
        assert_eq!(snapshot.total, 100.0);
        assert_eq!(snapshot.last_sequence, 2);
    }

    #[test]
    fn test_rule_skip_toggled_unskip_discount_rule() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        // Item starts with skipped rule
        let mut item = create_test_item_with_rule(
            "inst-1", 100.0, 100.0, 1,
            "rule-1", shared::models::price_rule::RuleType::Discount, 10.0, 10.0,
        );
        item.applied_rules.as_mut().unwrap()[0].skipped = true;
        snapshot.items.push(item);
        snapshot.subtotal = 100.0;
        snapshot.discount = 0.0;
        snapshot.total = 100.0;

        // Unskip the rule
        let event = create_rule_skip_toggled_event("order-1", 2, "rule-1", false);
        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        // Rule should be unskipped
        assert!(!snapshot.items[0].applied_rules.as_ref().unwrap()[0].skipped);

        // Discount should now apply: 100 - 10 = 90
        assert_eq!(snapshot.subtotal, 90.0);
        assert_eq!(snapshot.total, 90.0);
    }

    #[test]
    fn test_rule_skip_toggled_order_level_rule() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        // Simple item without item-level rules
        snapshot.items.push(CartItemSnapshot {
            id: "product:1".to_string(),
            instance_id: "inst-1".to_string(),
            name: "Test Product".to_string(),
            price: 100.0,
            original_price: Some(100.0),
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
        });

        // Order-level rule
        snapshot.order_applied_rules = Some(vec![AppliedRule {
            rule_id: "order-rule-1".to_string(),
            name: "order_discount".to_string(),
            display_name: "Order Discount".to_string(),
            receipt_name: "ORD".to_string(),
            rule_type: shared::models::price_rule::RuleType::Discount,
            adjustment_type: shared::models::price_rule::AdjustmentType::Percentage,
            product_scope: shared::models::price_rule::ProductScope::Global,
            zone_scope: "zone:all".to_string(),
            adjustment_value: 10.0,
            calculated_amount: 10.0,
            is_stackable: true,
            is_exclusive: false,
            skipped: false,
        }]);
        snapshot.subtotal = 100.0;
        snapshot.total = 90.0;
        snapshot.order_rule_discount_amount = Some(10.0);

        // Skip the order-level rule
        let event = create_rule_skip_toggled_event("order-1", 2, "order-rule-1", true);
        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        // Order-level rule should be skipped
        assert!(snapshot.order_applied_rules.as_ref().unwrap()[0].skipped);
    }

    #[test]
    fn test_rule_skip_toggled_nonexistent_rule() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items.push(create_test_item_with_rule(
            "inst-1", 90.0, 100.0, 1,
            "rule-1", shared::models::price_rule::RuleType::Discount, 10.0, 10.0,
        ));
        let original_skipped = snapshot.items[0].applied_rules.as_ref().unwrap()[0].skipped;

        // Toggle nonexistent rule
        let event = create_rule_skip_toggled_event("order-1", 2, "nonexistent", true);
        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        // Existing rule unchanged
        assert_eq!(
            snapshot.items[0].applied_rules.as_ref().unwrap()[0].skipped,
            original_skipped
        );
    }

    #[test]
    fn test_rule_skip_toggled_updates_sequence() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items.push(create_test_item_with_rule(
            "inst-1", 90.0, 100.0, 1,
            "rule-1", shared::models::price_rule::RuleType::Discount, 10.0, 10.0,
        ));
        snapshot.last_sequence = 5;

        let event = create_rule_skip_toggled_event("order-1", 10, "rule-1", true);
        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 10);
    }

    #[test]
    fn test_rule_skip_toggled_updates_checksum() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items.push(create_test_item_with_rule(
            "inst-1", 90.0, 100.0, 1,
            "rule-1", shared::models::price_rule::RuleType::Discount, 10.0, 10.0,
        ));
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_rule_skip_toggled_event("order-1", 2, "rule-1", true);
        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_rule_skip_toggled_multiple_items_same_rule() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        // Two items with the same rule
        snapshot.items.push(create_test_item_with_rule(
            "inst-1", 90.0, 100.0, 1,
            "rule-1", shared::models::price_rule::RuleType::Discount, 10.0, 10.0,
        ));
        snapshot.items.push(create_test_item_with_rule(
            "inst-2", 45.0, 50.0, 2,
            "rule-1", shared::models::price_rule::RuleType::Discount, 10.0, 5.0,
        ));

        // Skip the rule
        let event = create_rule_skip_toggled_event("order-1", 2, "rule-1", true);
        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        // Both items' rules should be skipped
        assert!(snapshot.items[0].applied_rules.as_ref().unwrap()[0].skipped);
        assert!(snapshot.items[1].applied_rules.as_ref().unwrap()[0].skipped);

        // Totals recalculated: 100 + 50*2 = 200
        assert_eq!(snapshot.subtotal, 200.0);
        assert_eq!(snapshot.total, 200.0);
    }

    #[test]
    fn test_rule_skip_toggled_surcharge_rule() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        // Item: base 100, surcharge +15 → price 115
        snapshot.items.push(create_test_item_with_rule(
            "inst-1", 115.0, 100.0, 1,
            "rule-1", shared::models::price_rule::RuleType::Surcharge, 15.0, 15.0,
        ));
        snapshot.subtotal = 115.0;
        snapshot.total = 115.0;

        // Skip the surcharge
        let event = create_rule_skip_toggled_event("order-1", 2, "rule-1", true);
        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        assert!(snapshot.items[0].applied_rules.as_ref().unwrap()[0].skipped);
        // Surcharge skipped → back to base price
        assert_eq!(snapshot.subtotal, 100.0);
        assert_eq!(snapshot.total, 100.0);
    }

    #[test]
    fn test_rule_skip_toggled_skip_unskip_cycle() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        snapshot.items.push(create_test_item_with_rule(
            "inst-1", 90.0, 100.0, 1,
            "rule-1", shared::models::price_rule::RuleType::Discount, 10.0, 10.0,
        ));
        snapshot.subtotal = 90.0;
        snapshot.total = 90.0;
        let applier = RuleSkipToggledApplier;

        // Skip
        let event = create_rule_skip_toggled_event("order-1", 2, "rule-1", true);
        applier.apply(&mut snapshot, &event);
        assert_eq!(snapshot.subtotal, 100.0);
        assert_eq!(snapshot.total, 100.0);
        let checksum_after_skip = snapshot.state_checksum.clone();

        // Unskip
        let event = create_rule_skip_toggled_event("order-1", 3, "rule-1", false);
        applier.apply(&mut snapshot, &event);
        assert_eq!(snapshot.subtotal, 90.0);
        assert_eq!(snapshot.total, 90.0);
        assert_eq!(snapshot.last_sequence, 3);

        // Skip again → should produce same totals as first skip
        let event = create_rule_skip_toggled_event("order-1", 4, "rule-1", true);
        applier.apply(&mut snapshot, &event);
        assert_eq!(snapshot.subtotal, 100.0);
        assert_eq!(snapshot.total, 100.0);
        // Note: checksum won't match exactly due to different sequence/timestamp,
        // but verify_checksum should still pass
        assert!(snapshot.verify_checksum());
        assert_ne!(snapshot.state_checksum, checksum_after_skip, "Different sequence → different checksum");
    }

    #[test]
    fn test_rule_skip_toggled_comped_item_stays_zero() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        // Comped item with a rule
        let mut item = create_test_item_with_rule(
            "inst-1", 0.0, 100.0, 1,
            "rule-1", shared::models::price_rule::RuleType::Discount, 10.0, 10.0,
        );
        item.is_comped = true;
        item.price = 0.0;
        snapshot.items.push(item);
        snapshot.subtotal = 0.0;
        snapshot.total = 0.0;

        // Skip the rule on comped item
        let event = create_rule_skip_toggled_event("order-1", 2, "rule-1", true);
        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        // Rule is marked skipped
        assert!(snapshot.items[0].applied_rules.as_ref().unwrap()[0].skipped);
        // But comped item still zero
        assert_eq!(snapshot.subtotal, 0.0);
        assert_eq!(snapshot.total, 0.0);
    }

    #[test]
    fn test_rule_skip_toggled_with_manual_discount() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        // Item: base 100, manual 20% off, rule discount 10 → unit_price = 100-20-10 = 70
        let mut item = create_test_item_with_rule(
            "inst-1", 70.0, 100.0, 1,
            "rule-1", shared::models::price_rule::RuleType::Discount, 10.0, 10.0,
        );
        item.manual_discount_percent = Some(20.0);
        snapshot.items.push(item);
        snapshot.subtotal = 70.0;
        snapshot.total = 70.0;

        // Skip the rule → only manual discount remains
        let event = create_rule_skip_toggled_event("order-1", 2, "rule-1", true);
        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        // unit_price = 100 - 20% = 80 (rule discount removed, manual stays)
        assert_eq!(snapshot.subtotal, 80.0);
        assert_eq!(snapshot.total, 80.0);
    }

    #[test]
    fn test_rule_skip_toggled_with_options_modifier() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        // Item: base 50, option +5, rule discount 6 → unit_price = 55-6 = 49
        let mut item = create_test_item_with_rule(
            "inst-1", 49.0, 50.0, 1,
            "rule-1", shared::models::price_rule::RuleType::Discount, 10.0, 6.0,
        );
        item.selected_options = Some(vec![shared::order::ItemOption {
            attribute_id: "attr:size".to_string(),
            attribute_name: "Size".to_string(),
            option_idx: 1,
            option_name: "Large".to_string(),
            price_modifier: Some(5.0),
        }]);
        snapshot.items.push(item);
        snapshot.subtotal = 49.0;
        snapshot.total = 49.0;

        // Skip the rule
        let event = create_rule_skip_toggled_event("order-1", 2, "rule-1", true);
        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        // unit_price = 50 + 5 = 55 (option still applies, rule discount removed)
        assert_eq!(snapshot.subtotal, 55.0);
        assert_eq!(snapshot.total, 55.0);
    }

    #[test]
    fn test_rule_skip_toggled_item_with_tax_rate() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        let mut item = create_test_item_with_rule(
            "inst-1", 90.0, 100.0, 1,
            "rule-1", shared::models::price_rule::RuleType::Discount, 10.0, 10.0,
        );
        item.tax_rate = Some(21); // 21% IVA
        snapshot.items.push(item);

        // Apply with discount active
        let applier = RuleSkipToggledApplier;
        // First recalculate to set baseline
        money::recalculate_totals(&mut snapshot);
        let tax_with_discount = snapshot.items[0].tax.unwrap();
        // Tax on 90: 90 * 21/121 ≈ 15.62
        assert_eq!(tax_with_discount, 15.62);

        // Skip rule → price goes to 100
        let event = create_rule_skip_toggled_event("order-1", 2, "rule-1", true);
        applier.apply(&mut snapshot, &event);

        // Tax on 100: 100 * 21/121 ≈ 17.36
        assert_eq!(snapshot.items[0].tax, Some(17.36));
        assert!(snapshot.items[0].tax.unwrap() > tax_with_discount,
            "Tax increases when discount is removed");
    }

    #[test]
    fn test_rule_skip_toggled_order_level_discount_with_item_level_rule() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        // Item with item-level discount
        snapshot.items.push(create_test_item_with_rule(
            "inst-1", 90.0, 100.0, 1,
            "item-rule-1", shared::models::price_rule::RuleType::Discount, 10.0, 10.0,
        ));

        // Order-level discount (separate rule)
        snapshot.order_applied_rules = Some(vec![AppliedRule {
            rule_id: "order-rule-1".to_string(),
            name: "order_discount".to_string(),
            display_name: "Order Discount".to_string(),
            receipt_name: "ORD".to_string(),
            rule_type: shared::models::price_rule::RuleType::Discount,
            adjustment_type: shared::models::price_rule::AdjustmentType::Percentage,
            product_scope: shared::models::price_rule::ProductScope::Global,
            zone_scope: "zone:all".to_string(),
            adjustment_value: 10.0,
            calculated_amount: 9.0, // 10% of 90 = 9
            is_stackable: true,
            is_exclusive: false,
            skipped: false,
        }]);
        snapshot.order_rule_discount_amount = Some(9.0);

        money::recalculate_totals(&mut snapshot);
        // subtotal = 90, order_discount = 9, total = 81
        assert_eq!(snapshot.subtotal, 90.0);
        assert_eq!(snapshot.total, 81.0);

        // Skip the item-level rule → subtotal goes up, order discount recalculated
        let event = create_rule_skip_toggled_event("order-1", 2, "item-rule-1", true);
        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        // subtotal = 100 (item rule skipped), order discount still 9.0 (calculated_amount fixed)
        // total = 100 - 9 = 91
        assert_eq!(snapshot.subtotal, 100.0);
        assert_eq!(snapshot.total, 91.0);
        // Order rule is NOT skipped
        assert!(!snapshot.order_applied_rules.as_ref().unwrap()[0].skipped);
    }

    #[test]
    fn test_rule_skip_toggled_already_in_desired_state() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        // Item with already-skipped rule
        let mut item = create_test_item_with_rule(
            "inst-1", 100.0, 100.0, 1,
            "rule-1", shared::models::price_rule::RuleType::Discount, 10.0, 10.0,
        );
        item.applied_rules.as_mut().unwrap()[0].skipped = true;
        snapshot.items.push(item);
        snapshot.subtotal = 100.0;
        snapshot.total = 100.0;

        // Skip again (no-op in terms of skipped state)
        let event = create_rule_skip_toggled_event("order-1", 2, "rule-1", true);
        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        // Should remain consistent
        assert!(snapshot.items[0].applied_rules.as_ref().unwrap()[0].skipped);
        assert_eq!(snapshot.subtotal, 100.0);
        assert_eq!(snapshot.total, 100.0);
        assert_eq!(snapshot.last_sequence, 2);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_rule_skip_toggled_multiple_rules_on_item_skip_one() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;

        // Item with two rules
        snapshot.items.push(CartItemSnapshot {
            id: "product:1".to_string(),
            instance_id: "inst-1".to_string(),
            name: "Test Product".to_string(),
            price: 82.0,
            original_price: Some(100.0),
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: Some(18.0),
            rule_surcharge_amount: None,
            applied_rules: Some(vec![
                AppliedRule {
                    rule_id: "rule-1".to_string(),
                    name: "lunch".to_string(),
                    display_name: "Lunch Discount".to_string(),
                    receipt_name: "LUNCH".to_string(),
                    rule_type: shared::models::price_rule::RuleType::Discount,
                    adjustment_type: shared::models::price_rule::AdjustmentType::Percentage,
                    product_scope: shared::models::price_rule::ProductScope::Global,
                    zone_scope: "zone:all".to_string(),
                    adjustment_value: 10.0,
                    calculated_amount: 10.0,
                    is_stackable: true,
                    is_exclusive: false,
                    skipped: false,
                },
                AppliedRule {
                    rule_id: "rule-2".to_string(),
                    name: "loyalty".to_string(),
                    display_name: "Loyalty Discount".to_string(),
                    receipt_name: "LOY".to_string(),
                    rule_type: shared::models::price_rule::RuleType::Discount,
                    adjustment_type: shared::models::price_rule::AdjustmentType::Percentage,
                    product_scope: shared::models::price_rule::ProductScope::Global,
                    zone_scope: "zone:all".to_string(),
                    adjustment_value: 8.0,
                    calculated_amount: 8.0,
                    is_stackable: true,
                    is_exclusive: false,
                    skipped: false,
                },
            ]),
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
        });

        money::recalculate_totals(&mut snapshot);
        // Both active: 100 - 10 - 8 = 82
        assert_eq!(snapshot.subtotal, 82.0);

        // Skip only rule-1
        let event = create_rule_skip_toggled_event("order-1", 2, "rule-1", true);
        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        // rule-1 skipped, rule-2 still active: 100 - 8 = 92
        assert!(snapshot.items[0].applied_rules.as_ref().unwrap()[0].skipped);
        assert!(!snapshot.items[0].applied_rules.as_ref().unwrap()[1].skipped);
        assert_eq!(snapshot.subtotal, 92.0);
        assert_eq!(snapshot.total, 92.0);
    }
}
