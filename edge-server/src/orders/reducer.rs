//! Event reducer for order snapshot computation
//!
//! The reducer applies events to snapshots to compute the current state.
//! This is the core of the event sourcing pattern - snapshots are always
//! derivable from the event stream.
//!
//! # Invariant
//!
//! For any order: `rebuild(events) == snapshot`

use shared::order::{
    CartItemSnapshot, EventPayload, ItemChanges, OrderEvent, OrderEventType, OrderSnapshot,
    OrderStatus, PaymentRecord,
};

/// Order event reducer
pub struct OrderReducer;

impl OrderReducer {
    /// Create a new snapshot from an initial TableOpened event
    pub fn create_snapshot(event: &OrderEvent) -> Option<OrderSnapshot> {
        match &event.payload {
            EventPayload::TableOpened {
                table_id,
                table_name,
                zone_id,
                zone_name,
                guest_count,
                is_retail,
                receipt_number,
            } => {
                let mut snapshot = OrderSnapshot::new(event.order_id.clone());
                snapshot.table_id = table_id.clone();
                snapshot.table_name = table_name.clone();
                snapshot.zone_id = zone_id.clone();
                snapshot.zone_name = zone_name.clone();
                snapshot.guest_count = *guest_count;
                snapshot.is_retail = *is_retail;
                snapshot.receipt_number = receipt_number.clone();
                snapshot.start_time = event.timestamp;
                snapshot.created_at = event.timestamp;
                snapshot.updated_at = event.timestamp;
                snapshot.last_sequence = event.sequence;
                // Update checksum after modifications
                snapshot.update_checksum();
                Some(snapshot)
            }
            _ => None,
        }
    }

    /// Apply an event to a snapshot, returning the updated snapshot
    pub fn apply_event(snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        // Skip if event is already applied
        if event.sequence <= snapshot.last_sequence {
            return;
        }

        match &event.payload {
            EventPayload::TableOpened { .. } => {
                // TableOpened should only create a new snapshot, not apply to existing
            }

            EventPayload::OrderCompleted {
                receipt_number,
                final_total: _,
                payment_summary: _,
            } => {
                snapshot.status = OrderStatus::Completed;
                snapshot.receipt_number = Some(receipt_number.clone());
                snapshot.end_time = Some(event.timestamp);
            }

            EventPayload::OrderVoided { .. } => {
                snapshot.status = OrderStatus::Void;
                snapshot.end_time = Some(event.timestamp);
            }

            EventPayload::OrderRestored {} => {
                snapshot.status = OrderStatus::Active;
                snapshot.end_time = None;
            }

            EventPayload::ItemsAdded { items } => {
                Self::apply_items_added(snapshot, items);
            }

            EventPayload::ItemModified {
                source,
                affected_quantity,
                changes,
                results,
                ..
            } => {
                Self::apply_item_modified(snapshot, source, *affected_quantity, changes, results);
            }

            EventPayload::ItemRemoved {
                instance_id,
                quantity,
                ..
            } => {
                Self::apply_item_removed(snapshot, instance_id, *quantity);
            }

            EventPayload::ItemRestored { instance_id, .. } => {
                // For now, item restoration is not fully implemented
                // This would require tracking removed items
                tracing::warn!(
                    order_id = %snapshot.order_id,
                    instance_id = %instance_id,
                    "ItemRestored event received but restoration is not fully implemented"
                );
            }

            EventPayload::PaymentAdded {
                payment_id,
                method,
                amount,
                tendered,
                change,
                note,
            } => {
                Self::apply_payment_added(
                    snapshot,
                    payment_id,
                    method,
                    *amount,
                    *tendered,
                    *change,
                    note.clone(),
                    event.timestamp,
                );
            }

            EventPayload::PaymentCancelled { payment_id, .. } => {
                Self::apply_payment_cancelled(snapshot, payment_id);
            }

            EventPayload::OrderSplit {
                split_amount,
                payment_method,
                items,
            } => {
                Self::apply_order_split(snapshot, *split_amount, payment_method, items);
            }

            EventPayload::OrderMoved {
                target_table_id,
                target_table_name,
                items,
                ..
            } => {
                // This order receives items from another table
                snapshot.table_id = Some(target_table_id.clone());
                snapshot.table_name = Some(target_table_name.clone());
                for item in items {
                    Self::add_or_merge_item(snapshot, item);
                }
            }

            EventPayload::OrderMovedOut { .. } => {
                // This order was moved out - mark as moved
                snapshot.status = OrderStatus::Moved;
            }

            EventPayload::OrderMerged { items, .. } => {
                // This order receives items from another order
                for item in items {
                    Self::add_or_merge_item(snapshot, item);
                }
            }

            EventPayload::OrderMergedOut { .. } => {
                // This order was merged into another - mark as merged
                snapshot.status = OrderStatus::Merged;
            }

            EventPayload::TableReassigned {
                target_table_id,
                target_table_name,
                target_zone_name,
                original_start_time,
                items,
                ..
            } => {
                snapshot.table_id = Some(target_table_id.clone());
                snapshot.table_name = Some(target_table_name.clone());
                snapshot.zone_name = target_zone_name.clone();
                snapshot.start_time = *original_start_time;
                snapshot.items = items.clone();
            }

            EventPayload::OrderInfoUpdated {
                receipt_number,
                guest_count,
                table_name,
                is_pre_payment,
            } => {
                if let Some(rn) = receipt_number {
                    snapshot.receipt_number = Some(rn.clone());
                }
                if let Some(gc) = guest_count {
                    snapshot.guest_count = *gc;
                }
                if let Some(tn) = table_name {
                    snapshot.table_name = Some(tn.clone());
                }
                if let Some(pp) = is_pre_payment {
                    snapshot.is_pre_payment = *pp;
                }
            }
        }

        // Update timestamp and sequence
        snapshot.updated_at = event.timestamp;
        snapshot.last_sequence = event.sequence;

        // Recalculate totals
        Self::recalculate_totals(snapshot);

        // Update state checksum for drift detection
        snapshot.update_checksum();
    }

