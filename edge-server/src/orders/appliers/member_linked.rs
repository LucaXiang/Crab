//! MemberLinked event applier
//!
//! Applies the MemberLinked event to set member info and MG discounts on the snapshot.

use crate::orders::money;
use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

/// MemberLinked applier
pub struct MemberLinkedApplier;

impl EventApplier for MemberLinkedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::MemberLinked {
            member_id,
            member_name,
            marketing_group_id,
            marketing_group_name,
            mg_item_discounts,
        } = &event.payload
        {
            snapshot.member_id = Some(*member_id);
            snapshot.member_name = Some(member_name.clone());
            snapshot.marketing_group_id = Some(*marketing_group_id);
            snapshot.marketing_group_name = Some(marketing_group_name.clone());

            // Apply pre-calculated MG discounts to items
            for discount in mg_item_discounts {
                if let Some(item) = snapshot
                    .items
                    .iter_mut()
                    .find(|i| i.instance_id == discount.instance_id)
                {
                    item.applied_mg_rules = discount.applied_mg_rules.clone();
                }
            }

            // Update sequence and timestamp
            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;

            // Recalculate totals (now including MG discounts)
            money::recalculate_totals(snapshot);

            // Update checksum
            snapshot.update_checksum();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::models::price_rule::{AdjustmentType, ProductScope};
    use shared::order::{AppliedMgRule, CartItemSnapshot, MgItemDiscount, OrderEventType, OrderSnapshot};

    fn create_member_linked_event(
        order_id: &str,
        seq: u64,
        member_id: i64,
        member_name: &str,
        mg_id: i64,
        mg_name: &str,
        mg_item_discounts: Vec<MgItemDiscount>,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            1,
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::MemberLinked,
            EventPayload::MemberLinked {
                member_id,
                member_name: member_name.to_string(),
                marketing_group_id: mg_id,
                marketing_group_name: mg_name.to_string(),
                mg_item_discounts,
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
    fn test_member_linked_sets_fields() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        assert!(snapshot.member_id.is_none());
        assert!(snapshot.marketing_group_id.is_none());

        let event = create_member_linked_event("order-1", 2, 42, "Alice", 1, "VIP", vec![]);
        let applier = MemberLinkedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.member_id, Some(42));
        assert_eq!(snapshot.member_name, Some("Alice".to_string()));
        assert_eq!(snapshot.marketing_group_id, Some(1));
        assert_eq!(snapshot.marketing_group_name, Some("VIP".to_string()));
        assert_eq!(snapshot.last_sequence, 2);
    }

    #[test]
    fn test_member_linked_replaces_existing() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.member_id = Some(10);
        snapshot.member_name = Some("Bob".to_string());
        snapshot.marketing_group_id = Some(5);
        snapshot.marketing_group_name = Some("Regular".to_string());

        let event = create_member_linked_event("order-1", 3, 42, "Alice", 1, "VIP", vec![]);
        let applier = MemberLinkedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.member_id, Some(42));
        assert_eq!(snapshot.member_name, Some("Alice".to_string()));
        assert_eq!(snapshot.marketing_group_id, Some(1));
        assert_eq!(snapshot.marketing_group_name, Some("VIP".to_string()));
    }

    #[test]
    fn test_member_linked_applies_mg_discounts() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(create_test_item("inst-1", 100.0));
        snapshot.subtotal = 100.0;
        snapshot.total = 100.0;

        let mg_item_discounts = vec![MgItemDiscount {
            instance_id: "inst-1".to_string(),
            applied_mg_rules: vec![AppliedMgRule {
                rule_id: 1,
                name: "vip-10".to_string(),
                display_name: "VIP 10%".to_string(),
                receipt_name: "VIP10".to_string(),
                product_scope: ProductScope::Global,
                adjustment_type: AdjustmentType::Percentage,
                adjustment_value: 10.0,
                calculated_amount: 10.0,
                skipped: false,
            }],
        }];

        let event =
            create_member_linked_event("order-1", 2, 42, "Alice", 1, "VIP", mg_item_discounts);
        let applier = MemberLinkedApplier;
        applier.apply(&mut snapshot, &event);

        // Item should have MG rules applied
        assert_eq!(snapshot.items[0].applied_mg_rules.len(), 1);
        // Totals should reflect MG discount: 100 * 0.9 = 90
        assert_eq!(snapshot.subtotal, 90.0);
        assert_eq!(snapshot.total, 90.0);
        assert_eq!(snapshot.mg_discount_amount, 10.0);
    }

    #[test]
    fn test_member_linked_updates_checksum() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_member_linked_event("order-1", 1, 42, "Alice", 1, "VIP", vec![]);
        let applier = MemberLinkedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }
}
