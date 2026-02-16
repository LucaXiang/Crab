//! StampRedemptionCancelled event applier
//!
//! Removes the reward item and the stamp_redemption record from the snapshot.
//! Recalculates totals since a comped item is being removed.

use crate::order_money;
use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

/// StampRedemptionCancelled applier
pub struct StampRedemptionCancelledApplier;

impl EventApplier for StampRedemptionCancelledApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::StampRedemptionCancelled {
            stamp_activity_id,
            reward_instance_id,
            is_comp_existing,
            comp_source_instance_id,
            ..
        } = &event.payload
        {
            if *is_comp_existing {
                if let Some(source_id) = comp_source_instance_id {
                    // Partial comp reversal: merge comped item back into source
                    let comped_qty = snapshot
                        .items
                        .iter()
                        .find(|i| i.instance_id == *reward_instance_id)
                        .map(|i| i.quantity)
                        .unwrap_or(0);
                    // Add quantity back to source item
                    if let Some(source) = snapshot
                        .items
                        .iter_mut()
                        .find(|i| i.instance_id == *source_id)
                    {
                        source.quantity += comped_qty;
                        source.unpaid_quantity += comped_qty;
                    }
                    // Remove the comped split item
                    snapshot
                        .items
                        .retain(|item| item.instance_id != *reward_instance_id);
                } else {
                    // Full comp reversal: uncomp the existing item
                    if let Some(item) = snapshot
                        .items
                        .iter_mut()
                        .find(|i| i.instance_id == *reward_instance_id)
                    {
                        item.is_comped = false;
                    }
                }
            } else {
                // Add-new mode: remove the reward item
                snapshot
                    .items
                    .retain(|item| item.instance_id != *reward_instance_id);
            }

            // Remove the stamp_redemption record
            snapshot
                .stamp_redemptions
                .retain(|r| r.stamp_activity_id != *stamp_activity_id);

            // 3. Update sequence and timestamp
            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;

            // 4. Recalculate totals
            order_money::recalculate_totals(snapshot);

            // 5. Update checksum
            snapshot.update_checksum();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::{CartItemSnapshot, OrderEventType, OrderSnapshot, StampRedemptionState};

    fn create_cancel_event(order_id: &str, seq: u64) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            1,
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::StampRedemptionCancelled,
            EventPayload::StampRedemptionCancelled {
                stamp_activity_id: 1,
                stamp_activity_name: "Coffee Card".to_string(),
                reward_instance_id: "stamp_reward::prev-cmd".to_string(),
                is_comp_existing: false,
                comp_source_instance_id: None,
            },
        )
    }

    fn create_reward_item(instance_id: &str) -> CartItemSnapshot {
        CartItemSnapshot {
            id: 100,
            instance_id: instance_id.to_string(),
            name: "Coffee".to_string(),
            price: 0.0,
            original_price: 3.50,
            quantity: 1,
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
            tax_rate: 10,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_id: None,
            category_name: Some("Drinks".to_string()),
            is_comped: true,
        }
    }

    fn create_paid_item(instance_id: &str, price: f64) -> CartItemSnapshot {
        CartItemSnapshot {
            id: 200,
            instance_id: instance_id.to_string(),
            name: "Cake".to_string(),
            price,
            original_price: price,
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
            unit_price: price,
            line_total: price,
            tax: 0.0,
            tax_rate: 0,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_id: None,
            category_name: None,
            is_comped: false,
        }
    }

    #[test]
    fn test_cancel_removes_reward_item() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(create_paid_item("inst-1", 5.0));
        snapshot
            .items
            .push(create_reward_item("stamp_reward::prev-cmd"));
        snapshot.stamp_redemptions.push(StampRedemptionState {
            stamp_activity_id: 1,
            reward_instance_id: "stamp_reward::prev-cmd".to_string(),
            is_comp_existing: false,
            comp_source_instance_id: None,
        });

        let event = create_cancel_event("order-1", 2);
        let applier = StampRedemptionCancelledApplier;
        applier.apply(&mut snapshot, &event);

        // Reward item removed, paid item remains
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "inst-1");
        // Redemption record removed
        assert!(snapshot.stamp_redemptions.is_empty());
    }

    #[test]
    fn test_cancel_recalculates_totals() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(create_paid_item("inst-1", 5.0));
        snapshot
            .items
            .push(create_reward_item("stamp_reward::prev-cmd"));
        snapshot.stamp_redemptions.push(StampRedemptionState {
            stamp_activity_id: 1,
            reward_instance_id: "stamp_reward::prev-cmd".to_string(),
            is_comp_existing: false,
            comp_source_instance_id: None,
        });
        order_money::recalculate_totals(&mut snapshot);

        let event = create_cancel_event("order-1", 2);
        let applier = StampRedemptionCancelledApplier;
        applier.apply(&mut snapshot, &event);

        // Total should still be 5.0 (comped item was free anyway)
        assert!((snapshot.total - 5.00).abs() < f64::EPSILON);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_cancel_updates_sequence_and_checksum() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_reward_item("stamp_reward::prev-cmd"));
        snapshot.stamp_redemptions.push(StampRedemptionState {
            stamp_activity_id: 1,
            reward_instance_id: "stamp_reward::prev-cmd".to_string(),
            is_comp_existing: false,
            comp_source_instance_id: None,
        });
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_cancel_event("order-1", 5);
        let applier = StampRedemptionCancelledApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 5);
        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    // =========================================================================
    // Comp-existing cancellation tests
    // =========================================================================

    fn create_test_item(
        instance_id: &str,
        product_id: i64,
        price: f64,
        quantity: i32,
    ) -> CartItemSnapshot {
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

    fn create_comped_item(
        instance_id: &str,
        product_id: i64,
        original_price: f64,
        quantity: i32,
    ) -> CartItemSnapshot {
        CartItemSnapshot {
            id: product_id,
            instance_id: instance_id.to_string(),
            name: format!("Product {}", product_id),
            price: 0.0,
            original_price,
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
            unit_price: 0.0,
            line_total: 0.0,
            tax: 0.0,
            tax_rate: 10,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_id: Some(1),
            category_name: Some("Food".to_string()),
            is_comped: true,
        }
    }

    fn create_cancel_comp_existing_event(
        order_id: &str,
        seq: u64,
        reward_instance_id: &str,
        comp_source_instance_id: Option<&str>,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            1,
            "Test User".to_string(),
            "cmd-cancel".to_string(),
            Some(1234567890),
            OrderEventType::StampRedemptionCancelled,
            EventPayload::StampRedemptionCancelled {
                stamp_activity_id: 1,
                stamp_activity_name: "Coffee Card".to_string(),
                reward_instance_id: reward_instance_id.to_string(),
                is_comp_existing: true,
                comp_source_instance_id: comp_source_instance_id.map(|s| s.to_string()),
            },
        )
    }

    #[test]
    fn test_cancel_full_comp_existing_uncomps_item() {
        // Full comp: item-1 was fully comped (reward_instance_id == item-1, no source)
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let mut item = create_test_item("item-1", 50, 4.50, 1);
        item.is_comped = true;
        snapshot.items.push(item);
        snapshot.stamp_redemptions.push(StampRedemptionState {
            stamp_activity_id: 1,
            reward_instance_id: "item-1".to_string(),
            is_comp_existing: true,
            comp_source_instance_id: None,
        });
        order_money::recalculate_totals(&mut snapshot);
        assert!((snapshot.total - 0.0).abs() < f64::EPSILON); // comped = free

        let event = create_cancel_comp_existing_event("order-1", 2, "item-1", None);
        let applier = StampRedemptionCancelledApplier;
        applier.apply(&mut snapshot, &event);

        // Item uncomped
        assert_eq!(snapshot.items.len(), 1);
        assert!(!snapshot.items[0].is_comped);
        assert_eq!(snapshot.items[0].instance_id, "item-1");
        assert_eq!(snapshot.items[0].quantity, 1);

        // Total restored
        assert!((snapshot.total - 4.50).abs() < 0.01);

        // Redemption cleared
        assert!(snapshot.stamp_redemptions.is_empty());
    }

    #[test]
    fn test_cancel_partial_comp_existing_merges_back() {
        // Partial comp: item-1 had 7, split to 6 + 1 comped
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(create_test_item("item-1", 50, 4.50, 6)); // source: reduced to 6
        snapshot
            .items
            .push(create_comped_item("stamp_reward::cmd-2", 50, 4.50, 1)); // comped split: 1
        snapshot.stamp_redemptions.push(StampRedemptionState {
            stamp_activity_id: 1,
            reward_instance_id: "stamp_reward::cmd-2".to_string(),
            is_comp_existing: true,
            comp_source_instance_id: Some("item-1".to_string()),
        });
        order_money::recalculate_totals(&mut snapshot);

        let event =
            create_cancel_comp_existing_event("order-1", 2, "stamp_reward::cmd-2", Some("item-1"));
        let applier = StampRedemptionCancelledApplier;
        applier.apply(&mut snapshot, &event);

        // Comped item removed, source restored to 7
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "item-1");
        assert_eq!(snapshot.items[0].quantity, 7);
        assert_eq!(snapshot.items[0].unpaid_quantity, 7);
        assert!(!snapshot.items[0].is_comped);

        // Total: 7 * 4.50 = 31.50
        assert!((snapshot.total - 31.50).abs() < 0.01);

        // Redemption cleared
        assert!(snapshot.stamp_redemptions.is_empty());
    }

    #[test]
    fn test_cancel_partial_comp_with_other_items() {
        // Order: 5 coffees + 6 potatoes (source) + 1 comped potato
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("coffee-1", 10, 3.50, 5));
        snapshot
            .items
            .push(create_test_item("potato-1", 50, 4.50, 6));
        snapshot
            .items
            .push(create_comped_item("stamp_reward::cmd-2", 50, 4.50, 1));
        snapshot.stamp_redemptions.push(StampRedemptionState {
            stamp_activity_id: 1,
            reward_instance_id: "stamp_reward::cmd-2".to_string(),
            is_comp_existing: true,
            comp_source_instance_id: Some("potato-1".to_string()),
        });
        order_money::recalculate_totals(&mut snapshot);

        let event = create_cancel_comp_existing_event(
            "order-1",
            2,
            "stamp_reward::cmd-2",
            Some("potato-1"),
        );
        let applier = StampRedemptionCancelledApplier;
        applier.apply(&mut snapshot, &event);

        // 2 items remain (coffee + restored potato)
        assert_eq!(snapshot.items.len(), 2);

        let coffee = snapshot
            .items
            .iter()
            .find(|i| i.instance_id == "coffee-1")
            .unwrap();
        assert_eq!(coffee.quantity, 5);

        let potato = snapshot
            .items
            .iter()
            .find(|i| i.instance_id == "potato-1")
            .unwrap();
        assert_eq!(potato.quantity, 7); // 6 + 1 merged back
        assert!(!potato.is_comped);
    }

    #[test]
    fn test_cancel_partial_comp_large_quantity_merge() {
        // Item had 10, split to 7 + 3 comped, now cancel
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(create_test_item("item-1", 50, 2.00, 7));
        snapshot
            .items
            .push(create_comped_item("stamp_reward::cmd-2", 50, 2.00, 3));
        snapshot.stamp_redemptions.push(StampRedemptionState {
            stamp_activity_id: 1,
            reward_instance_id: "stamp_reward::cmd-2".to_string(),
            is_comp_existing: true,
            comp_source_instance_id: Some("item-1".to_string()),
        });
        order_money::recalculate_totals(&mut snapshot);

        let event =
            create_cancel_comp_existing_event("order-1", 2, "stamp_reward::cmd-2", Some("item-1"));
        let applier = StampRedemptionCancelledApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].quantity, 10); // 7 + 3 = 10
        assert!((snapshot.total - 20.0).abs() < 0.01); // 10 * 2.00
    }
}
