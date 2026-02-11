//! MemberUnlinked event applier
//!
//! Clears member info and MG discount data from the snapshot.
//! Recalculates totals since MG discounts are removed.

use crate::orders::money;
use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

/// MemberUnlinked applier
pub struct MemberUnlinkedApplier;

impl EventApplier for MemberUnlinkedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::MemberUnlinked { .. } = &event.payload {
            // Clear member info
            snapshot.member_id = None;
            snapshot.member_name = None;
            snapshot.marketing_group_id = None;
            snapshot.marketing_group_name = None;

            // Clear MG discount data from all items
            for item in &mut snapshot.items {
                item.applied_mg_rules.clear();
            }
            snapshot.mg_discount_amount = 0.0;

            // Reverse any pending stamp redemptions
            if !snapshot.stamp_redemptions.is_empty() {
                let mut remove_ids = Vec::new();
                // Collect partial comp merge-back info: (source_id, comped_qty)
                let mut merge_backs: Vec<(String, i32)> = Vec::new();
                for r in &snapshot.stamp_redemptions {
                    if r.is_comp_existing {
                        if let Some(source_id) = &r.comp_source_instance_id {
                            // Partial comp: collect for merge-back
                            let comped_qty = snapshot
                                .items
                                .iter()
                                .find(|i| i.instance_id == r.reward_instance_id)
                                .map(|i| i.quantity)
                                .unwrap_or(0);
                            merge_backs.push((source_id.clone(), comped_qty));
                            remove_ids.push(r.reward_instance_id.clone());
                        } else {
                            // Full comp: uncomp the existing item
                            if let Some(item) = snapshot.items.iter_mut().find(|i| i.instance_id == r.reward_instance_id) {
                                item.is_comped = false;
                            }
                        }
                    } else {
                        // Add-new mode: collect for removal
                        remove_ids.push(r.reward_instance_id.clone());
                    }
                }
                // Apply merge-backs: restore quantity to source items
                for (source_id, qty) in &merge_backs {
                    if let Some(source) = snapshot.items.iter_mut().find(|i| i.instance_id == *source_id) {
                        source.quantity += qty;
                        source.unpaid_quantity += qty;
                    }
                }
                if !remove_ids.is_empty() {
                    snapshot.items.retain(|item| !remove_ids.contains(&item.instance_id));
                }
                snapshot.stamp_redemptions.clear();
            }

            // Update sequence and timestamp
            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;

            // Recalculate totals (MG discounts removed)
            money::recalculate_totals(snapshot);

            // Update checksum
            snapshot.update_checksum();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::{CartItemSnapshot, OrderEventType, OrderSnapshot};

