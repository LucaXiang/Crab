//! StampRedeemed event applier
//!
//! Adds the reward item as a new comped line in the order and records
//! the redemption in snapshot.stamp_redemptions for reversal on member unlink.

use crate::orders::money;
use crate::orders::traits::EventApplier;
use shared::order::{CartItemSnapshot, EventPayload, OrderEvent, OrderSnapshot, StampRedemptionState};

/// StampRedeemed applier
pub struct StampRedeemedApplier;

impl EventApplier for StampRedeemedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::StampRedeemed {
            stamp_activity_id,
            reward_instance_id,
            product_id,
            product_name,
            original_price,
            quantity,
            tax_rate,
            category_id,
            category_name,
            comp_existing_instance_id,
            ..
        } = &event.payload
        {
            let is_comp_existing = comp_existing_instance_id.is_some();
            // Track source instance_id for partial comp reversal
            let mut comp_source_instance_id = None;

            if let Some(existing_id) = comp_existing_instance_id {
                // Comp-existing mode: full comp or partial comp (split)
                let is_full_comp = reward_instance_id == existing_id;

                if is_full_comp {
                    // Full comp: item.quantity <= reward_quantity, comp the whole item
                    if let Some(item) = snapshot.items.iter_mut().find(|i| i.instance_id == *existing_id) {
                        item.is_comped = true;
                    }
                } else {
                    // Partial comp: item.quantity > reward_quantity, split the item
                    if let Some(source_idx) = snapshot.items.iter().position(|i| i.instance_id == *existing_id) {
                        let source = snapshot.items[source_idx].clone();

                        // Reduce source quantity
                        snapshot.items[source_idx].quantity -= quantity;
                        snapshot.items[source_idx].unpaid_quantity =
                            (snapshot.items[source_idx].unpaid_quantity - quantity).max(0);

                        // Create new comped item as a split
                        let mut comped_item = source;
                        comped_item.instance_id = reward_instance_id.clone();
                        comped_item.quantity = *quantity;
                        comped_item.unpaid_quantity = *quantity;
                        comped_item.is_comped = true;
                        comped_item.price = 0.0;

                        snapshot.items.push(comped_item);
                        comp_source_instance_id = Some(existing_id.clone());
                    }
                }
            } else {
                // Add-new mode: add reward item as a new comped line
                let reward_item = CartItemSnapshot {
                    id: *product_id,
                    instance_id: reward_instance_id.clone(),
                    name: product_name.clone(),
                    price: 0.0,
                    original_price: *original_price,
                    quantity: *quantity,
                    unpaid_quantity: 0,
                    selected_options: None,
                    selected_specification: None,
                    manual_discount_percent: None,
                    rule_discount_amount: 0.0,
                    rule_surcharge_amount: 0.0,
                    applied_rules: vec![],
                    applied_mg_rules: vec![],
                    mg_discount_amount: 0.0,
                    unit_price: 0.0,
                    line_total: 0.0,
                    tax: 0.0,
                    tax_rate: *tax_rate,
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                    category_id: *category_id,
                    category_name: category_name.clone(),
                    is_comped: true,
                };
                snapshot.items.push(reward_item);
            }

            // Record redemption for reversal
            snapshot.stamp_redemptions.push(StampRedemptionState {
                stamp_activity_id: *stamp_activity_id,
                reward_instance_id: reward_instance_id.clone(),
                is_comp_existing,
                comp_source_instance_id,
            });

            // Update sequence and timestamp
            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;

            // Recalculate totals
            money::recalculate_totals(snapshot);

            // Update checksum
            snapshot.update_checksum();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::{OrderEventType, OrderSnapshot};

    fn create_stamp_redeemed_event(order_id: &str, seq: u64) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            1,
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::StampRedeemed,
            EventPayload::StampRedeemed {
                stamp_activity_id: 1,
                stamp_activity_name: "Coffee Card".to_string(),
                reward_instance_id: "stamp_reward::cmd-1".to_string(),
                reward_strategy: "DESIGNATED".to_string(),
                product_id: 100,
                product_name: "Coffee".to_string(),
                original_price: 3.50,
                quantity: 1,
                tax_rate: 10,
                category_id: None,
                category_name: Some("Drinks".to_string()),
                comp_existing_instance_id: None,
            },
        )
    }

    #[test]
    fn test_stamp_redeemed_adds_comped_item() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());

        let event = create_stamp_redeemed_event("order-1", 1);
        let applier = StampRedeemedApplier;
        applier.apply(&mut snapshot, &event);

        // Item added
        assert_eq!(snapshot.items.len(), 1);
        let item = &snapshot.items[0];
        assert_eq!(item.id, 100);
        assert_eq!(item.instance_id, "stamp_reward::cmd-1");
        assert_eq!(item.name, "Coffee");
        assert!(item.is_comped);
        assert!((item.price - 0.0).abs() < f64::EPSILON);
        assert!((item.original_price - 3.50).abs() < f64::EPSILON);
        assert_eq!(item.quantity, 1);
        assert_eq!(item.tax_rate, 10);
        assert_eq!(item.category_name.as_deref(), Some("Drinks"));
    }

    #[test]
    fn test_stamp_redeemed_records_redemption_state() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());

        let event = create_stamp_redeemed_event("order-1", 1);
        let applier = StampRedeemedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.stamp_redemptions.len(), 1);
        assert_eq!(snapshot.stamp_redemptions[0].stamp_activity_id, 1);
        assert_eq!(
            snapshot.stamp_redemptions[0].reward_instance_id,
            "stamp_reward::cmd-1"
        );
    }

    #[test]
    fn test_stamp_redeemed_updates_totals() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        // Add a paid item first
        snapshot.items.push(CartItemSnapshot {
            id: 200,
            instance_id: "inst-1".to_string(),
            name: "Cake".to_string(),
            price: 5.00,
            original_price: 5.00,
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
            unit_price: 5.00,
            line_total: 5.00,
            tax: 0.0,
            tax_rate: 0,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_id: None,
            category_name: None,
            is_comped: false,
        });
        money::recalculate_totals(&mut snapshot);
        assert!((snapshot.total - 5.00).abs() < f64::EPSILON);

        // Redeem stamp — adds free Coffee
        let event = create_stamp_redeemed_event("order-1", 1);
        let applier = StampRedeemedApplier;
        applier.apply(&mut snapshot, &event);

        // Total should still be 5.00 (free item doesn't add cost)
        assert_eq!(snapshot.items.len(), 2);
        assert!((snapshot.total - 5.00).abs() < f64::EPSILON);
        assert!(snapshot.verify_checksum());
    }

    fn create_test_item(instance_id: &str, product_id: i64, price: f64, quantity: i32) -> CartItemSnapshot {
        CartItemSnapshot {
            id: product_id,
            instance_id: instance_id.to_string(),
            name: format!("Product {}", product_id),
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
            applied_mg_rules: vec![],
            mg_discount_amount: 0.0,
            unit_price: price,
            line_total: price * quantity as f64,
            tax: 0.0,
            tax_rate: 10,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_id: Some(1),
            category_name: Some("Food".to_string()),
            is_comped: false,
        }
    }

    fn create_comp_existing_event(
        order_id: &str,
        seq: u64,
        reward_instance_id: &str,
        comp_existing_instance_id: &str,
        quantity: i32,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            1,
            "Test User".to_string(),
            "cmd-2".to_string(),
            Some(1234567890),
            OrderEventType::StampRedeemed,
            EventPayload::StampRedeemed {
                stamp_activity_id: 1,
                stamp_activity_name: "Coffee Card".to_string(),
                reward_instance_id: reward_instance_id.to_string(),
                reward_strategy: "DESIGNATED".to_string(),
                product_id: 50,
                product_name: "Patatas Bravas".to_string(),
                original_price: 4.50,
                quantity,
                tax_rate: 10,
                category_id: Some(1),
                category_name: Some("Food".to_string()),
                comp_existing_instance_id: Some(comp_existing_instance_id.to_string()),
            },
        )
    }

    // =========================================================================
    // Comp-existing: FULL COMP (item.qty <= reward_qty)
    // =========================================================================

    #[test]
    fn test_comp_existing_full_comp_single_item() {
        // Item qty=1, reward_qty=1 → full comp (no split)
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(create_test_item("item-1", 50, 4.50, 1));
        money::recalculate_totals(&mut snapshot);

        // Full comp: reward_instance_id == comp_existing_instance_id
        let event = create_comp_existing_event("order-1", 1, "item-1", "item-1", 1);
        let applier = StampRedeemedApplier;
        applier.apply(&mut snapshot, &event);

        // Still 1 item, but comped
        assert_eq!(snapshot.items.len(), 1);
        assert!(snapshot.items[0].is_comped);
        assert_eq!(snapshot.items[0].instance_id, "item-1");
        assert_eq!(snapshot.items[0].quantity, 1);

        // Total should be 0 (item is comped)
        assert!((snapshot.total - 0.0).abs() < f64::EPSILON);

        // Redemption state: no source (full comp)
        assert_eq!(snapshot.stamp_redemptions.len(), 1);
        assert!(snapshot.stamp_redemptions[0].is_comp_existing);
        assert!(snapshot.stamp_redemptions[0].comp_source_instance_id.is_none());
    }

    #[test]
    fn test_comp_existing_full_comp_reward_exceeds_qty() {
        // Item qty=2, reward_qty=3 → full comp (cap to item qty, no split)
        // Event quantity is already capped by action to min(3, 2) = 2
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(create_test_item("item-1", 50, 4.50, 2));
        money::recalculate_totals(&mut snapshot);

        // Full comp: reward_instance_id == comp_existing_instance_id
        let event = create_comp_existing_event("order-1", 1, "item-1", "item-1", 2);
        let applier = StampRedeemedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items.len(), 1);
        assert!(snapshot.items[0].is_comped);
        assert_eq!(snapshot.items[0].quantity, 2);
        assert!((snapshot.total - 0.0).abs() < f64::EPSILON);
    }

    // =========================================================================
    // Comp-existing: PARTIAL COMP (item.qty > reward_qty)
    // =========================================================================

    #[test]
    fn test_comp_existing_partial_comp_splits_item() {
        // Item qty=7, reward_qty=1 → partial comp (split 1 off)
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(create_test_item("item-1", 50, 4.50, 7));
        money::recalculate_totals(&mut snapshot);
        let initial_total = snapshot.total;

        // Partial comp: reward_instance_id != comp_existing_instance_id
        let event = create_comp_existing_event("order-1", 1, "stamp_reward::cmd-2", "item-1", 1);
        let applier = StampRedeemedApplier;
        applier.apply(&mut snapshot, &event);

        // Should have 2 items now
        assert_eq!(snapshot.items.len(), 2);

        // Original item: qty reduced from 7 to 6
        let source = snapshot.items.iter().find(|i| i.instance_id == "item-1").unwrap();
        assert_eq!(source.quantity, 6);
        assert!(!source.is_comped);

        // New comped item: qty = 1
        let comped = snapshot.items.iter().find(|i| i.instance_id == "stamp_reward::cmd-2").unwrap();
        assert_eq!(comped.quantity, 1);
        assert!(comped.is_comped);
        assert_eq!(comped.id, 50);
        assert_eq!(comped.name, "Product 50");

        // Total should decrease by 1 unit price (4.50)
        let expected_total = initial_total - 4.50;
        assert!((snapshot.total - expected_total).abs() < 0.01);

        // Redemption state: has source for partial comp
        assert_eq!(snapshot.stamp_redemptions.len(), 1);
        assert!(snapshot.stamp_redemptions[0].is_comp_existing);
        assert_eq!(
            snapshot.stamp_redemptions[0].comp_source_instance_id.as_deref(),
            Some("item-1")
        );
        assert_eq!(snapshot.stamp_redemptions[0].reward_instance_id, "stamp_reward::cmd-2");
    }

    #[test]
    fn test_comp_existing_partial_comp_large_split() {
        // Item qty=10, reward_qty=3 → split 3 off, leave 7
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(create_test_item("item-1", 50, 2.00, 10));
        money::recalculate_totals(&mut snapshot);

        let event = create_comp_existing_event("order-1", 1, "stamp_reward::cmd-2", "item-1", 3);
        let applier = StampRedeemedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items.len(), 2);

        let source = snapshot.items.iter().find(|i| i.instance_id == "item-1").unwrap();
        assert_eq!(source.quantity, 7);
        assert!(!source.is_comped);

        let comped = snapshot.items.iter().find(|i| i.instance_id == "stamp_reward::cmd-2").unwrap();
        assert_eq!(comped.quantity, 3);
        assert!(comped.is_comped);

        // Total: 7 * 2.00 = 14.00 (3 comped items are free)
        assert!((snapshot.total - 14.0).abs() < 0.01);
    }

    #[test]
    fn test_comp_existing_partial_preserves_unpaid_quantity() {
        // Item qty=5, unpaid_qty=5, reward_qty=2
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(create_test_item("item-1", 50, 3.00, 5));
        money::recalculate_totals(&mut snapshot);

        let event = create_comp_existing_event("order-1", 1, "stamp_reward::cmd-2", "item-1", 2);
        let applier = StampRedeemedApplier;
        applier.apply(&mut snapshot, &event);

        let source = snapshot.items.iter().find(|i| i.instance_id == "item-1").unwrap();
        assert_eq!(source.quantity, 3);
        assert_eq!(source.unpaid_quantity, 3);

        let comped = snapshot.items.iter().find(|i| i.instance_id == "stamp_reward::cmd-2").unwrap();
        assert_eq!(comped.quantity, 2);
        assert_eq!(comped.unpaid_quantity, 2);
    }

    #[test]
    fn test_comp_existing_with_other_items_in_order() {
        // Order has multiple items, only one gets partially comped
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(create_test_item("coffee-1", 10, 3.50, 5)); // 5 coffees
        snapshot.items.push(create_test_item("potato-1", 50, 4.50, 7)); // 7 potatoes (target)
        snapshot.items.push(create_test_item("cake-1", 20, 5.00, 1));   // 1 cake
        money::recalculate_totals(&mut snapshot);

        let event = create_comp_existing_event("order-1", 1, "stamp_reward::cmd-2", "potato-1", 1);
        let applier = StampRedeemedApplier;
        applier.apply(&mut snapshot, &event);

        // 4 items now (coffee, potato-6, cake, comped-potato-1)
        assert_eq!(snapshot.items.len(), 4);

        // Coffee untouched
        let coffee = snapshot.items.iter().find(|i| i.instance_id == "coffee-1").unwrap();
        assert_eq!(coffee.quantity, 5);
        assert!(!coffee.is_comped);

        // Potato reduced to 6
        let potato = snapshot.items.iter().find(|i| i.instance_id == "potato-1").unwrap();
        assert_eq!(potato.quantity, 6);
        assert!(!potato.is_comped);

        // Cake untouched
        let cake = snapshot.items.iter().find(|i| i.instance_id == "cake-1").unwrap();
        assert_eq!(cake.quantity, 1);
        assert!(!cake.is_comped);

        // New comped potato
        let comped = snapshot.items.iter().find(|i| i.instance_id == "stamp_reward::cmd-2").unwrap();
        assert_eq!(comped.quantity, 1);
        assert!(comped.is_comped);
    }

    #[test]
    fn test_comp_existing_checksum_updates() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(create_test_item("item-1", 50, 4.50, 3));
        money::recalculate_totals(&mut snapshot);
        snapshot.update_checksum();
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_comp_existing_event("order-1", 1, "stamp_reward::cmd-2", "item-1", 1);
        let applier = StampRedeemedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }
}
