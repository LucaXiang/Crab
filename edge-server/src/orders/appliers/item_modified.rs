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

            // Merge items with duplicate instance_id that have no paid quantities.
            // This prevents fragmentation from split-modify cycles (e.g., discount
            // cycling + payment cancellation leaves multiple items with the same
            // content-addressed instance_id).
            merge_duplicate_unpaid_items(snapshot);

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
            let has_price_change =
                changes.price.is_some() || changes.manual_discount_percent.is_some();
            let unpaid_qty = original_qty - paid_qty;

            if has_price_change && unpaid_qty > 0 {
                // Price/discount change on partially paid item — split to protect paid portion.
                // Similar to how item_comped splits: paid portion keeps original price/discount,
                // unpaid portion gets the new changes applied.

                // 1. Shrink original item to paid-only portion (frozen at original price)
                snapshot.items[idx].quantity = paid_qty;
                snapshot.items[idx].unpaid_quantity = 0;
                // paid_item_quantities stays with original instance_id

                // 2. Create new item for unpaid portion with changes applied
                let mut new_item = source.clone();
                if let Some(result) = results.iter().find(|r| r.action == "UPDATED") {
                    new_item.instance_id = result.instance_id.clone();
                }
                let new_unpaid = changes.quantity.unwrap_or(unpaid_qty);
                new_item.quantity = new_unpaid;
                new_item.unpaid_quantity = new_unpaid;
                if let Some(price) = changes.price {
                    new_item.price = price;
                    new_item.original_price = price;
                }
                if let Some(discount) = changes.manual_discount_percent {
                    new_item.manual_discount_percent = if discount.abs() < 0.01 { None } else { Some(discount) };
                }
                if let Some(ref note) = changes.note {
                    new_item.note = Some(note.clone());
                }
                if let Some(ref options) = changes.selected_options {
                    new_item.selected_options = Some(options.clone());
                }
                if let Some(ref spec) = changes.selected_specification {
                    new_item.selected_specification = Some(spec.clone());
                }

                snapshot.items.push(new_item);
            } else {
                // Non-price change (quantity, note, options, spec) — safe to update in place
                if let Some(new_unpaid) = changes.quantity {
                    snapshot.items[idx].quantity = paid_qty + new_unpaid;
                }

                apply_changes_to_item_skip_quantity(&mut snapshot.items[idx], changes);

                // Migrate paid_item_quantities key to new instance_id
                if let Some(result) = results.iter().find(|r| r.action == "UPDATED") {
                    snapshot.items[idx].instance_id = result.instance_id.clone();
                    snapshot
                        .paid_item_quantities
                        .remove(&source.instance_id);
                    snapshot
                        .paid_item_quantities
                        .insert(result.instance_id.clone(), paid_qty);
                }
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
                    new_item.original_price = result.price;
                    new_item.manual_discount_percent = result.manual_discount_percent;

                    // Apply additional changes (note, options, specification)
                    if let Some(ref note) = changes.note {
                        new_item.note = Some(note.clone());
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

/// Merge items with duplicate instance_id when neither has paid quantities.
///
/// After modification, a content-addressed instance_id may match an existing item
/// (e.g., discount cycling: 50%→20%→50% → cancel payments → remove all discounts
/// results in multiple fragments with identical instance_id).
///
/// Having duplicate instance_ids is a correctness hazard: `paid_item_quantities`
/// is keyed by instance_id, so a single paid entry would incorrectly subtract
/// from ALL duplicates in `recalculate_totals`.
fn merge_duplicate_unpaid_items(snapshot: &mut OrderSnapshot) {
    let mut i = 0;
    while i < snapshot.items.len() {
        let instance_id = snapshot.items[i].instance_id.clone();

        // Skip if this instance_id has paid quantities — merging paid items is unsafe
        if snapshot
            .paid_item_quantities
            .get(&instance_id)
            .copied()
            .unwrap_or(0)
            > 0
        {
            i += 1;
            continue;
        }

        // Absorb all later duplicates with the same instance_id
        let mut j = i + 1;
        while j < snapshot.items.len() {
            if snapshot.items[j].instance_id == instance_id {
                snapshot.items[i].quantity += snapshot.items[j].quantity;
                snapshot.items[i].unpaid_quantity += snapshot.items[j].unpaid_quantity;
                snapshot.items.remove(j);
            } else {
                j += 1;
            }
        }
        i += 1;
    }
}

/// Apply changes to a single item, skipping quantity (used in split-bill path
/// where quantity is already calculated correctly as unpaid_qty).
fn apply_changes_to_item_skip_quantity(item: &mut CartItemSnapshot, changes: &ItemChanges) {
    if let Some(price) = changes.price {
        item.price = price;
        item.original_price = price;
    }
    // Skip quantity — already set correctly by the caller
    if let Some(discount) = changes.manual_discount_percent {
        // 0% discount ≡ no discount — normalize to None so instance_id stays consistent
        item.manual_discount_percent = if discount.abs() < 0.01 { None } else { Some(discount) };
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

/// Apply changes to a single item
fn apply_changes_to_item(item: &mut CartItemSnapshot, changes: &ItemChanges) {
    if let Some(price) = changes.price {
        item.price = price;
        item.original_price = price;
    }
    if let Some(quantity) = changes.quantity {
        item.quantity = quantity;
        item.unpaid_quantity = quantity; // Reset unpaid quantity
    }
    if let Some(discount) = changes.manual_discount_percent {
        item.manual_discount_percent = if discount.abs() < 0.01 { None } else { Some(discount) };
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
            unit_price: 0.0,
            line_total: 0.0,
            tax: 0.0,
            tax_rate: 0,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
        is_comped: false,
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
            1,
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
            .push(create_test_item("item-1", 1, "Product A", 10.0, 2));
        snapshot.subtotal = 20.0;
        snapshot.total = 20.0;

        let source = create_test_item("item-1", 1, "Product A", 10.0, 2);
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
            .push(create_test_item("item-1", 1, "Product A", 100.0, 1));
        snapshot.subtotal = 100.0;
        snapshot.total = 100.0;

        let source = create_test_item("item-1", 1, "Product A", 100.0, 1);
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
            .push(create_test_item("item-1", 1, "Product A", 10.0, 5));
        snapshot.subtotal = 50.0;
        snapshot.total = 50.0;

        let source = create_test_item("item-1", 1, "Product A", 10.0, 5);
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
    fn test_item_modified_note() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 1));

        let source = create_test_item("item-1", 1, "Product A", 10.0, 1);
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
            .push(create_test_item("item-1", 1, "Product A", 10.0, 3));
        snapshot.subtotal = 30.0;
        snapshot.total = 30.0;

        let source = create_test_item("item-1", 1, "Product A", 10.0, 3);
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
            .push(create_test_item("item-1", 1, "Product A", 10.0, 1));
        snapshot.last_sequence = 5;

        let source = create_test_item("item-1", 1, "Product A", 10.0, 1);
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
            .push(create_test_item("item-1", 1, "Product A", 10.0, 1));
        let initial_checksum = snapshot.state_checksum.clone();

        let source = create_test_item("item-1", 1, "Product A", 10.0, 1);
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
            .push(create_test_item("item-1", 1, "Product A", 10.0, 1));

        // Try to modify nonexistent item
        let source = create_test_item("nonexistent", 1, "Product A", 10.0, 1);
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
            .push(create_test_item("item-1", 1, "Product A", 10.0, 2));

        let source = create_test_item("item-1", 1, "Product A", 10.0, 2);
        let changes = ItemChanges {
            price: Some(15.0),
            manual_discount_percent: Some(10.0),
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

        // item.price is synced to computed unit_price by recalculate_totals
        // base=15.0 * (1 - 10%) = 13.5
        assert_eq!(snapshot.items[0].price, 13.5);
        assert_eq!(snapshot.items[0].original_price, 15.0);
        assert_eq!(snapshot.items[0].manual_discount_percent, Some(10.0));
        assert_eq!(snapshot.items[0].note, Some("Special order".to_string()));
        // subtotal = 13.5 * 2 = 27.0
        assert!((snapshot.subtotal - 27.0).abs() < 0.001);
    }

    #[test]
    fn test_item_modified_split_with_note() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 3));

        let source = create_test_item("item-1", 1, "Product A", 10.0, 3);
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
        let mut item = create_test_item("item-1", 1, "Product A", 10.0, 2);

        let changes = ItemChanges {
            price: Some(20.0),
            quantity: Some(5),
            manual_discount_percent: Some(15.0),
            note: Some("Test note".to_string()),
            selected_options: None,
            selected_specification: None,
        };

        apply_changes_to_item(&mut item, &changes);

        assert_eq!(item.price, 20.0);
        assert_eq!(item.quantity, 5);
        assert_eq!(item.unpaid_quantity, 5);
        assert_eq!(item.manual_discount_percent, Some(15.0));
        assert_eq!(item.note, Some("Test note".to_string()));
    }

    #[test]
    fn test_apply_changes_partial() {
        let mut item = create_test_item("item-1", 1, "Product A", 10.0, 2);
        item.manual_discount_percent = Some(5.0);
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
        assert_eq!(item.note, Some("Original note".to_string())); // Unchanged
    }

    #[test]
    fn test_apply_changes_options() {
        let mut item = create_test_item("item-1", 1, "Product A", 10.0, 1);
        assert!(item.selected_options.is_none());

        let new_options = vec![shared::order::ItemOption {
            attribute_id: 1,
            attribute_name: "Size".to_string(),
            option_id: 1,
            option_name: "Large".to_string(),
            price_modifier: Some(2.0),
            quantity: 1,
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
        let mut item = create_test_item("item-1", 1, "Product A", 10.0, 1);
        assert!(item.selected_specification.is_none());

        let new_spec = shared::order::SpecificationInfo {
            id: 1,
            name: "Large".to_string(),
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
            .push(create_test_item("item-1", 1, "Product A", 10.0, 1));

        let source = create_test_item("item-1", 1, "Product A", 10.0, 1);

        let new_options = vec![shared::order::ItemOption {
            attribute_id: 1,
            attribute_name: "Spicy".to_string(),
            option_id: 2,
            option_name: "Extra Hot".to_string(),
            price_modifier: None,
            quantity: 1,
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
            .push(create_test_item("item-1", 1, "Product A", 10.0, 3));

        let source = create_test_item("item-1", 1, "Product A", 10.0, 3);

        let new_options = vec![shared::order::ItemOption {
            attribute_id: 1,
            attribute_name: "Spicy".to_string(),
            option_id: 1,
            option_name: "Mild".to_string(),
            price_modifier: None,
            quantity: 1,
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
            .push(create_test_item("item-1", 1, "Product A", 100.0, 2));

        let source = create_test_item("item-1", 1, "Product A", 100.0, 2);
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
        let initial_item = create_test_item("original-id", 1, "Product A", 100.0, 1);
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
        let new_item = create_test_item("original-id", 1, "Product A", 100.0, 1);
        let items_added_event = OrderEvent::new(
            3,
            "order-1".to_string(),
            1,
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

    /// Test: after partial payment, modifying quantity updates in place (no split).
    ///
    /// Scenario: item x6, paid x1 → unpaid x5.
    /// User changes unpaid to 7. Expected: total=8, unpaid=7, single item.
    #[test]
    fn test_item_modified_paid_quantity_change_in_place() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 6));
        snapshot.paid_item_quantities.insert("item-1".to_string(), 1);

        let source = create_test_item("item-1", 1, "Product A", 10.0, 6);
        let changes = ItemChanges {
            quantity: Some(7), // new unpaid quantity
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1-updated".to_string(),
            quantity: 8,
            price: 10.0,
            manual_discount_percent: None,
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 2, source, 6, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        // Single item, no splitting
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "item-1-updated");
        assert_eq!(snapshot.items[0].quantity, 8);  // paid(1) + unpaid(7)
        assert_eq!(snapshot.items[0].unpaid_quantity, 7);

        // paid_item_quantities key migrated to new instance_id
        assert_eq!(snapshot.paid_item_quantities.get("item-1"), None);
        assert_eq!(snapshot.paid_item_quantities.get("item-1-updated"), Some(&1));
    }

    /// Test: after partial payment, modifying note without quantity stays in place.
    ///
    /// Scenario: item x6, paid x1. User changes note only.
    /// Expected: single item, quantities unchanged.
    #[test]
    fn test_item_modified_paid_note_only_in_place() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 6));
        snapshot.paid_item_quantities.insert("item-1".to_string(), 1);

        let source = create_test_item("item-1", 1, "Product A", 10.0, 6);
        let changes = ItemChanges {
            note: Some("Extra spicy".to_string()),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1-updated".to_string(),
            quantity: 6,
            price: 10.0,
            manual_discount_percent: None,
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 2, source, 6, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        // Single item, no splitting
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "item-1-updated");
        assert_eq!(snapshot.items[0].quantity, 6);  // unchanged
        assert_eq!(snapshot.items[0].unpaid_quantity, 5);  // 6 - paid(1) = 5
        assert_eq!(snapshot.items[0].note, Some("Extra spicy".to_string()));
    }

    /// BUG FIX: applying discount to partially paid item must split, not in-place.
    ///
    /// Scenario: item x10 @ 5€, paid x9 → unpaid x1.
    /// User applies 50% discount. Expected: paid portion frozen at 5€, unpaid at 2.5€.
    #[test]
    fn test_item_modified_paid_discount_splits_to_protect_paid() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 5.0, 10));
        snapshot.paid_item_quantities.insert("item-1".to_string(), 9);
        snapshot.paid_amount = 45.0;

        let source = create_test_item("item-1", 1, "Product A", 5.0, 10);
        let changes = ItemChanges {
            manual_discount_percent: Some(50.0),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1-discounted".to_string(),
            quantity: 10,
            price: 5.0,
            manual_discount_percent: Some(50.0),
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 2, source, 10, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        // Should have 2 items: paid portion + unpaid portion
        assert_eq!(snapshot.items.len(), 2);

        // Item 0: paid portion (frozen at original price, no discount)
        assert_eq!(snapshot.items[0].instance_id, "item-1");
        assert_eq!(snapshot.items[0].quantity, 9);
        assert_eq!(snapshot.items[0].unpaid_quantity, 0);
        assert_eq!(snapshot.items[0].price, 5.0);
        assert_eq!(snapshot.items[0].manual_discount_percent, None);

        // Item 1: unpaid portion (discount applied)
        // item.price synced to unit_price: 5.0 * 0.5 = 2.5
        assert_eq!(snapshot.items[1].instance_id, "item-1-discounted");
        assert_eq!(snapshot.items[1].quantity, 1);
        assert_eq!(snapshot.items[1].unpaid_quantity, 1);
        assert_eq!(snapshot.items[1].price, 2.5);
        assert_eq!(snapshot.items[1].manual_discount_percent, Some(50.0));

        // paid_item_quantities stays with original instance_id (paid portion)
        assert_eq!(snapshot.paid_item_quantities.get("item-1"), Some(&9));
        assert_eq!(snapshot.paid_item_quantities.get("item-1-discounted"), None);

        // Totals: paid(9×5=45) + unpaid(1×2.5=2.5) = 47.5
        assert!((snapshot.subtotal - 47.5).abs() < 0.01);
        assert!((snapshot.total - 47.5).abs() < 0.01);
        // remaining = 47.5 - 45 = 2.5
        assert!((snapshot.remaining_amount - 2.5).abs() < 0.01);
    }

    /// Test: discount + quantity change on partially paid item.
    ///
    /// Scenario: item x10 @ 5€, paid x9 → unpaid x1.
    /// User applies 50% discount AND changes unpaid to 10.
    /// Expected: paid(9@5€) + unpaid(10@2.5€=25€). Total=70, remaining=25.
    #[test]
    fn test_item_modified_paid_discount_with_quantity_change() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 5.0, 10));
        snapshot.paid_item_quantities.insert("item-1".to_string(), 9);
        snapshot.paid_amount = 45.0;

        let source = create_test_item("item-1", 1, "Product A", 5.0, 10);
        let changes = ItemChanges {
            manual_discount_percent: Some(50.0),
            quantity: Some(10), // new unpaid qty
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1-discounted".to_string(),
            quantity: 10,
            price: 5.0,
            manual_discount_percent: Some(50.0),
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 2, source, 10, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        // 2 items: paid(9) + unpaid(10)
        assert_eq!(snapshot.items.len(), 2);

        // Paid portion frozen
        assert_eq!(snapshot.items[0].quantity, 9);
        assert_eq!(snapshot.items[0].price, 5.0);
        assert_eq!(snapshot.items[0].manual_discount_percent, None);

        // Unpaid portion with discount and new quantity
        assert_eq!(snapshot.items[1].quantity, 10);
        assert_eq!(snapshot.items[1].manual_discount_percent, Some(50.0));

        // Total: 9*5 + 10*2.5 = 45 + 25 = 70
        assert!((snapshot.subtotal - 70.0).abs() < 0.01);
        // Remaining: 70 - 45 = 25
        assert!((snapshot.remaining_amount - 25.0).abs() < 0.01);
    }

    /// Test: price change on partially paid item also splits.
    #[test]
    fn test_item_modified_paid_price_change_splits() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 5));
        snapshot.paid_item_quantities.insert("item-1".to_string(), 3);
        snapshot.paid_amount = 30.0;

        let source = create_test_item("item-1", 1, "Product A", 10.0, 5);
        let changes = ItemChanges {
            price: Some(8.0), // reduce price
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1-repriced".to_string(),
            quantity: 5,
            price: 8.0,
            manual_discount_percent: None,
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 2, source, 5, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        // Split: paid(3@10) + unpaid(2@8)
        assert_eq!(snapshot.items.len(), 2);
        assert_eq!(snapshot.items[0].quantity, 3);
        assert_eq!(snapshot.items[0].price, 10.0);
        assert_eq!(snapshot.items[1].quantity, 2);
        assert_eq!(snapshot.items[1].price, 8.0);

        // Total: 30 + 16 = 46, remaining: 46 - 30 = 16
        assert!((snapshot.subtotal - 46.0).abs() < 0.01);
        assert!((snapshot.remaining_amount - 16.0).abs() < 0.01);
    }
}
