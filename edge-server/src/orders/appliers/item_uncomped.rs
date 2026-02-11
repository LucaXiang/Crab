//! ItemUncomped event applier
//!
//! Applies the ItemUncomped event to reverse a comp.
//! If merged_into is Some, merges the comped item back into the source.
//! If merged_into is None, restores the item's price in place.

use crate::orders::money;
use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

/// ItemUncomped applier
pub struct ItemUncompedApplier;

impl EventApplier for ItemUncompedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::ItemUncomped {
            instance_id,
            restored_price,
            merged_into,
            ..
        } = &event.payload
        {
            if let Some(source_id) = merged_into {
                // Merge back: find comped item and source item
                let comped_qty = snapshot
                    .items
                    .iter()
                    .find(|i| i.instance_id == *instance_id)
                    .map(|i| (i.quantity, i.unpaid_quantity));

                if let Some((qty, unpaid_qty)) = comped_qty {
                    // Add quantity back to source item
                    if let Some(source) = snapshot.items.iter_mut().find(|i| i.instance_id == *source_id) {
                        source.quantity += qty;
                        source.unpaid_quantity += unpaid_qty;
                    }

                    // Remove comped item from items array
                    snapshot.items.retain(|i| i.instance_id != *instance_id);
                }
            } else {
                // No merge: restore price in place
                if let Some(item) = snapshot.items.iter_mut().find(|i| i.instance_id == *instance_id) {
                    item.is_comped = false;
                    item.price = *restored_price;
                    // original_price stays as-is (it was the pre-comp price)
                }
            }

            // Remove CompRecord where instance_id matches
            snapshot.comps.retain(|c| c.instance_id != *instance_id);

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
    use shared::order::{CartItemSnapshot, CompRecord, OrderEventType};

    fn create_test_item(
        instance_id: &str,
        product_id: i64,
        name: &str,
        price: f64,
        quantity: i32,
        is_comped: bool,
    ) -> CartItemSnapshot {
        CartItemSnapshot {
            id: product_id,
            instance_id: instance_id.to_string(),
            name: name.to_string(),
            price,
            original_price: if is_comped { 10.0 } else { 0.0 },
            quantity,
            unpaid_quantity: quantity,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: 0.0,
            rule_surcharge_amount: 0.0,
            applied_rules: vec![],
            applied_mg_rules: vec![],
            unit_price: 0.0,
            line_total: 0.0,
            tax: 0.0,
            tax_rate: 0,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped,
        }
    }

    fn create_comp_record(
        instance_id: &str,
        source_instance_id: &str,
        original_price: f64,
    ) -> CompRecord {
        CompRecord {
            comp_id: "comp-1".to_string(),
            instance_id: instance_id.to_string(),
            source_instance_id: source_instance_id.to_string(),
            item_name: "Product A".to_string(),
            quantity: 2,
            original_price,
            reason: "VIP".to_string(),
            authorizer_id: 100,
            authorizer_name: "Manager".to_string(),
            timestamp: 1234567890,
        }
    }

    fn create_item_uncomped_event(
        order_id: &str,
        seq: u64,
        instance_id: &str,
        restored_price: f64,
        merged_into: Option<String>,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            1,
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::ItemUncomped,
            EventPayload::ItemUncomped {
                instance_id: instance_id.to_string(),
                item_name: "Product A".to_string(),
                restored_price,
                merged_into,
                authorizer_id: 100,
                authorizer_name: "Manager".to_string(),
            },
        )
    }

    #[test]
    fn test_uncomp_merge_back() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        // Source item (3 remaining after split)
        snapshot.items.push(create_test_item("item-1", 1, "Product A", 10.0, 3, false));
        // Comped item (2 comped)
        snapshot.items.push(create_test_item("item-1::comp::uuid-1", 1, "Product A", 0.0, 2, true));
        snapshot.comps.push(create_comp_record("item-1::comp::uuid-1", "item-1", 10.0));

        let event = create_item_uncomped_event(
            "order-1", 3, "item-1::comp::uuid-1", 10.0, Some("item-1".to_string()),
        );

        let applier = ItemUncompedApplier;
        applier.apply(&mut snapshot, &event);

        // Comped item should be removed
        assert_eq!(snapshot.items.len(), 1);
        // Source item should have quantity restored
        assert_eq!(snapshot.items[0].instance_id, "item-1");
        assert_eq!(snapshot.items[0].quantity, 5); // 3 + 2
        assert_eq!(snapshot.items[0].unpaid_quantity, 5);
        // CompRecord should be removed
        assert_eq!(snapshot.comps.len(), 0);
        // Totals recalculated
        assert_eq!(snapshot.subtotal, 50.0); // 10 * 5
    }

    #[test]
    fn test_uncomp_no_merge_restore_price() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        // Full comped item (source == instance)
        let mut item = create_test_item("item-1", 1, "Product A", 0.0, 2, true);
        item.original_price = 10.0;
        snapshot.items.push(item);
        snapshot.comps.push(create_comp_record("item-1", "item-1", 10.0));

        let event = create_item_uncomped_event("order-1", 3, "item-1", 10.0, None);

        let applier = ItemUncompedApplier;
        applier.apply(&mut snapshot, &event);

        // Item should have price restored
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "item-1");
        assert!(!snapshot.items[0].is_comped);
        assert_eq!(snapshot.items[0].price, 10.0);
        assert_eq!(snapshot.items[0].original_price, 10.0); // Preserved
        // CompRecord should be removed
        assert_eq!(snapshot.comps.len(), 0);
        // Totals recalculated
        assert_eq!(snapshot.subtotal, 20.0); // 10 * 2
    }

    #[test]
    fn test_uncomp_updates_checksum() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let mut item = create_test_item("item-1", 1, "Product A", 0.0, 1, true);
        item.original_price = 10.0;
        snapshot.items.push(item);
        snapshot.comps.push(create_comp_record("item-1", "item-1", 10.0));
        snapshot.update_checksum();
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_item_uncomped_event("order-1", 1, "item-1", 10.0, None);

        let applier = ItemUncompedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_uncomp_updates_sequence() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let mut item = create_test_item("item-1", 1, "Product A", 0.0, 1, true);
        item.original_price = 10.0;
        snapshot.items.push(item);
        snapshot.comps.push(create_comp_record("item-1", "item-1", 10.0));
        snapshot.last_sequence = 5;

        let event = create_item_uncomped_event("order-1", 6, "item-1", 10.0, None);

        let applier = ItemUncompedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 6);
    }
}