    fn create_member_unlinked_event(order_id: &str, seq: u64) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            1,
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::MemberUnlinked,
            EventPayload::MemberUnlinked {
                previous_member_id: 42,
                previous_member_name: "Alice".to_string(),
            },
        )
    }

    fn create_test_item(instance_id: &str, price: f64) -> CartItemSnapshot {
        CartItemSnapshot {
            id: 1,
            instance_id: instance_id.to_string(),
            name: "Product".to_string(),
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
    fn test_member_unlinked_clears_fields() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.member_id = Some(42);
        snapshot.member_name = Some("Alice".to_string());
        snapshot.marketing_group_id = Some(1);
        snapshot.marketing_group_name = Some("VIP".to_string());
        snapshot.mg_discount_amount = 5.0;
        snapshot.items.push(create_test_item("inst-1", 10.0));

        let event = create_member_unlinked_event("order-1", 2);
        let applier = MemberUnlinkedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.member_id, None);
        assert_eq!(snapshot.member_name, None);
        assert_eq!(snapshot.marketing_group_id, None);
        assert_eq!(snapshot.marketing_group_name, None);
        assert_eq!(snapshot.mg_discount_amount, 0.0);
        assert!(snapshot.items[0].applied_mg_rules.is_empty());
        assert_eq!(snapshot.last_sequence, 2);
    }

    #[test]
    fn test_member_unlinked_recalculates_totals() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.member_id = Some(42);
        snapshot.items.push(create_test_item("inst-1", 10.0));
        snapshot.subtotal = 10.0;
        snapshot.total = 10.0;

        let event = create_member_unlinked_event("order-1", 2);
        let applier = MemberUnlinkedApplier;
        applier.apply(&mut snapshot, &event);

        // Totals should be recalculated
        assert_eq!(snapshot.subtotal, 10.0);
        assert_eq!(snapshot.total, 10.0);
    }

    #[test]
    fn test_member_unlinked_updates_checksum() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.member_id = Some(42);
        snapshot.member_name = Some("Alice".to_string());
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_member_unlinked_event("order-1", 1);
        let applier = MemberUnlinkedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    // =========================================================================
    // Member unlink with stamp redemption reversal
    // =========================================================================

    use shared::order::StampRedemptionState;

    fn create_comped_item(instance_id: &str, price: f64, quantity: i32) -> CartItemSnapshot {
        CartItemSnapshot {
            id: 1,
            instance_id: instance_id.to_string(),
            name: "Product".to_string(),
            price: 0.0,
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
            unit_price: 0.0,
            line_total: 0.0,
            tax: 0.0,
            tax_rate: 0,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_id: None,
            category_name: None,
            is_comped: true,
        }
    }

    #[test]
    fn test_unlink_reverses_full_comp_existing() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.member_id = Some(42);
        snapshot.member_name = Some("Alice".to_string());

        // Full comp: item-1 was fully comped
        let mut item = create_test_item("item-1", 4.50);
        item.is_comped = true;
        snapshot.items.push(item);
        snapshot.stamp_redemptions.push(StampRedemptionState {
            stamp_activity_id: 1,
            reward_instance_id: "item-1".to_string(),
            is_comp_existing: true,
            comp_source_instance_id: None,
        });

        let event = create_member_unlinked_event("order-1", 2);
        MemberUnlinkedApplier.apply(&mut snapshot, &event);

        // Item uncomped
        assert_eq!(snapshot.items.len(), 1);
        assert!(!snapshot.items[0].is_comped);
        assert_eq!(snapshot.items[0].instance_id, "item-1");
        assert!(snapshot.stamp_redemptions.is_empty());
    }

    #[test]
    fn test_unlink_reverses_partial_comp_existing() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.member_id = Some(42);
        snapshot.member_name = Some("Alice".to_string());

        // Partial comp: item-1 had qty=7, split to 6 + 1 comped
        snapshot.items.push(create_test_item("item-1", 4.50));
        snapshot.items[0].quantity = 6;
        snapshot.items[0].unpaid_quantity = 6;
        snapshot.items.push(create_comped_item("stamp_reward::cmd-2", 4.50, 1));
        snapshot.stamp_redemptions.push(StampRedemptionState {
            stamp_activity_id: 1,
            reward_instance_id: "stamp_reward::cmd-2".to_string(),
            is_comp_existing: true,
            comp_source_instance_id: Some("item-1".to_string()),
        });

        let event = create_member_unlinked_event("order-1", 2);
        MemberUnlinkedApplier.apply(&mut snapshot, &event);

        // Comped item removed, source restored
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "item-1");
        assert_eq!(snapshot.items[0].quantity, 7); // 6 + 1
        assert_eq!(snapshot.items[0].unpaid_quantity, 7);
        assert!(!snapshot.items[0].is_comped);
        assert!(snapshot.stamp_redemptions.is_empty());
    }

    #[test]
    fn test_unlink_reverses_add_new_redemption() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.member_id = Some(42);
        snapshot.member_name = Some("Alice".to_string());

        // Add-new mode: separate reward item
        snapshot.items.push(create_test_item("inst-1", 5.00));
        snapshot.items.push(create_comped_item("stamp_reward::cmd-1", 3.50, 1));
        snapshot.stamp_redemptions.push(StampRedemptionState {
            stamp_activity_id: 1,
            reward_instance_id: "stamp_reward::cmd-1".to_string(),
            is_comp_existing: false,
            comp_source_instance_id: None,
        });

        let event = create_member_unlinked_event("order-1", 2);
        MemberUnlinkedApplier.apply(&mut snapshot, &event);

        // Reward item removed, original item unchanged
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "inst-1");
        assert!(snapshot.stamp_redemptions.is_empty());
    }

    #[test]
    fn test_unlink_reverses_mixed_redemptions() {
        // Two stamp activities: one add-new, one partial comp-existing
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.member_id = Some(42);
        snapshot.member_name = Some("Alice".to_string());

        snapshot.items.push(create_test_item("item-1", 4.50));
        snapshot.items[0].quantity = 6;
        snapshot.items[0].unpaid_quantity = 6;
        snapshot.items.push(create_comped_item("stamp_reward::partial", 4.50, 1)); // partial comp split
        snapshot.items.push(create_comped_item("stamp_reward::added", 3.00, 1)); // add-new

        snapshot.stamp_redemptions.push(StampRedemptionState {
            stamp_activity_id: 1,
            reward_instance_id: "stamp_reward::partial".to_string(),
            is_comp_existing: true,
            comp_source_instance_id: Some("item-1".to_string()),
        });
        snapshot.stamp_redemptions.push(StampRedemptionState {
            stamp_activity_id: 2,
            reward_instance_id: "stamp_reward::added".to_string(),
            is_comp_existing: false,
            comp_source_instance_id: None,
        });

        let event = create_member_unlinked_event("order-1", 2);
        MemberUnlinkedApplier.apply(&mut snapshot, &event);

        // Both reward items removed/merged, only source remains
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "item-1");
        assert_eq!(snapshot.items[0].quantity, 7); // 6 + 1 merged back
        assert!(snapshot.stamp_redemptions.is_empty());
    }
}
