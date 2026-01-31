//! ItemsAdded event applier
//!
//! Applies the ItemsAdded event to add items to the snapshot.

use crate::orders::money;
use crate::orders::traits::EventApplier;
use shared::order::{CartItemSnapshot, EventPayload, OrderEvent, OrderSnapshot};

/// ItemsAdded applier
pub struct ItemsAddedApplier;

impl EventApplier for ItemsAddedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::ItemsAdded { items } = &event.payload {
            // Add items to snapshot (merge if same instance_id exists)
            for item in items {
                add_or_merge_item(snapshot, item);
            }

            // Update sequence and timestamp
            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;

            // Recalculate totals using precise decimal arithmetic
            money::recalculate_totals(snapshot);

            // Update checksum
            snapshot.update_checksum();
        }
    }
}

/// Add item to snapshot, merging with existing item if instance_id matches
pub(crate) fn add_or_merge_item(snapshot: &mut OrderSnapshot, item: &CartItemSnapshot) {
    if let Some(existing) = snapshot
        .items
        .iter_mut()
        .find(|i| i.instance_id == item.instance_id)
    {
        // Merge by adding quantity
        existing.quantity += item.quantity;
        existing.unpaid_quantity += item.quantity;
    } else {
        // Add new item
        snapshot.items.push(item.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::OrderEventType;

    fn create_test_item(
        instance_id: &str,
        name: &str,
        price: f64,
        quantity: i32,
    ) -> CartItemSnapshot {
        CartItemSnapshot {
            id: "product:1".to_string(),
            instance_id: instance_id.to_string(),
            name: name.to_string(),
            price,
            original_price: None,
            quantity,
            unpaid_quantity: quantity,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            surcharge: None,
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
        }
    }

    fn create_items_added_event(
        order_id: &str,
        seq: u64,
        items: Vec<CartItemSnapshot>,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::ItemsAdded,
            EventPayload::ItemsAdded { items },
        )
    }

    #[test]
    fn test_items_added_applier_single_item() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.last_sequence = 0;

        let items = vec![create_test_item("item-1", "Product A", 10.0, 2)];
        let event = create_items_added_event("order-1", 1, items);

        let applier = ItemsAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "item-1");
        assert_eq!(snapshot.items[0].name, "Product A");
        assert_eq!(snapshot.items[0].quantity, 2);
        assert_eq!(snapshot.items[0].price, 10.0);
        assert_eq!(snapshot.subtotal, 20.0);
        assert_eq!(snapshot.total, 20.0);
        assert_eq!(snapshot.last_sequence, 1);
    }

    #[test]
    fn test_items_added_applier_multiple_items() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());

        let items = vec![
            create_test_item("item-1", "Product A", 10.0, 2),
            create_test_item("item-2", "Product B", 15.0, 1),
        ];
        let event = create_items_added_event("order-1", 1, items);

        let applier = ItemsAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items.len(), 2);
        // 10.0 * 2 + 15.0 * 1 = 35.0
        assert_eq!(snapshot.subtotal, 35.0);
        assert_eq!(snapshot.total, 35.0);
    }

    #[test]
    fn test_items_added_applier_merges_same_instance_id() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());

        // Add initial item
        snapshot
            .items
            .push(create_test_item("item-1", "Product A", 10.0, 2));
        snapshot.subtotal = 20.0;
        snapshot.total = 20.0;

        // Add same item again (same instance_id)
        let items = vec![create_test_item("item-1", "Product A", 10.0, 3)];
        let event = create_items_added_event("order-1", 2, items);

        let applier = ItemsAddedApplier;
        applier.apply(&mut snapshot, &event);

        // Should merge, not add new item
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].quantity, 5); // 2 + 3
        assert_eq!(snapshot.items[0].unpaid_quantity, 5);
        // 10.0 * 5 = 50.0
        assert_eq!(snapshot.subtotal, 50.0);
        assert_eq!(snapshot.total, 50.0);
    }

    #[test]
    fn test_items_added_applier_adds_different_items() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());

        // Add initial item
        snapshot
            .items
            .push(create_test_item("item-1", "Product A", 10.0, 2));

        // Add different item
        let items = vec![create_test_item("item-2", "Product B", 15.0, 1)];
        let event = create_items_added_event("order-1", 2, items);

        let applier = ItemsAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items.len(), 2);
        assert_eq!(snapshot.items[0].instance_id, "item-1");
        assert_eq!(snapshot.items[1].instance_id, "item-2");
    }

    #[test]
    fn test_items_added_applier_with_discount() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());

        let mut item = create_test_item("item-1", "Product A", 100.0, 1);
        item.manual_discount_percent = Some(20.0); // 20% discount

        let items = vec![item];
        let event = create_items_added_event("order-1", 1, items);

        let applier = ItemsAddedApplier;
        applier.apply(&mut snapshot, &event);

        // 100.0 * 1 * (1 - 0.20) = 80.0
        assert_eq!(snapshot.subtotal, 80.0);
        assert_eq!(snapshot.total, 80.0);
    }

    #[test]
    fn test_items_added_applier_updates_sequence() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.last_sequence = 5;

        let items = vec![create_test_item("item-1", "Product A", 10.0, 1)];
        let event = create_items_added_event("order-1", 6, items);

        let applier = ItemsAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 6);
    }

    #[test]
    fn test_items_added_applier_updates_checksum() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let initial_checksum = snapshot.state_checksum.clone();

        let items = vec![create_test_item("item-1", "Product A", 10.0, 1)];
        let event = create_items_added_event("order-1", 1, items);

        let applier = ItemsAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_items_added_with_empty_items() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.last_sequence = 0;

        let event = create_items_added_event("order-1", 1, vec![]);

        let applier = ItemsAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items.len(), 0);
        assert_eq!(snapshot.subtotal, 0.0);
        assert_eq!(snapshot.total, 0.0);
        assert_eq!(snapshot.last_sequence, 1);
    }

    /// Test that replay uses Event data, not external state
    ///
    /// Scenario:
    /// 1. Item was added with price rule applied (10% discount)
    /// 2. Event stores the computed price and applied_rules
    /// 3. Later, the product's tag/category changes (would no longer match the rule)
    /// 4. On replay, the Event data should be used as-is
    ///
    /// This verifies that:
    /// - Checksum remains stable after replay
    /// - applied_rules from Event are preserved
    /// - Price from Event is used (not recalculated)
    #[test]
    fn test_replay_uses_event_data_not_external_state() {
        use shared::models::price_rule::{AdjustmentType, ProductScope, RuleType};
        use shared::order::AppliedRule;

        // Create item with price rule already applied (as if it matched "Lunch Special" rule)
        let mut item = create_test_item("item-1", "Lunch Set", 100.0, 1);
        item.original_price = Some(100.0);
        item.price = 90.0; // 10% discount already applied
        item.rule_discount_amount = Some(10.0);
        item.applied_rules = Some(vec![AppliedRule {
            rule_id: "rule:lunch-special".to_string(),
            name: "lunch-special".to_string(),
            display_name: "Lunch Special 10% Off".to_string(),
            receipt_name: "Lunch 10%".to_string(),
            rule_type: RuleType::Discount,
            adjustment_type: AdjustmentType::Percentage,
            product_scope: ProductScope::Global,
            zone_scope: "zone:all".to_string(),
            adjustment_value: 10.0,
            calculated_amount: 10.0,
            priority: 0,
            is_stackable: true,
            is_exclusive: false,
            skipped: false,
        }]);

        // Create Event with this item
        let event = create_items_added_event("order-1", 1, vec![item]);

        // Apply to empty snapshot (simulating replay after restart)
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let applier = ItemsAddedApplier;
        applier.apply(&mut snapshot, &event);

        // Record checksum after first replay
        let checksum_after_first_replay = snapshot.state_checksum.clone();

        // Verify the item data is preserved from Event
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].price, 90.0); // Discounted price from Event
        assert_eq!(snapshot.items[0].original_price, Some(100.0));
        assert_eq!(snapshot.items[0].rule_discount_amount, Some(10.0));
        assert!(snapshot.items[0].applied_rules.is_some());
        assert_eq!(snapshot.items[0].applied_rules.as_ref().unwrap().len(), 1);
        assert_eq!(
            snapshot.items[0].applied_rules.as_ref().unwrap()[0].display_name,
            "Lunch Special 10% Off"
        );

        // Subtotal should be based on the price from Event (90.0), not recalculated
        assert_eq!(snapshot.subtotal, 90.0);
        assert_eq!(snapshot.total, 90.0);

        // Simulate second replay (e.g., another restart)
        // Even if external product metadata changed, result should be identical
        let mut snapshot2 = OrderSnapshot::new("order-1".to_string());
        applier.apply(&mut snapshot2, &event);

        // Checksum should be identical
        assert_eq!(snapshot2.state_checksum, checksum_after_first_replay);
        assert_eq!(snapshot2.subtotal, 90.0);
        assert_eq!(snapshot2.total, 90.0);
    }

    /// Test that multiple replays produce identical checksum
    #[test]
    fn test_replay_determinism() {
        let items = vec![
            create_test_item("item-1", "Product A", 10.50, 2),
            create_test_item("item-2", "Product B", 25.99, 1),
        ];
        let event = create_items_added_event("order-1", 1, items);

        // Replay 10 times, all should produce same checksum
        let applier = ItemsAddedApplier;
        let mut checksums = Vec::new();

        for _ in 0..10 {
            let mut snapshot = OrderSnapshot::new("order-1".to_string());
            applier.apply(&mut snapshot, &event);
            checksums.push(snapshot.state_checksum);
        }

        // All checksums should be identical
        let first = &checksums[0];
        for checksum in &checksums {
            assert_eq!(checksum, first, "Replay should be deterministic");
        }
    }
}