    /// Apply multiple events to a snapshot
    pub fn apply_events(snapshot: &mut OrderSnapshot, events: &[OrderEvent]) {
        for event in events {
            Self::apply_event(snapshot, event);
        }
    }

    /// Rebuild a snapshot from events
    pub fn rebuild_from_events(events: &[OrderEvent]) -> Option<OrderSnapshot> {
        if events.is_empty() {
            return None;
        }

        // Find the TableOpened event
        let table_opened = events
            .iter()
            .find(|e| e.event_type == OrderEventType::TableOpened)?;

        let mut snapshot = Self::create_snapshot(table_opened)?;

        // Apply remaining events
        for event in events {
            if event.sequence > snapshot.last_sequence {
                Self::apply_event(&mut snapshot, event);
            }
        }

        Some(snapshot)
    }

    // ========== Private Helpers ==========

    fn apply_items_added(snapshot: &mut OrderSnapshot, items: &[CartItemSnapshot]) {
        for item in items {
            Self::add_or_merge_item(snapshot, item);
        }
    }

    fn add_or_merge_item(snapshot: &mut OrderSnapshot, item: &CartItemSnapshot) {
        // Check if an item with the same instance_id exists
        if let Some(existing) = snapshot
            .items
            .iter_mut()
            .find(|i| i.instance_id == item.instance_id)
        {
            // Merge by adding quantity
            existing.quantity += item.quantity;
        } else {
            // Add new item
            snapshot.items.push(item.clone());
        }
    }

    fn apply_item_modified(
        snapshot: &mut OrderSnapshot,
        source: &CartItemSnapshot,
        affected_quantity: i32,
        changes: &ItemChanges,
        results: &[shared::order::ItemModificationResult],
    ) {
        // Find the source item
        if let Some(idx) = snapshot
            .items
            .iter()
            .position(|i| i.instance_id == source.instance_id)
        {
            let original_qty = snapshot.items[idx].quantity;

            if affected_quantity >= original_qty {
                // Modify entire item
                Self::apply_changes_to_item(&mut snapshot.items[idx], changes);
            } else {
                // Split: reduce original quantity
                snapshot.items[idx].quantity = original_qty - affected_quantity;

                // Find new items from results
                for result in results {
                    if result.action == "CREATED" {
                        // Create new item with the changes applied
                        let mut new_item = source.clone();
                        new_item.instance_id = result.instance_id.clone();
                        new_item.quantity = result.quantity;
                        new_item.price = result.price;
                        new_item.discount_percent = result.discount_percent;
                        Self::apply_changes_to_item(&mut new_item, changes);
                        snapshot.items.push(new_item);
                    }
                }
            }
        }
    }

