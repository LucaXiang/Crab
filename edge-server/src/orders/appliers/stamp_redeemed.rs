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

            if let Some(existing_id) = comp_existing_instance_id {
                // Match mode: comp the existing item
                if let Some(item) = snapshot.items.iter_mut().find(|i| i.instance_id == *existing_id) {
                    item.is_comped = true;
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
}
