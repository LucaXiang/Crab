//! ItemRemoved event applier
//!
//! Applies the ItemRemoved event to remove or reduce item quantity in the snapshot.
//! Handles both full removal and partial removal.
//!
//! Note: In the current implementation, items are physically removed from the
//! snapshot (or their quantity is reduced). For full audit trail support,
//! a future enhancement could mark items as "voided" instead.

use crate::order_money;
use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

/// ItemRemoved applier
pub struct ItemRemovedApplier;

impl EventApplier for ItemRemovedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::ItemRemoved {
            instance_id,
            quantity,
            ..
        } = &event.payload
        {
            apply_item_removed(snapshot, instance_id, *quantity);

            // Update sequence and timestamp
            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;

            // Recalculate totals using precise decimal arithmetic
            order_money::recalculate_totals(snapshot);

            // Update checksum
            snapshot.update_checksum();
        }
    }
}

/// Apply item removal to snapshot
///
/// Removes only the FIRST matching item by index, not all items with the same
/// instance_id. This is important because duplicate instance_ids can exist
/// transiently (e.g., after discount cycling + payment cancellation).
fn apply_item_removed(snapshot: &mut OrderSnapshot, instance_id: &str, quantity: Option<i32>) {
    let Some(idx) = snapshot
        .items
        .iter()
        .position(|i| i.instance_id == instance_id)
    else {
        return;
    };

    if let Some(qty) = quantity {
        // Partial removal: reduce quantity
        snapshot.items[idx].quantity = (snapshot.items[idx].quantity - qty).max(0);
        snapshot.items[idx].unpaid_quantity = (snapshot.items[idx].unpaid_quantity - qty).max(0);

        // If quantity reaches 0, remove this specific item
        if snapshot.items[idx].quantity == 0 {
            snapshot.items.remove(idx);
        }
    } else {
        // Full removal: remove this specific item (not all with same instance_id)
        snapshot.items.remove(idx);
    }

    // Clean up paid_item_quantities if no items with this instance_id remain
    if !snapshot.items.iter().any(|i| i.instance_id == instance_id) {
        snapshot.paid_item_quantities.remove(instance_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order_money::recalculate_totals;
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

    fn create_item_removed_event(
        order_id: &str,
        seq: u64,
        instance_id: &str,
        item_name: &str,
        quantity: Option<i32>,
        reason: Option<String>,
    ) -> OrderEvent {
        // Use struct initialization directly to set a known timestamp for testing
        OrderEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            sequence: seq,
            order_id: order_id.to_string(),
            timestamp: 1234567890,
            client_timestamp: Some(1234567890),
            operator_id: 1,
            operator_name: "Test User".to_string(),
            command_id: "cmd-1".to_string(),
            event_type: OrderEventType::ItemRemoved,
            payload: EventPayload::ItemRemoved {
                instance_id: instance_id.to_string(),
                item_name: item_name.to_string(),
                quantity,
                reason,
                authorizer_id: None,
                authorizer_name: None,
            },
        }
    }

    #[test]
    fn test_item_removed_full() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 2));
        snapshot.subtotal = 20.0;
        snapshot.total = 20.0;

        let event = create_item_removed_event("order-1", 2, "item-1", "Product A", None, None);

        let applier = ItemRemovedApplier;
        applier.apply(&mut snapshot, &event);

        // Item should be removed
        assert_eq!(snapshot.items.len(), 0);
        assert_eq!(snapshot.subtotal, 0.0);
        assert_eq!(snapshot.total, 0.0);
        assert_eq!(snapshot.last_sequence, 2);
    }

    #[test]
    fn test_item_removed_partial() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 5));
        snapshot.subtotal = 50.0;
        snapshot.total = 50.0;

        // Remove 2 of 5
        let event = create_item_removed_event("order-1", 2, "item-1", "Product A", Some(2), None);

        let applier = ItemRemovedApplier;
        applier.apply(&mut snapshot, &event);

        // Item should still exist with reduced quantity
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].quantity, 3);
        assert_eq!(snapshot.items[0].unpaid_quantity, 3);
        // 10.0 * 3 = 30.0
        assert_eq!(snapshot.subtotal, 30.0);
        assert_eq!(snapshot.total, 30.0);
    }

    #[test]
    fn test_item_removed_partial_to_zero() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 3));
        snapshot.subtotal = 30.0;
        snapshot.total = 30.0;

        // Remove all 3
        let event = create_item_removed_event("order-1", 2, "item-1", "Product A", Some(3), None);

        let applier = ItemRemovedApplier;
        applier.apply(&mut snapshot, &event);

        // Item should be removed when quantity reaches 0
        assert_eq!(snapshot.items.len(), 0);
        assert_eq!(snapshot.subtotal, 0.0);
        assert_eq!(snapshot.total, 0.0);
    }

    #[test]
    fn test_item_removed_with_discount() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let mut item = create_test_item("item-1", 1, "Product A", 100.0, 2);
        item.manual_discount_percent = Some(10.0);
        snapshot.items.push(item);
        // 100 * 2 * 0.9 = 180
        snapshot.subtotal = 180.0;
        snapshot.total = 180.0;

        // Remove 1 of 2 with 10% discount
        let event = create_item_removed_event("order-1", 2, "item-1", "Product A", Some(1), None);

        let applier = ItemRemovedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].quantity, 1);
        // 100 * 1 * 0.9 = 90
        assert!((snapshot.subtotal - 90.0).abs() < 0.001);
    }

    #[test]
    fn test_item_removed_updates_sequence() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 1));
        snapshot.last_sequence = 5;

        let event = create_item_removed_event("order-1", 6, "item-1", "Product A", None, None);

        let applier = ItemRemovedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 6);
    }

    #[test]
    fn test_item_removed_updates_checksum() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 1));
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_item_removed_event("order-1", 1, "item-1", "Product A", None, None);

        let applier = ItemRemovedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_item_removed_nonexistent_is_noop() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 1));
        snapshot.subtotal = 10.0;
        snapshot.total = 10.0;

        // Try to remove nonexistent item
        let event = create_item_removed_event("order-1", 1, "nonexistent", "Product A", None, None);

        let applier = ItemRemovedApplier;
        applier.apply(&mut snapshot, &event);

        // Original item should be unchanged
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "item-1");
        // Subtotal recalculated but unchanged
        assert_eq!(snapshot.subtotal, 10.0);
    }

    #[test]
    fn test_item_removed_multiple_items() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 2));
        snapshot
            .items
            .push(create_test_item("item-2", 2, "Product B", 20.0, 1));
        snapshot.subtotal = 40.0; // 10*2 + 20*1
        snapshot.total = 40.0;

        // Remove item-1
        let event = create_item_removed_event("order-1", 2, "item-1", "Product A", None, None);

        let applier = ItemRemovedApplier;
        applier.apply(&mut snapshot, &event);

        // Only item-2 should remain
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "item-2");
        assert_eq!(snapshot.subtotal, 20.0);
        assert_eq!(snapshot.total, 20.0);
    }

    #[test]
    fn test_item_removed_with_paid_quantities() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 3));
        // 1 of 3 already paid
        snapshot
            .paid_item_quantities
            .insert("item-1".to_string(), 1);

        // Remove item entirely
        let event = create_item_removed_event("order-1", 1, "item-1", "Product A", None, None);

        let applier = ItemRemovedApplier;
        applier.apply(&mut snapshot, &event);

        // Item removed, paid_item_quantities should be cleaned up
        assert_eq!(snapshot.items.len(), 0);
        assert!(!snapshot.paid_item_quantities.contains_key("item-1"));
    }

    #[test]
    fn test_item_removed_partial_with_paid_quantities() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let mut item = create_test_item("item-1", 1, "Product A", 10.0, 5);
        item.unpaid_quantity = 4; // 1 already paid
        snapshot.items.push(item);
        snapshot
            .paid_item_quantities
            .insert("item-1".to_string(), 1);
        snapshot.subtotal = 50.0;
        snapshot.total = 50.0;

        // Remove 2 items
        let event = create_item_removed_event("order-1", 1, "item-1", "Product A", Some(2), None);

        let applier = ItemRemovedApplier;
        applier.apply(&mut snapshot, &event);

        // Quantity should be 3, unpaid recalculated
        assert_eq!(snapshot.items[0].quantity, 3);
        // unpaid = quantity - paid = 3 - 1 = 2
        assert_eq!(snapshot.items[0].unpaid_quantity, 2);
        // paid_item_quantities should still exist since item still exists
        assert!(snapshot.paid_item_quantities.contains_key("item-1"));
    }

    #[test]
    fn test_item_removed_with_reason() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 1));

        let event = create_item_removed_event(
            "order-1",
            1,
            "item-1",
            "Product A",
            None,
            Some("Customer changed mind".to_string()),
        );

        let applier = ItemRemovedApplier;
        applier.apply(&mut snapshot, &event);

        // Item should be removed (reason is for audit trail, not stored in snapshot)
        assert_eq!(snapshot.items.len(), 0);
    }

    #[test]
    fn test_item_removed_updates_timestamp() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 1));
        snapshot.updated_at = 1000;

        // Event has a different timestamp
        let event = create_item_removed_event("order-1", 1, "item-1", "Product A", None, None);

        let applier = ItemRemovedApplier;
        applier.apply(&mut snapshot, &event);

        // updated_at should be updated to event timestamp (1234567890)
        assert_eq!(snapshot.updated_at, 1234567890);
    }

    #[test]
    fn test_apply_item_removed_function_full() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 2));

        apply_item_removed(&mut snapshot, "item-1", None);

        assert_eq!(snapshot.items.len(), 0);
    }

    #[test]
    fn test_apply_item_removed_function_partial() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot
            .items
            .push(create_test_item("item-1", 1, "Product A", 10.0, 5));

        apply_item_removed(&mut snapshot, "item-1", Some(2));

        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].quantity, 3);
    }

    #[test]
    fn test_recalculate_totals_empty() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.subtotal = 100.0;
        snapshot.total = 100.0;

        recalculate_totals(&mut snapshot);

        assert_eq!(snapshot.subtotal, 0.0);
        assert_eq!(snapshot.total, 0.0);
    }

    #[test]
    fn test_recalculate_totals_with_tax_discount() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        // Create item with 21% IVA tax rate (Spanish standard rate)
        let mut item = create_test_item("item-1", 1, "Product A", 121.0, 1);
        item.tax_rate = 21; // 21% IVA
        snapshot.items.push(item);
        snapshot.order_manual_discount_fixed = Some(5.0); // Use structured field

        recalculate_totals(&mut snapshot);

        assert_eq!(snapshot.subtotal, 121.0);
        // Spanish IVA: price is tax-inclusive
        // Tax = 121 * 21 / (100 + 21) = 121 * 21 / 121 = 21.0
        assert_eq!(snapshot.tax, 21.0);
        // total = subtotal - discount = 121 - 5 = 116 (tax already included)
        assert_eq!(snapshot.total, 116.0);
        assert_eq!(snapshot.discount, 5.0);
    }
}
