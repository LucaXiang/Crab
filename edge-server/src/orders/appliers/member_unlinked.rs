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
                for r in &snapshot.stamp_redemptions {
                    if r.is_comp_existing {
                        // Match mode: uncomp the existing item
                        if let Some(item) = snapshot.items.iter_mut().find(|i| i.instance_id == r.reward_instance_id) {
                            item.is_comped = false;
                        }
                    } else {
                        // Add-new mode: collect for removal
                        remove_ids.push(r.reward_instance_id.clone());
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
}
