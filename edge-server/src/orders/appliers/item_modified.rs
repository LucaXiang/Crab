//! ItemModified event applier
//!
//! Applies the ItemModified event to update items in the snapshot.
//! Handles both full modifications and split scenarios.

use crate::orders::money;
use crate::orders::traits::EventApplier;
use shared::order::{
    CartItemSnapshot, EventPayload, ItemChanges, ItemModificationResult, OrderEvent, OrderSnapshot,
};

/// ItemModified applier
pub struct ItemModifiedApplier;

impl EventApplier for ItemModifiedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::ItemModified {
            source,
            affected_quantity,
            changes,
            results,
            ..
        } = &event.payload
        {
            apply_item_modified(snapshot, source, *affected_quantity, changes, results);

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

/// Apply item modification to snapshot
fn apply_item_modified(
    snapshot: &mut OrderSnapshot,
    source: &CartItemSnapshot,
    affected_quantity: i32,
    changes: &ItemChanges,
    results: &[ItemModificationResult],
) {
    // Find the source item
    if let Some(idx) = snapshot
        .items
        .iter()
        .position(|i| i.instance_id == source.instance_id)
    {
        let original_qty = snapshot.items[idx].quantity;

        // Check if this item has been partially paid (split bill)
        let paid_qty = snapshot
            .paid_item_quantities
            .get(&source.instance_id)
            .copied()
            .unwrap_or(0);

        if paid_qty > 0 && affected_quantity >= original_qty {
            // Item has been partially paid - need to split to preserve paid portion
            // 1. Keep paid portion with original instance_id (so split_items can find it)
            snapshot.items[idx].quantity = paid_qty;
            snapshot.items[idx].unpaid_quantity = 0;
            // Don't apply changes or update instance_id for paid portion

            // 2. Create new item for unpaid portion with modifications
            let unpaid_qty = original_qty - paid_qty;
            if unpaid_qty > 0
                && let Some(result) = results.iter().find(|r| r.action == "UPDATED") {
                    let mut new_item = source.clone();
                    new_item.instance_id = result.instance_id.clone();
                    new_item.quantity = unpaid_qty;
                    new_item.unpaid_quantity = unpaid_qty;
                    apply_changes_to_item(&mut new_item, changes);
                    snapshot.items.push(new_item);
                }
        } else if affected_quantity >= original_qty {
            // Full modification (no paid portion): update entire item in place
            apply_changes_to_item(&mut snapshot.items[idx], changes);

            // Update instance_id from results (it may have changed due to modifications)
            if let Some(result) = results.iter().find(|r| r.action == "UPDATED") {
                snapshot.items[idx].instance_id = result.instance_id.clone();
            }
        } else {
            // Partial modification (split scenario):
            // 1. Reduce original item quantity
            snapshot.items[idx].quantity = original_qty - affected_quantity;
            snapshot.items[idx].unpaid_quantity =
                (snapshot.items[idx].unpaid_quantity - affected_quantity).max(0);

            // 2. Create new item(s) from results
            for result in results {
                if result.action == "CREATED" {
                    // Create new item based on source with changes applied
                    let mut new_item = source.clone();
                    new_item.instance_id = result.instance_id.clone();
                    new_item.quantity = result.quantity;
                    new_item.unpaid_quantity = result.quantity;
                    new_item.price = result.price;
                    new_item.manual_discount_percent = result.manual_discount_percent;

                    // Apply additional changes (note, surcharge, options, specification)
                    if let Some(ref note) = changes.note {
                        new_item.note = Some(note.clone());
                    }
                    if let Some(surcharge) = changes.surcharge {
                        new_item.surcharge = Some(surcharge);
                    }
                    if let Some(ref options) = changes.selected_options {
                        new_item.selected_options = Some(options.clone());
                    }
                    if let Some(ref specification) = changes.selected_specification {
                        new_item.selected_specification = Some(specification.clone());
                    }

                    snapshot.items.push(new_item);
                }
            }
        }
    }
}

/// Apply changes to a single item
fn apply_changes_to_item(item: &mut CartItemSnapshot, changes: &ItemChanges) {
    if let Some(price) = changes.price {
        item.price = price;
    }
    if let Some(quantity) = changes.quantity {
        item.quantity = quantity;
        item.unpaid_quantity = quantity; // Reset unpaid quantity
    }
    if let Some(discount) = changes.manual_discount_percent {
        item.manual_discount_percent = Some(discount);
    }
    if let Some(surcharge) = changes.surcharge {
        item.surcharge = Some(surcharge);
    }
    if let Some(ref note) = changes.note {
        item.note = Some(note.clone());
    }
    if let Some(ref options) = changes.selected_options {
        item.selected_options = Some(options.clone());
    }
    if let Some(ref specification) = changes.selected_specification {
        item.selected_specification = Some(specification.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::OrderEventType;

    fn create_test_item(
        instance_id: &str,
        product_id: &str,
        name: &str,
        price: f64,
        quantity: i32,
    ) -> CartItemSnapshot {
        CartItemSnapshot {
            id: product_id.to_string(),
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

    fn create_item_modified_event(
        order_id: &str,
        seq: u64,
        source: CartItemSnapshot,
        affected_quantity: i32,
        changes: ItemChanges,
        results: Vec<ItemModificationResult>,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::ItemModified,
            EventPayload::ItemModified {
                operation: "MODIFY_PRICE".to_string(),
                source: Box::new(source),
                affected_quantity,
                changes: Box::new(changes),
                previous_values: Box::new(ItemChanges::default()),
                results,
                authorizer_id: None,
                authorizer_name: None,
            },
        )
    }

    #[test]
    fn test_item_modified_full_price_change() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", "product:p1", "Product A", 10.0, 2));
        snapshot.subtotal = 20.0;
        snapshot.total = 20.0;

        let source = create_test_item("item-1", "product:p1", "Product A", 10.0, 2);
        let changes = ItemChanges {
            price: Some(15.0),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1".to_string(),
            quantity: 2,
            price: 15.0,
            manual_discount_percent: None,
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 2, source, 2, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].price, 15.0);
        assert_eq!(snapshot.items[0].quantity, 2);
        // 15.0 * 2 = 30.0
        assert_eq!(snapshot.subtotal, 30.0);
        assert_eq!(snapshot.total, 30.0);
        assert_eq!(snapshot.last_sequence, 2);
    }

    #[test]
    fn test_item_modified_full_discount() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", "product:p1", "Product A", 100.0, 1));
        snapshot.subtotal = 100.0;
        snapshot.total = 100.0;

        let source = create_test_item("item-1", "product:p1", "Product A", 100.0, 1);
        let changes = ItemChanges {
            manual_discount_percent: Some(20.0),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1".to_string(),
            quantity: 1,
            price: 100.0,
            manual_discount_percent: Some(20.0),
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 2, source, 1, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items[0].manual_discount_percent, Some(20.0));
        // 100.0 * 1 * (1 - 0.20) = 80.0
        assert_eq!(snapshot.subtotal, 80.0);
        assert_eq!(snapshot.total, 80.0);
    }

    #[test]
    fn test_item_modified_partial_split() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", "product:p1", "Product A", 10.0, 5));
        snapshot.subtotal = 50.0;
        snapshot.total = 50.0;

        let source = create_test_item("item-1", "product:p1", "Product A", 10.0, 5);
        let changes = ItemChanges {
            manual_discount_percent: Some(10.0),
            ..Default::default()
        };
        let results = vec![
            ItemModificationResult {
                instance_id: "item-1".to_string(),
                quantity: 3,
                price: 10.0,
                manual_discount_percent: None,
                action: "UNCHANGED".to_string(),
            },
            ItemModificationResult {
                instance_id: "item-2-split".to_string(),
                quantity: 2,
                price: 10.0,
                manual_discount_percent: Some(10.0),
                action: "CREATED".to_string(),
            },
        ];

        let event = create_item_modified_event("order-1", 2, source, 2, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        // Should have 2 items now
        assert_eq!(snapshot.items.len(), 2);

        // Original item with reduced quantity
        assert_eq!(snapshot.items[0].instance_id, "item-1");
        assert_eq!(snapshot.items[0].quantity, 3);
        assert_eq!(snapshot.items[0].manual_discount_percent, None);

        // New split item with discount
        assert_eq!(snapshot.items[1].instance_id, "item-2-split");
        assert_eq!(snapshot.items[1].quantity, 2);
        assert_eq!(snapshot.items[1].manual_discount_percent, Some(10.0));

        // Totals: 10.0 * 3 + 10.0 * 2 * 0.9 = 30.0 + 18.0 = 48.0
        assert!((snapshot.subtotal - 48.0).abs() < 0.001);
    }

    #[test]
    fn test_item_modified_surcharge() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", "product:p1", "Product A", 10.0, 1));

        let source = create_test_item("item-1", "product:p1", "Product A", 10.0, 1);
        let changes = ItemChanges {
            surcharge: Some(5.0),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1".to_string(),
            quantity: 1,
            price: 10.0,
            manual_discount_percent: None,
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 2, source, 1, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items[0].surcharge, Some(5.0));
    }

    #[test]
    fn test_item_modified_note() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", "product:p1", "Product A", 10.0, 1));

        let source = create_test_item("item-1", "product:p1", "Product A", 10.0, 1);
        let changes = ItemChanges {
            note: Some("Extra spicy".to_string()),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1".to_string(),
            quantity: 1,
            price: 10.0,
            manual_discount_percent: None,
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 2, source, 1, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items[0].note, Some("Extra spicy".to_string()));
    }

    #[test]
    fn test_item_modified_quantity_change() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", "product:p1", "Product A", 10.0, 3));
        snapshot.subtotal = 30.0;
        snapshot.total = 30.0;

        let source = create_test_item("item-1", "product:p1", "Product A", 10.0, 3);
        let changes = ItemChanges {
            quantity: Some(5),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1".to_string(),
            quantity: 5,
            price: 10.0,
            manual_discount_percent: None,
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 2, source, 3, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items[0].quantity, 5);
        assert_eq!(snapshot.items[0].unpaid_quantity, 5);
        assert_eq!(snapshot.subtotal, 50.0);
    }

    #[test]
    fn test_item_modified_updates_sequence() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", "product:p1", "Product A", 10.0, 1));
        snapshot.last_sequence = 5;

        let source = create_test_item("item-1", "product:p1", "Product A", 10.0, 1);
        let changes = ItemChanges {
            price: Some(15.0),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1".to_string(),
            quantity: 1,
            price: 15.0,
            manual_discount_percent: None,
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 6, source, 1, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 6);
    }

    #[test]
    fn test_item_modified_updates_checksum() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", "product:p1", "Product A", 10.0, 1));
        let initial_checksum = snapshot.state_checksum.clone();

        let source = create_test_item("item-1", "product:p1", "Product A", 10.0, 1);
        let changes = ItemChanges {
            price: Some(15.0),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1".to_string(),
            quantity: 1,
            price: 15.0,
            manual_discount_percent: None,
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 1, source, 1, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_item_modified_nonexistent_item_is_noop() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", "product:p1", "Product A", 10.0, 1));

        // Try to modify nonexistent item
        let source = create_test_item("nonexistent", "product:p1", "Product A", 10.0, 1);
        let changes = ItemChanges {
            price: Some(15.0),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "nonexistent".to_string(),
            quantity: 1,
            price: 15.0,
            manual_discount_percent: None,
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 1, source, 1, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        // Original item should be unchanged
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].price, 10.0);
    }

    #[test]
    fn test_item_modified_multiple_changes() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", "product:p1", "Product A", 10.0, 2));

        let source = create_test_item("item-1", "product:p1", "Product A", 10.0, 2);
        let changes = ItemChanges {
            price: Some(15.0),
            manual_discount_percent: Some(10.0),
            surcharge: Some(2.0),
            note: Some("Special order".to_string()),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1".to_string(),
            quantity: 2,
            price: 15.0,
            manual_discount_percent: Some(10.0),
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 1, source, 2, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items[0].price, 15.0);
        assert_eq!(snapshot.items[0].manual_discount_percent, Some(10.0));
        assert_eq!(snapshot.items[0].surcharge, Some(2.0));
        assert_eq!(snapshot.items[0].note, Some("Special order".to_string()));
        // unit_price = (15.0 * 0.9) + 2.0 surcharge = 15.5
        // subtotal = 15.5 * 2 = 31.0
        assert!((snapshot.subtotal - 31.0).abs() < 0.001);
    }

    #[test]
    fn test_item_modified_split_with_note() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", "product:p1", "Product A", 10.0, 3));

        let source = create_test_item("item-1", "product:p1", "Product A", 10.0, 3);
        let changes = ItemChanges {
            note: Some("Make it spicy".to_string()),
            ..Default::default()
        };
        let results = vec![
            ItemModificationResult {
                instance_id: "item-1".to_string(),
                quantity: 2,
                price: 10.0,
                manual_discount_percent: None,
                action: "UNCHANGED".to_string(),
            },
            ItemModificationResult {
                instance_id: "item-1-spicy".to_string(),
                quantity: 1,
                price: 10.0,
                manual_discount_percent: None,
                action: "CREATED".to_string(),
            },
        ];

        let event = create_item_modified_event("order-1", 1, source, 1, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items.len(), 2);
        assert_eq!(snapshot.items[0].quantity, 2);
        assert_eq!(snapshot.items[0].note, None);
        assert_eq!(snapshot.items[1].quantity, 1);
        assert_eq!(snapshot.items[1].note, Some("Make it spicy".to_string()));
    }

    #[test]
    fn test_apply_changes_to_item() {
        let mut item = create_test_item("item-1", "product:p1", "Product A", 10.0, 2);

        let changes = ItemChanges {
            price: Some(20.0),
            quantity: Some(5),
            manual_discount_percent: Some(15.0),
            surcharge: Some(3.0),
            note: Some("Test note".to_string()),
            selected_options: None,
            selected_specification: None,
        };

        apply_changes_to_item(&mut item, &changes);

        assert_eq!(item.price, 20.0);
        assert_eq!(item.quantity, 5);
        assert_eq!(item.unpaid_quantity, 5);
        assert_eq!(item.manual_discount_percent, Some(15.0));
        assert_eq!(item.surcharge, Some(3.0));
        assert_eq!(item.note, Some("Test note".to_string()));
    }

    #[test]
    fn test_apply_changes_partial() {
        let mut item = create_test_item("item-1", "product:p1", "Product A", 10.0, 2);
        item.manual_discount_percent = Some(5.0);
        item.surcharge = Some(1.0);
        item.note = Some("Original note".to_string());

        // Only change price
        let changes = ItemChanges {
            price: Some(20.0),
            ..Default::default()
        };

        apply_changes_to_item(&mut item, &changes);

        assert_eq!(item.price, 20.0);
        assert_eq!(item.quantity, 2); // Unchanged
        assert_eq!(item.manual_discount_percent, Some(5.0)); // Unchanged
        assert_eq!(item.surcharge, Some(1.0)); // Unchanged
        assert_eq!(item.note, Some("Original note".to_string())); // Unchanged
    }

    #[test]
    fn test_apply_changes_options() {
        let mut item = create_test_item("item-1", "product:p1", "Product A", 10.0, 1);
        assert!(item.selected_options.is_none());

        let new_options = vec![shared::order::ItemOption {
            attribute_id: "attribute:a1".to_string(),
            attribute_name: "Size".to_string(),
            option_idx: 1,
            option_name: "Large".to_string(),
            price_modifier: Some(2.0),
        }];

        let changes = ItemChanges {
            selected_options: Some(new_options),
            ..Default::default()
        };

        apply_changes_to_item(&mut item, &changes);

        assert!(item.selected_options.is_some());
        let options = item.selected_options.unwrap();
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].option_name, "Large");
        assert_eq!(options[0].price_modifier, Some(2.0));
    }

    #[test]
    fn test_apply_changes_specification() {
        let mut item = create_test_item("item-1", "product:p1", "Product A", 10.0, 1);
        assert!(item.selected_specification.is_none());

        let new_spec = shared::order::SpecificationInfo {
            id: "spec-1".to_string(),
            name: "Large".to_string(),
            external_id: None,
            receipt_name: Some("L".to_string()),
            price: Some(15.0),
        };

        let changes = ItemChanges {
            selected_specification: Some(new_spec),
            ..Default::default()
        };

        apply_changes_to_item(&mut item, &changes);

        assert!(item.selected_specification.is_some());
        let spec = item.selected_specification.unwrap();
        assert_eq!(spec.name, "Large");
        assert_eq!(spec.price, Some(15.0));
    }

    #[test]
    fn test_item_modified_options_full() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", "product:p1", "Product A", 10.0, 1));

        let source = create_test_item("item-1", "product:p1", "Product A", 10.0, 1);

        let new_options = vec![shared::order::ItemOption {
            attribute_id: "attribute:a1".to_string(),
            attribute_name: "Spicy".to_string(),
            option_idx: 2,
            option_name: "Extra Hot".to_string(),
            price_modifier: None,
        }];

        let changes = ItemChanges {
            selected_options: Some(new_options),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1".to_string(),
            quantity: 1,
            price: 10.0,
            manual_discount_percent: None,
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 1, source, 1, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        assert!(snapshot.items[0].selected_options.is_some());
        let options = snapshot.items[0].selected_options.as_ref().unwrap();
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].option_name, "Extra Hot");
    }

    #[test]
    fn test_item_modified_split_with_options() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", "product:p1", "Product A", 10.0, 3));

        let source = create_test_item("item-1", "product:p1", "Product A", 10.0, 3);

        let new_options = vec![shared::order::ItemOption {
            attribute_id: "attribute:a1".to_string(),
            attribute_name: "Spicy".to_string(),
            option_idx: 1,
            option_name: "Mild".to_string(),
            price_modifier: None,
        }];

        let changes = ItemChanges {
            selected_options: Some(new_options.clone()),
            ..Default::default()
        };

        let results = vec![
            ItemModificationResult {
                instance_id: "item-1".to_string(),
                quantity: 2,
                price: 10.0,
                manual_discount_percent: None,
                action: "UNCHANGED".to_string(),
            },
            ItemModificationResult {
                instance_id: "item-1-spicy".to_string(),
                quantity: 1,
                price: 10.0,
                manual_discount_percent: None,
                action: "CREATED".to_string(),
            },
        ];

        let event = create_item_modified_event("order-1", 1, source, 1, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        // Original item unchanged
        assert_eq!(snapshot.items.len(), 2);
        assert_eq!(snapshot.items[0].quantity, 2);
        assert!(snapshot.items[0].selected_options.is_none());

        // New item has options
        assert_eq!(snapshot.items[1].quantity, 1);
        assert!(snapshot.items[1].selected_options.is_some());
        let options = snapshot.items[1].selected_options.as_ref().unwrap();
        assert_eq!(options[0].option_name, "Mild");
    }

    /// Test that full modification updates instance_id
    ///
    /// Scenario:
    /// 1. Add item with instance_id "item-1"
    /// 2. Apply discount → instance_id should change to "item-1-new"
    /// 3. Verify the item's instance_id is updated
    #[test]
    fn test_item_modified_full_updates_instance_id() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", "product:p1", "Product A", 100.0, 2));

        let source = create_test_item("item-1", "product:p1", "Product A", 100.0, 2);
        let changes = ItemChanges {
            manual_discount_percent: Some(10.0),
            ..Default::default()
        };
        // The result now contains a NEW instance_id (simulating what modify_item.rs generates)
        let results = vec![ItemModificationResult {
            instance_id: "item-1-discounted".to_string(), // New instance_id
            quantity: 2,
            price: 100.0,
            manual_discount_percent: Some(10.0),
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 2, source, 2, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        // instance_id should be updated
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "item-1-discounted");
        assert_eq!(snapshot.items[0].manual_discount_percent, Some(10.0));
    }

    /// Test that after full modification, adding the same product creates a separate item
    ///
    /// This is an integration-style test demonstrating the fix for the merge bug:
    /// 1. Add item A (instance_id based on product + price)
    /// 2. Modify item A with discount (instance_id changes)
    /// 3. Add item A again (instance_id = original, different from modified)
    /// 4. Should have 2 separate items (not merged)
    #[test]
    fn test_modified_item_does_not_merge_with_new_item() {
        use crate::orders::appliers::items_added::ItemsAddedApplier;

        let mut snapshot = OrderSnapshot::new("order-1".to_string());

        // Step 1: Add initial item
        let initial_item = create_test_item("original-id", "product:p1", "Product A", 100.0, 1);
        snapshot.items.push(initial_item.clone());

        // Step 2: Modify item with discount (instance_id changes)
        let changes = ItemChanges {
            manual_discount_percent: Some(20.0),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "discounted-id".to_string(), // NEW instance_id
            quantity: 1,
            price: 100.0,
            manual_discount_percent: Some(20.0),
            action: "UPDATED".to_string(),
        }];

        let modify_event = create_item_modified_event("order-1", 2, initial_item, 1, changes, results);

        let modify_applier = ItemModifiedApplier;
        modify_applier.apply(&mut snapshot, &modify_event);

        // Verify instance_id changed
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "discounted-id");
        assert_eq!(snapshot.items[0].manual_discount_percent, Some(20.0));

        // Step 3: Add same product again (with original instance_id, no discount)
        let new_item = create_test_item("original-id", "product:p1", "Product A", 100.0, 1);
        let items_added_event = OrderEvent::new(
            3,
            "order-1".to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            "cmd-3".to_string(),
            Some(1234567890),
            OrderEventType::ItemsAdded,
            EventPayload::ItemsAdded {
                items: vec![new_item],
            },
        );

        let add_applier = ItemsAddedApplier;
        add_applier.apply(&mut snapshot, &items_added_event);

        // Step 4: Should have 2 separate items
        assert_eq!(snapshot.items.len(), 2);

        // Item 1: discounted (instance_id = "discounted-id")
        assert_eq!(snapshot.items[0].instance_id, "discounted-id");
        assert_eq!(snapshot.items[0].manual_discount_percent, Some(20.0));
        assert_eq!(snapshot.items[0].quantity, 1);

        // Item 2: new item without discount (instance_id = "original-id")
        assert_eq!(snapshot.items[1].instance_id, "original-id");
        assert_eq!(snapshot.items[1].manual_discount_percent, None);
        assert_eq!(snapshot.items[1].quantity, 1);
    }
}
