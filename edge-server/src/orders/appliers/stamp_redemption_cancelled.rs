//! StampRedemptionCancelled event applier
//!
//! Removes the reward item and the stamp_redemption record from the snapshot.
//! Recalculates totals since a comped item is being removed.

use crate::orders::money;
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
            ..
        } = &event.payload
        {
            if *is_comp_existing {
                // Match mode: uncomp the existing item
                if let Some(item) = snapshot.items.iter_mut().find(|i| i.instance_id == *reward_instance_id) {
                    item.is_comped = false;
                }
            } else {
                // Add-new mode: remove the reward item
                snapshot.items.retain(|item| item.instance_id != *reward_instance_id);
            }

            // Remove the stamp_redemption record
            snapshot
                .stamp_redemptions
                .retain(|r| r.stamp_activity_id != *stamp_activity_id);

            // 3. Update sequence and timestamp
            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;

            // 4. Recalculate totals
            money::recalculate_totals(snapshot);

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
        snapshot.items.push(create_reward_item("stamp_reward::prev-cmd"));
        snapshot.stamp_redemptions.push(StampRedemptionState {
            stamp_activity_id: 1,
            reward_instance_id: "stamp_reward::prev-cmd".to_string(),
            is_comp_existing: false,
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
        snapshot.items.push(create_reward_item("stamp_reward::prev-cmd"));
        snapshot.stamp_redemptions.push(StampRedemptionState {
            stamp_activity_id: 1,
            reward_instance_id: "stamp_reward::prev-cmd".to_string(),
            is_comp_existing: false,
        });
        money::recalculate_totals(&mut snapshot);

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
        snapshot.items.push(create_reward_item("stamp_reward::prev-cmd"));
        snapshot.stamp_redemptions.push(StampRedemptionState {
            stamp_activity_id: 1,
            reward_instance_id: "stamp_reward::prev-cmd".to_string(),
            is_comp_existing: false,
        });
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_cancel_event("order-1", 5);
        let applier = StampRedemptionCancelledApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 5);
        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }
}