    fn apply_changes_to_item(item: &mut CartItemSnapshot, changes: &ItemChanges) {
        if let Some(price) = changes.price {
            item.price = price;
        }
        if let Some(quantity) = changes.quantity {
            item.quantity = quantity;
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

    fn apply_item_removed(snapshot: &mut OrderSnapshot, instance_id: &str, quantity: Option<i32>) {
        if let Some(qty) = quantity {
            // Partial removal
            if let Some(item) = snapshot
                .items
                .iter_mut()
                .find(|i| i.instance_id == instance_id)
            {
                item.quantity = (item.quantity - qty).max(0);
                if item.quantity == 0 {
                    snapshot.items.retain(|i| i.instance_id != instance_id);
                }
            }
        } else {
            // Full removal
            snapshot.items.retain(|i| i.instance_id != instance_id);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_payment_added(
        snapshot: &mut OrderSnapshot,
        payment_id: &str,
        method: &str,
        amount: f64,
        tendered: Option<f64>,
        change: Option<f64>,
        note: Option<String>,
        timestamp: i64,
    ) {
        let payment = PaymentRecord {
            payment_id: payment_id.to_string(),
            method: method.to_string(),
            amount,
            tendered,
            change,
            note,
            timestamp,
            cancelled: false,
            cancel_reason: None,
        };
        snapshot.payments.push(payment);
        snapshot.paid_amount += amount;
    }

    fn apply_payment_cancelled(snapshot: &mut OrderSnapshot, payment_id: &str) {
        if let Some(payment) = snapshot
            .payments
            .iter_mut()
            .find(|p| p.payment_id == payment_id)
            && !payment.cancelled
        {
            payment.cancelled = true;
            snapshot.paid_amount -= payment.amount;
        }
    }

    fn apply_order_split(
        snapshot: &mut OrderSnapshot,
        split_amount: f64,
        _payment_method: &str,
        items: &[shared::order::SplitItem],
    ) {
        // Track paid quantities
        for split_item in items {
            *snapshot
                .paid_item_quantities
                .entry(split_item.instance_id.clone())
                .or_insert(0) += split_item.quantity;
        }
        snapshot.paid_amount += split_amount;
    }

    fn recalculate_totals(snapshot: &mut OrderSnapshot) {
        // Calculate subtotal and update unpaid_quantity for each item
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

        // Calculate total (surcharge is now per-item via Price Rules)
        snapshot.total = subtotal + snapshot.tax - snapshot.discount;
    }
}

/// Generate a content-addressed instance_id from item properties
pub fn generate_instance_id(
    product_id: &str,
    price: f64,
    discount_percent: Option<f64>,
    options: &Option<Vec<shared::order::ItemOption>>,
    specification: &Option<shared::order::SpecificationInfo>,
    surcharge: Option<f64>,
) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();

    hasher.update(product_id.as_bytes());
    hasher.update(price.to_be_bytes());

    if let Some(discount) = discount_percent {
        hasher.update(discount.to_be_bytes());
    }

    if let Some(opts) = options {
        for opt in opts {
            hasher.update(opt.attribute_id.as_bytes());
            hasher.update(opt.option_idx.to_be_bytes());
        }
    }

    if let Some(spec) = specification {
        hasher.update(spec.id.as_bytes());
    }

    if let Some(s) = surcharge {
        hasher.update(s.to_be_bytes());
    }

    let result = hasher.finalize();
    hex::encode(&result[..16]) // Use first 16 bytes for shorter ID
}

/// Convert CartItemInput to CartItemSnapshot with generated instance_id
pub fn input_to_snapshot(input: &shared::order::CartItemInput) -> CartItemSnapshot {
    let instance_id = generate_instance_id(
        &input.product_id,
        input.price,
        input.discount_percent,
        &input.selected_options,
        &input.selected_specification,
        input.surcharge,
    );

    CartItemSnapshot {
        id: input.product_id.clone(),
        instance_id,
        name: input.name.clone(),
        price: input.price,
        original_price: input.original_price,
        quantity: input.quantity,
        unpaid_quantity: input.quantity, // Initially all unpaid
        selected_options: input.selected_options.clone(),
        selected_specification: input.selected_specification.clone(),
        discount_percent: input.discount_percent,
        surcharge: input.surcharge,
        note: input.note.clone(),
        authorizer_id: input.authorizer_id.clone(),
        authorizer_name: input.authorizer_name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::OrderEventType;

    fn create_table_opened_event(order_id: &str, sequence: u64) -> OrderEvent {
        OrderEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            sequence,
            order_id: order_id.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            client_timestamp: None,
            operator_id: "test_op".to_string(),
            operator_name: "Test Operator".to_string(),
            command_id: uuid::Uuid::new_v4().to_string(),
            event_type: OrderEventType::TableOpened,
            payload: EventPayload::TableOpened {
                table_id: Some("T1".to_string()),
                table_name: Some("Table 1".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 2,
                is_retail: false,
                receipt_number: None,
            },
        }
    }

    fn create_items_added_event(
        order_id: &str,
        sequence: u64,
        items: Vec<CartItemSnapshot>,
    ) -> OrderEvent {
        OrderEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            sequence,
            order_id: order_id.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            client_timestamp: None,
            operator_id: "test_op".to_string(),
            operator_name: "Test Operator".to_string(),
            command_id: uuid::Uuid::new_v4().to_string(),
            event_type: OrderEventType::ItemsAdded,
            payload: EventPayload::ItemsAdded { items },
        }
    }

    fn create_test_item(instance_id: &str, quantity: i32, price: f64) -> CartItemSnapshot {
        CartItemSnapshot {
            id: "product-1".to_string(),
            instance_id: instance_id.to_string(),
            name: "Test Product".to_string(),
            price,
            original_price: None,
            quantity,
            unpaid_quantity: quantity, // Initially all unpaid
            selected_options: None,
            selected_specification: None,
            discount_percent: None,
            surcharge: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        }
    }

    #[test]
    fn test_create_snapshot() {
        let event = create_table_opened_event("order-1", 1);
        let snapshot = OrderReducer::create_snapshot(&event);

        assert!(snapshot.is_some());
        let snapshot = snapshot.unwrap();
        assert_eq!(snapshot.order_id, "order-1");
        assert_eq!(snapshot.table_id, Some("T1".to_string()));
        assert_eq!(snapshot.guest_count, 2);
        assert_eq!(snapshot.status, OrderStatus::Active);
        assert_eq!(snapshot.last_sequence, 1);
    }

    #[test]
    fn test_apply_items_added() {
        let event1 = create_table_opened_event("order-1", 1);
        let mut snapshot = OrderReducer::create_snapshot(&event1).unwrap();

        let item = create_test_item("item-1", 2, 10.0);
        let event2 = create_items_added_event("order-1", 2, vec![item]);

        OrderReducer::apply_event(&mut snapshot, &event2);

        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].quantity, 2);
        assert_eq!(snapshot.subtotal, 20.0);
        assert_eq!(snapshot.total, 20.0);
        assert_eq!(snapshot.last_sequence, 2);
    }

    #[test]
    fn test_merge_same_instance_id() {
        let event1 = create_table_opened_event("order-1", 1);
        let mut snapshot = OrderReducer::create_snapshot(&event1).unwrap();

        // Add 2 items
        let item1 = create_test_item("item-1", 2, 10.0);
        let event2 = create_items_added_event("order-1", 2, vec![item1]);
        OrderReducer::apply_event(&mut snapshot, &event2);

        // Add 3 more of the same item
        let item2 = create_test_item("item-1", 3, 10.0);
        let event3 = create_items_added_event("order-1", 3, vec![item2]);
        OrderReducer::apply_event(&mut snapshot, &event3);

        // Should have merged into 5 items
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].quantity, 5);
        assert_eq!(snapshot.subtotal, 50.0);
    }

    #[test]
    fn test_rebuild_from_events() {
        let order_id = "order-1";

        let event1 = create_table_opened_event(order_id, 1);
        let item = create_test_item("item-1", 2, 10.0);
        let event2 = create_items_added_event(order_id, 2, vec![item]);

        let events = vec![event1, event2];
        let snapshot = OrderReducer::rebuild_from_events(&events);

        assert!(snapshot.is_some());
        let snapshot = snapshot.unwrap();
        assert_eq!(snapshot.order_id, order_id);
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.subtotal, 20.0);
        assert_eq!(snapshot.last_sequence, 2);
    }

    #[test]
    fn test_generate_instance_id() {
        let id1 = generate_instance_id("product-1", 10.0, None, &None, &None, None);
        let id2 = generate_instance_id("product-1", 10.0, None, &None, &None, None);
        let id3 = generate_instance_id("product-1", 10.0, Some(50.0), &None, &None, None);

        // Same inputs should produce same ID
        assert_eq!(id1, id2);

        // Different inputs should produce different ID
        assert_ne!(id1, id3);
    }
}
