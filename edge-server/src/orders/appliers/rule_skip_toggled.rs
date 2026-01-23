//! RuleSkipToggled event applier
//!
//! Applies the RuleSkipToggled event to toggle a rule's skipped status
//! and update order totals.

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

/// RuleSkipToggled applier
pub struct RuleSkipToggledApplier;

impl EventApplier for RuleSkipToggledApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::RuleSkipToggled {
            rule_id,
            skipped,
            subtotal,
            discount,
            surcharge,
            total,
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

            // 2. Update order-level totals from the recalculated values in the event
            snapshot.subtotal = *subtotal;
            snapshot.discount = *discount;
            // Note: OrderSnapshot uses order_rule_surcharge_amount for rule-based surcharges
            snapshot.order_rule_surcharge_amount = if *surcharge > 0.0 {
                Some(*surcharge)
            } else {
                None
            };
            snapshot.total = *total;

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
    use shared::order::{AppliedRule, CartItemSnapshot, OrderEventType, OrderStatus};

    fn create_test_snapshot_with_rules(order_id: &str) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;

        // Add an item with applied rules
        let item = CartItemSnapshot {
            id: "product-1".to_string(),
            instance_id: "inst-1".to_string(),
            name: "Test Product".to_string(),
            price: 90.0, // After 10% discount
            original_price: Some(100.0),
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: Some(10.0),
            rule_surcharge_amount: None,
            applied_rules: Some(vec![AppliedRule {
                rule_id: "rule-1".to_string(),
                name: "happy_hour".to_string(),
                display_name: "Happy Hour".to_string(),
                receipt_name: "HH".to_string(),
                rule_type: shared::models::price_rule::RuleType::Discount,
                adjustment_type: shared::models::price_rule::AdjustmentType::Percentage,
                product_scope: shared::models::price_rule::ProductScope::Global,
                zone_scope: "zone:all".to_string(),
                adjustment_value: 10.0,
                calculated_amount: 10.0,
                priority: 0,
                is_stackable: true,
                is_exclusive: false,
                skipped: false,
            }]),
            surcharge: None,
            line_total: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        snapshot.items.push(item);
        snapshot.subtotal = 90.0;
        snapshot.discount = 10.0;
        snapshot.total = 90.0;
        snapshot
    }

    fn create_rule_skip_toggled_event(
        order_id: &str,
        seq: u64,
        rule_id: &str,
        skipped: bool,
        subtotal: f64,
        discount: f64,
        surcharge: f64,
        total: f64,
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
                subtotal,
                discount,
                surcharge,
                total,
            },
        )
    }

    #[test]
    fn test_rule_skip_toggled_skip_rule() {
        let mut snapshot = create_test_snapshot_with_rules("order-1");

        // Verify initial state
        assert!(!snapshot.items[0]
            .applied_rules
            .as_ref()
            .unwrap()[0]
            .skipped);
        assert_eq!(snapshot.subtotal, 90.0);
        assert_eq!(snapshot.discount, 10.0);

        // Skip the rule - price goes back to original
        let event = create_rule_skip_toggled_event(
            "order-1",
            2,
            "rule-1",
            true,
            100.0, // subtotal now 100 (no discount)
            0.0,   // discount now 0
            0.0,
            100.0, // total now 100
        );

        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        // Rule should be marked as skipped
        assert!(snapshot.items[0]
            .applied_rules
            .as_ref()
            .unwrap()[0]
            .skipped);

        // Totals should be updated
        assert_eq!(snapshot.subtotal, 100.0);
        assert_eq!(snapshot.discount, 0.0);
        assert_eq!(snapshot.total, 100.0);
    }

    #[test]
    fn test_rule_skip_toggled_unskip_rule() {
        let mut snapshot = create_test_snapshot_with_rules("order-1");

        // First skip the rule
        snapshot.items[0]
            .applied_rules
            .as_mut()
            .unwrap()[0]
            .skipped = true;
        snapshot.subtotal = 100.0;
        snapshot.discount = 0.0;
        snapshot.total = 100.0;

        // Unskip the rule - discount applies again
        let event = create_rule_skip_toggled_event(
            "order-1",
            2,
            "rule-1",
            false,
            90.0,  // subtotal with discount
            10.0,  // discount restored
            0.0,
            90.0,  // total with discount
        );

        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        // Rule should be marked as not skipped
        assert!(!snapshot.items[0]
            .applied_rules
            .as_ref()
            .unwrap()[0]
            .skipped);

        // Totals should be updated
        assert_eq!(snapshot.subtotal, 90.0);
        assert_eq!(snapshot.discount, 10.0);
        assert_eq!(snapshot.total, 90.0);
    }

    #[test]
    fn test_rule_skip_toggled_nonexistent_rule() {
        let mut snapshot = create_test_snapshot_with_rules("order-1");
        let original_skipped = snapshot.items[0]
            .applied_rules
            .as_ref()
            .unwrap()[0]
            .skipped;

        // Try to skip a rule that doesn't exist
        let event = create_rule_skip_toggled_event(
            "order-1",
            2,
            "nonexistent-rule",
            true,
            90.0,  // Values unchanged
            10.0,
            0.0,
            90.0,
        );

        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        // Original rule should be unchanged
        assert_eq!(
            snapshot.items[0]
                .applied_rules
                .as_ref()
                .unwrap()[0]
                .skipped,
            original_skipped
        );

        // But totals are still updated from event
        assert_eq!(snapshot.subtotal, 90.0);
    }

    #[test]
    fn test_rule_skip_toggled_updates_sequence() {
        let mut snapshot = create_test_snapshot_with_rules("order-1");
        snapshot.last_sequence = 5;

        let event = create_rule_skip_toggled_event(
            "order-1",
            10,
            "rule-1",
            true,
            100.0,
            0.0,
            0.0,
            100.0,
        );

        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 10);
    }

    #[test]
    fn test_rule_skip_toggled_updates_checksum() {
        let mut snapshot = create_test_snapshot_with_rules("order-1");
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_rule_skip_toggled_event(
            "order-1",
            2,
            "rule-1",
            true,
            100.0,
            0.0,
            0.0,
            100.0,
        );

        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_rule_skip_toggled_with_surcharge() {
        let mut snapshot = create_test_snapshot_with_rules("order-1");

        // Skip discount rule and add surcharge
        let event = create_rule_skip_toggled_event(
            "order-1",
            2,
            "rule-1",
            true,
            100.0, // subtotal
            0.0,   // no discount
            5.0,   // surcharge added
            105.0, // total with surcharge
        );

        let applier = RuleSkipToggledApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.subtotal, 100.0);
        assert_eq!(snapshot.discount, 0.0);
        assert_eq!(snapshot.order_rule_surcharge_amount, Some(5.0));
        assert_eq!(snapshot.total, 105.0);
    }
}
