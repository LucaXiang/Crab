//! ItemModified event applier
//!
//! Applies the ItemModified event to update items in the snapshot.
//! Handles both full modifications and split scenarios.

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

            // Recalculate totals
            recalculate_totals(snapshot);

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

        if affected_quantity >= original_qty {
            // Full modification: update entire item in place
            apply_changes_to_item(&mut snapshot.items[idx], changes);
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
                    new_item.discount_percent = result.discount_percent;

                    // Apply additional changes (note, surcharge)
                    if let Some(ref note) = changes.note {
                        new_item.note = Some(note.clone());
                    }
                    if let Some(surcharge) = changes.surcharge {
                        new_item.surcharge = Some(surcharge);
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
    if let Some(discount) = changes.discount_percent {
        item.discount_percent = Some(discount);
    }
    if let Some(surcharge) = changes.surcharge {
        item.surcharge = Some(surcharge);
    }
    if let Some(ref note) = changes.note {
        item.note = Some(note.clone());
    }
}

/// Recalculate totals from items
fn recalculate_totals(snapshot: &mut OrderSnapshot) {
    let subtotal: f64 = snapshot
        .items
        .iter_mut()
        .map(|item| {
            // Compute unpaid_quantity: quantity - paid_quantity
            let paid_qty = snapshot
                .paid_item_quantities
                .get(&item.instance_id)
                .copied()
                .unwrap_or(0);
            item.unpaid_quantity = (item.quantity - paid_qty).max(0);

            let base_price = item.price * item.quantity as f64;
            let discount = item.discount_percent.unwrap_or(0.0) / 100.0;
            base_price * (1.0 - discount)
        })
        .sum();

    snapshot.subtotal = subtotal;
    snapshot.total = subtotal; // For now, total = subtotal (no tax/service charge)
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
            discount_percent: None,
            surcharge: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
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
                changes,
                previous_values: ItemChanges::default(),
                results,
                authorizer_id: None,
                authorizer_name: None,
            },
        )
    }

    #[test]
    fn test_item_modified_full_price_change() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(create_test_item(
            "item-1",
            "prod-1",
            "Product A",
            10.0,
            2,
        ));
        snapshot.subtotal = 20.0;
        snapshot.total = 20.0;

        let source = create_test_item("item-1", "prod-1", "Product A", 10.0, 2);
        let changes = ItemChanges {
            price: Some(15.0),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1".to_string(),
            quantity: 2,
            price: 15.0,
            discount_percent: None,
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
        snapshot.items.push(create_test_item(
            "item-1",
            "prod-1",
            "Product A",
            100.0,
            1,
        ));
        snapshot.subtotal = 100.0;
        snapshot.total = 100.0;

        let source = create_test_item("item-1", "prod-1", "Product A", 100.0, 1);
        let changes = ItemChanges {
            discount_percent: Some(20.0),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1".to_string(),
            quantity: 1,
            price: 100.0,
            discount_percent: Some(20.0),
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 2, source, 1, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items[0].discount_percent, Some(20.0));
        // 100.0 * 1 * (1 - 0.20) = 80.0
        assert_eq!(snapshot.subtotal, 80.0);
        assert_eq!(snapshot.total, 80.0);
    }

    #[test]
    fn test_item_modified_partial_split() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(create_test_item(
            "item-1",
            "prod-1",
            "Product A",
            10.0,
            5,
        ));
        snapshot.subtotal = 50.0;
        snapshot.total = 50.0;

        let source = create_test_item("item-1", "prod-1", "Product A", 10.0, 5);
        let changes = ItemChanges {
            discount_percent: Some(10.0),
            ..Default::default()
        };
        let results = vec![
            ItemModificationResult {
                instance_id: "item-1".to_string(),
                quantity: 3,
                price: 10.0,
                discount_percent: None,
                action: "UNCHANGED".to_string(),
            },
            ItemModificationResult {
                instance_id: "item-2-split".to_string(),
                quantity: 2,
                price: 10.0,
                discount_percent: Some(10.0),
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
        assert_eq!(snapshot.items[0].discount_percent, None);

        // New split item with discount
        assert_eq!(snapshot.items[1].instance_id, "item-2-split");
        assert_eq!(snapshot.items[1].quantity, 2);
        assert_eq!(snapshot.items[1].discount_percent, Some(10.0));

        // Totals: 10.0 * 3 + 10.0 * 2 * 0.9 = 30.0 + 18.0 = 48.0
        assert!((snapshot.subtotal - 48.0).abs() < 0.001);
    }

    #[test]
    fn test_item_modified_surcharge() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(create_test_item(
            "item-1",
            "prod-1",
            "Product A",
            10.0,
            1,
        ));

        let source = create_test_item("item-1", "prod-1", "Product A", 10.0, 1);
        let changes = ItemChanges {
            surcharge: Some(5.0),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1".to_string(),
            quantity: 1,
            price: 10.0,
            discount_percent: None,
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
        snapshot.items.push(create_test_item(
            "item-1",
            "prod-1",
            "Product A",
            10.0,
            1,
        ));

        let source = create_test_item("item-1", "prod-1", "Product A", 10.0, 1);
        let changes = ItemChanges {
            note: Some("Extra spicy".to_string()),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1".to_string(),
            quantity: 1,
            price: 10.0,
            discount_percent: None,
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
        snapshot.items.push(create_test_item(
            "item-1",
            "prod-1",
            "Product A",
            10.0,
            3,
        ));
        snapshot.subtotal = 30.0;
        snapshot.total = 30.0;

        let source = create_test_item("item-1", "prod-1", "Product A", 10.0, 3);
        let changes = ItemChanges {
            quantity: Some(5),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1".to_string(),
            quantity: 5,
            price: 10.0,
            discount_percent: None,
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
        snapshot.items.push(create_test_item(
            "item-1",
            "prod-1",
            "Product A",
            10.0,
            1,
        ));
        snapshot.last_sequence = 5;

        let source = create_test_item("item-1", "prod-1", "Product A", 10.0, 1);
        let changes = ItemChanges {
            price: Some(15.0),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1".to_string(),
            quantity: 1,
            price: 15.0,
            discount_percent: None,
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
        snapshot.items.push(create_test_item(
            "item-1",
            "prod-1",
            "Product A",
            10.0,
            1,
        ));
        let initial_checksum = snapshot.state_checksum.clone();

        let source = create_test_item("item-1", "prod-1", "Product A", 10.0, 1);
        let changes = ItemChanges {
            price: Some(15.0),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1".to_string(),
            quantity: 1,
            price: 15.0,
            discount_percent: None,
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
        snapshot.items.push(create_test_item(
            "item-1",
            "prod-1",
            "Product A",
            10.0,
            1,
        ));

        // Try to modify nonexistent item
        let source = create_test_item("nonexistent", "prod-1", "Product A", 10.0, 1);
        let changes = ItemChanges {
            price: Some(15.0),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "nonexistent".to_string(),
            quantity: 1,
            price: 15.0,
            discount_percent: None,
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
        snapshot.items.push(create_test_item(
            "item-1",
            "prod-1",
            "Product A",
            10.0,
            2,
        ));

        let source = create_test_item("item-1", "prod-1", "Product A", 10.0, 2);
        let changes = ItemChanges {
            price: Some(15.0),
            discount_percent: Some(10.0),
            surcharge: Some(2.0),
            note: Some("Special order".to_string()),
            ..Default::default()
        };
        let results = vec![ItemModificationResult {
            instance_id: "item-1".to_string(),
            quantity: 2,
            price: 15.0,
            discount_percent: Some(10.0),
            action: "UPDATED".to_string(),
        }];

        let event = create_item_modified_event("order-1", 1, source, 2, changes, results);

        let applier = ItemModifiedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items[0].price, 15.0);
        assert_eq!(snapshot.items[0].discount_percent, Some(10.0));
        assert_eq!(snapshot.items[0].surcharge, Some(2.0));
        assert_eq!(snapshot.items[0].note, Some("Special order".to_string()));
        // 15.0 * 2 * 0.9 = 27.0
        assert!((snapshot.subtotal - 27.0).abs() < 0.001);
    }

    #[test]
    fn test_item_modified_split_with_note() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(create_test_item(
            "item-1",
            "prod-1",
            "Product A",
            10.0,
            3,
        ));

        let source = create_test_item("item-1", "prod-1", "Product A", 10.0, 3);
        let changes = ItemChanges {
            note: Some("Make it spicy".to_string()),
            ..Default::default()
        };
        let results = vec![
            ItemModificationResult {
                instance_id: "item-1".to_string(),
                quantity: 2,
                price: 10.0,
                discount_percent: None,
                action: "UNCHANGED".to_string(),
            },
            ItemModificationResult {
                instance_id: "item-1-spicy".to_string(),
                quantity: 1,
                price: 10.0,
                discount_percent: None,
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
        let mut item = create_test_item("item-1", "prod-1", "Product A", 10.0, 2);

        let changes = ItemChanges {
            price: Some(20.0),
            quantity: Some(5),
            discount_percent: Some(15.0),
            surcharge: Some(3.0),
            note: Some("Test note".to_string()),
        };

        apply_changes_to_item(&mut item, &changes);

        assert_eq!(item.price, 20.0);
        assert_eq!(item.quantity, 5);
        assert_eq!(item.unpaid_quantity, 5);
        assert_eq!(item.discount_percent, Some(15.0));
        assert_eq!(item.surcharge, Some(3.0));
        assert_eq!(item.note, Some("Test note".to_string()));
    }

    #[test]
    fn test_apply_changes_partial() {
        let mut item = create_test_item("item-1", "prod-1", "Product A", 10.0, 2);
        item.discount_percent = Some(5.0);
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
        assert_eq!(item.discount_percent, Some(5.0)); // Unchanged
        assert_eq!(item.surcharge, Some(1.0)); // Unchanged
        assert_eq!(item.note, Some("Original note".to_string())); // Unchanged
    }
}
