//! ItemComped event applier
//!
//! Applies the ItemComped event to mark an item as comped (gifted).
//! Handles both full comp and partial comp (split + mark).
//! Uses source_instance_id for deterministic replay.

use crate::orders::money;
use crate::orders::traits::EventApplier;
use shared::order::{CompRecord, EventPayload, OrderEvent, OrderSnapshot};

/// ItemComped applier
pub struct ItemCompedApplier;

impl EventApplier for ItemCompedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::ItemComped {
            instance_id,
            source_instance_id,
            item_name,
            quantity,
            original_price,
            reason,
            authorizer_id,
            authorizer_name,
        } = &event.payload
        {
            let is_full_comp = instance_id == source_instance_id;

            if is_full_comp {
                // Full comp: find item by instance_id and mark as comped
                if let Some(item) = snapshot.items.iter_mut().find(|i| i.instance_id == *instance_id) {
                    // Save original_price if not already set
                    if item.original_price == 0.0 {
                        item.original_price = item.price;
                    }
                    item.is_comped = true;
                    item.price = 0.0;
                    // Keep applied_rules and manual_discount_percent intact:
                    // calculate_unit_price() returns 0 for comped items anyway,
                    // and preserving them allows correct restoration on uncomp.
                }
            } else {
                // Partial comp: find SOURCE item by source_instance_id (deterministic!)
                if let Some(source_idx) = snapshot
                    .items
                    .iter()
                    .position(|i| i.instance_id == *source_instance_id)
                {
                    let source = snapshot.items[source_idx].clone();

                    // Reduce source qty and unpaid_qty
                    snapshot.items[source_idx].quantity -= quantity;
                    snapshot.items[source_idx].unpaid_quantity =
                        (snapshot.items[source_idx].unpaid_quantity - quantity).max(0);

                    // Create new comped item as a split
                    // Clone preserves applied_rules and manual_discount_percent
                    // for correct restoration on uncomp.
                    let mut comped_item = source;
                    comped_item.instance_id = instance_id.to_string();
                    comped_item.quantity = *quantity;
                    comped_item.unpaid_quantity = *quantity;
                    comped_item.is_comped = true;
                    comped_item.price = 0.0;
                    comped_item.original_price = *original_price;

                    snapshot.items.push(comped_item);
                }
            }

            // Create CompRecord and push to snapshot.comps
            let comp_record = CompRecord {
                comp_id: event.event_id.clone(),
                instance_id: instance_id.clone(),
                source_instance_id: source_instance_id.clone(),
                item_name: item_name.clone(),
                quantity: *quantity,
                original_price: *original_price,
                reason: reason.clone(),
                authorizer_id: *authorizer_id,
                authorizer_name: authorizer_name.clone(),
                timestamp: event.timestamp,
            };
            snapshot.comps.push(comp_record);

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

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::{CartItemSnapshot, OrderEventType};

    fn create_test_item(
        instance_id: &str,
        product_id: i64,
        name: &str,
        price: f64,
        quantity: i32,
    ) -> CartItemSnapshot {
        CartItemSnapshot {
            id: product_id,
            instance_id: instance_id.to_string(),
            name: name.to_string(),
            price,
            original_price: 0.0,
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
            is_comped: false,
        }
    }

    fn create_item_comped_event(
        order_id: &str,
        seq: u64,
        instance_id: &str,
        source_instance_id: &str,
        item_name: &str,
        quantity: i32,
        original_price: f64,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            1,
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::ItemComped,
            EventPayload::ItemComped {
                instance_id: instance_id.to_string(),
                source_instance_id: source_instance_id.to_string(),
                item_name: item_name.to_string(),
                quantity,
                original_price,
                reason: "VIP customer".to_string(),
                authorizer_id: 1,
                authorizer_name: "Manager".to_string(),
            },
        )
    }

    #[test]
    fn test_item_comped_full() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 2));
        snapshot.subtotal = 20.0;
        snapshot.total = 20.0;

        let event = create_item_comped_event("order-1", 2, "item-1", "item-1", "Product A", 2, 10.0);

        let applier = ItemCompedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items.len(), 1);
        assert!(snapshot.items[0].is_comped);
        assert_eq!(snapshot.items[0].price, 0.0);
        assert_eq!(snapshot.items[0].quantity, 2);
        // original_price should be saved
        assert_eq!(snapshot.items[0].original_price, 10.0);
        // Total should be 0 after comp
        assert_eq!(snapshot.subtotal, 0.0);
        assert_eq!(snapshot.total, 0.0);
        assert_eq!(snapshot.last_sequence, 2);

        // CompRecord should be created
        assert_eq!(snapshot.comps.len(), 1);
        assert_eq!(snapshot.comps[0].instance_id, "item-1");
        assert_eq!(snapshot.comps[0].source_instance_id, "item-1");
        assert_eq!(snapshot.comps[0].original_price, 10.0);
    }

    #[test]
    fn test_item_comped_full_preserves_existing_original_price() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let mut item = create_test_item("item-1", 1, "Product A", 8.0, 1);
        item.original_price = 12.0; // Already has original_price
        snapshot.items.push(item);

        let event = create_item_comped_event("order-1", 2, "item-1", "item-1", "Product A", 1, 12.0);

        let applier = ItemCompedApplier;
        applier.apply(&mut snapshot, &event);

        // original_price should be preserved (12.0), not overwritten to 8.0
        assert_eq!(snapshot.items[0].original_price, 12.0);
        assert_eq!(snapshot.items[0].price, 0.0);
        assert!(snapshot.items[0].is_comped);
    }

    #[test]
    fn test_item_comped_partial_split() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 5));
        snapshot.subtotal = 50.0;
        snapshot.total = 50.0;

        // Partial comp: 2 of 5 items (derived instance_id, source is "item-1")
        let event = create_item_comped_event(
            "order-1", 2, "item-1::comp::uuid-1", "item-1", "Product A", 2, 10.0,
        );

        let applier = ItemCompedApplier;
        applier.apply(&mut snapshot, &event);

        // Should have 2 items now
        assert_eq!(snapshot.items.len(), 2);

        // Original item with reduced quantity
        assert_eq!(snapshot.items[0].instance_id, "item-1");
        assert_eq!(snapshot.items[0].quantity, 3);
        assert!(!snapshot.items[0].is_comped);
        assert_eq!(snapshot.items[0].price, 10.0);

        // New comped item
        assert_eq!(snapshot.items[1].instance_id, "item-1::comp::uuid-1");
        assert_eq!(snapshot.items[1].quantity, 2);
        assert!(snapshot.items[1].is_comped);
        assert_eq!(snapshot.items[1].price, 0.0);
        assert_eq!(snapshot.items[1].original_price, 10.0);

        // Totals: 10.0 * 3 + 0.0 * 2 = 30.0
        assert_eq!(snapshot.subtotal, 30.0);
        assert_eq!(snapshot.total, 30.0);

        // CompRecord
        assert_eq!(snapshot.comps.len(), 1);
        assert_eq!(snapshot.comps[0].instance_id, "item-1::comp::uuid-1");
        assert_eq!(snapshot.comps[0].source_instance_id, "item-1");
        assert_eq!(snapshot.comps[0].quantity, 2);
        assert_eq!(snapshot.comps[0].original_price, 10.0);
    }

    #[test]
    fn test_item_comped_updates_checksum() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 1));
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_item_comped_event("order-1", 1, "item-1", "item-1", "Product A", 1, 10.0);

        let applier = ItemCompedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_item_comped_updates_sequence() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 1));
        snapshot.last_sequence = 5;

        let event = create_item_comped_event("order-1", 6, "item-1", "item-1", "Product A", 1, 10.0);

        let applier = ItemCompedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 6);
    }

    #[test]
    fn test_item_comped_preserves_discounts_and_rules() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let mut item = create_test_item("item-1", 1, "Product A", 100.0, 1);
        item.manual_discount_percent = Some(10.0);
        item.rule_discount_amount = 5.0;
        item.rule_surcharge_amount = 2.0;
        snapshot.items.push(item);

        let event = create_item_comped_event("order-1", 1, "item-1", "item-1", "Product A", 1, 100.0);

        let applier = ItemCompedApplier;
        applier.apply(&mut snapshot, &event);

        assert!(snapshot.items[0].is_comped);
        assert_eq!(snapshot.items[0].price, 0.0);
        // Discounts and rules are preserved for uncomp restoration
        assert_eq!(snapshot.items[0].manual_discount_percent, Some(10.0));
        assert_eq!(snapshot.items[0].rule_discount_amount, 5.0);
        assert_eq!(snapshot.items[0].rule_surcharge_amount, 2.0);
    }

    #[test]
    fn test_item_comped_original_price_not_zero() {
        // This test verifies the bug fix: original_price should NOT be 0.0
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 15.50, 1));

        let event = create_item_comped_event("order-1", 1, "item-1", "item-1", "Product A", 1, 15.50);

        let applier = ItemCompedApplier;
        applier.apply(&mut snapshot, &event);

        // BUG FIX: original_price must be the real price, not 0.0
        assert_eq!(snapshot.items[0].original_price, 15.50);
        assert_eq!(snapshot.items[0].price, 0.0);
        // CompRecord must also have correct original_price
        assert_eq!(snapshot.comps[0].original_price, 15.50);
    }
}
