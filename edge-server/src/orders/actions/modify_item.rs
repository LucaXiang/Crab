//! ModifyItem command handler
//!
//! Modifies an existing item in an order. Supports:
//! - Full modification (all items of same instance_id)
//! - Partial modification (splits item into unchanged + modified portions)
//!
//! Operations include: APPLY_DISCOUNT, MODIFY_PRICE, MODIFY_QUANTITY, MODIFY_ITEM

use async_trait::async_trait;

use crate::orders::reducer::generate_instance_id_from_parts;
use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{
    CartItemSnapshot, EventPayload, ItemChanges, ItemModificationResult, OrderEvent,
    OrderEventType, OrderStatus,
};

/// ModifyItem action
#[derive(Debug, Clone)]
pub struct ModifyItemAction {
    pub order_id: String,
    pub instance_id: String,
    pub affected_quantity: Option<i32>,
    pub changes: ItemChanges,
    pub authorizer_id: Option<i64>,
    pub authorizer_name: Option<String>,
}

#[async_trait]
impl CommandHandler for ModifyItemAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Validate changes
        crate::orders::money::validate_item_changes(&self.changes)?;

        // 2. Load existing snapshot
        let snapshot = ctx.load_snapshot(&self.order_id)?;

        // 3. Validate order status
        match snapshot.status {
            OrderStatus::Completed => {
                return Err(OrderError::OrderAlreadyCompleted(self.order_id.clone()));
            }
            OrderStatus::Void => {
                return Err(OrderError::OrderAlreadyVoided(self.order_id.clone()));
            }
            _ => {}
        }

        // 4. Find the item
        let item = snapshot
            .items
            .iter()
            .find(|i| i.instance_id == self.instance_id)
            .ok_or_else(|| OrderError::ItemNotFound(self.instance_id.clone()))?;

        // 5. Reject modifications to comped items (locked)
        if item.is_comped {
            return Err(OrderError::InvalidOperation(
                "Cannot modify a comped item".to_string(),
            ));
        }

        // 6. Reject no-op modifications (all specified changes match current state)
        if !has_actual_changes(item, &self.changes) {
            return Err(OrderError::InvalidOperation(
                "No actual changes detected".to_string(),
            ));
        }

        // 7. Validate affected quantity
        let affected_qty = self.affected_quantity.unwrap_or(item.quantity);
        if affected_qty <= 0 {
            return Err(OrderError::InvalidOperation(
                "affected_quantity must be positive".to_string(),
            ));
        }
        if affected_qty > item.quantity {
            return Err(OrderError::InsufficientQuantity);
        }

        // 8. Validate quantity against paid amount
        if let Some(new_qty) = self.changes.quantity
            && new_qty <= 0
        {
            return Err(OrderError::InvalidOperation(
                "quantity must be positive".to_string(),
            ));
        }

        // 8b. Validate affected_qty doesn't exceed unpaid quantity
        //     (cannot modify already-paid portions via partial split)
        let paid_qty = snapshot
            .paid_item_quantities
            .get(&self.instance_id)
            .copied()
            .unwrap_or(0);
        if paid_qty > 0 && affected_qty > item.unpaid_quantity && affected_qty < item.quantity {
            return Err(OrderError::InvalidOperation(format!(
                "affected_quantity ({}) exceeds unpaid quantity ({})",
                affected_qty, item.unpaid_quantity
            )));
        }

        // 9. Calculate previous values for audit trail
        let previous_values = ItemChanges {
            price: if self.changes.price.is_some() {
                Some(item.price)
            } else {
                None
            },
            quantity: if self.changes.quantity.is_some() {
                Some(item.unpaid_quantity)
            } else {
                None
            },
            manual_discount_percent: if self.changes.manual_discount_percent.is_some() {
                item.manual_discount_percent
            } else {
                None
            },
            note: if self.changes.note.is_some() {
                item.note.clone()
            } else {
                None
            },
            selected_options: if self.changes.selected_options.is_some() {
                item.selected_options.clone()
            } else {
                None
            },
            selected_specification: if self.changes.selected_specification.is_some() {
                item.selected_specification.clone()
            } else {
                None
            },
        };

        // 10. Determine operation type for audit
        let operation = determine_operation(&self.changes);

        // 11. Calculate modification results (handle split scenario)
        let results = calculate_modification_results(item, affected_qty, &self.changes, paid_qty);

        // 12. Allocate sequence number
        let seq = ctx.next_sequence();

        // 13. Create event
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id,
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::ItemModified,
            EventPayload::ItemModified {
                operation: operation.to_string(),
                source: Box::new(item.clone()),
                affected_quantity: affected_qty,
                changes: Box::new(self.changes.clone()),
                previous_values: Box::new(previous_values),
                results,
                authorizer_id: self.authorizer_id,
                authorizer_name: self.authorizer_name.clone(),
            },
        );

        Ok(vec![event])
    }
}

/// Check if the changes contain any actual difference from the current item state.
///
/// Returns `false` (no-op) when all specified fields match current values.
/// Treats `None` and `Some(0.0)` as equivalent for discount.
fn has_actual_changes(item: &CartItemSnapshot, changes: &ItemChanges) -> bool {
    if let Some(price) = changes.price
        && (price - item.price).abs() > 0.01 {
            return true;
        }
    if let Some(qty) = changes.quantity
        && qty != item.unpaid_quantity {
            return true;
        }
    if let Some(discount) = changes.manual_discount_percent {
        let current = item.manual_discount_percent.unwrap_or(0.0);
        if (discount - current).abs() > 0.01 {
            return true;
        }
    }
    if let Some(ref note) = changes.note
        && item.note.as_ref() != Some(note) {
            return true;
        }
    if let Some(ref new_opts) = changes.selected_options {
        let current = item.selected_options.as_deref().unwrap_or(&[]);
        if new_opts.len() != current.len() {
            return true;
        }
        // Compare by (attribute_id, option_idx, quantity) tuples
        let mut new_keys: Vec<_> = new_opts
            .iter()
            .map(|o| (&o.attribute_id, o.option_idx, o.quantity))
            .collect();
        let mut cur_keys: Vec<_> = current
            .iter()
            .map(|o| (&o.attribute_id, o.option_idx, o.quantity))
            .collect();
        new_keys.sort();
        cur_keys.sort();
        if new_keys != cur_keys {
            return true;
        }
    }
    if let Some(ref new_spec) = changes.selected_specification {
        match &item.selected_specification {
            None => return true, // adding a spec is a change
            Some(curr) => {
                if new_spec.id != curr.id {
                    return true;
                }
                // Compare price (both optional)
                let new_p = new_spec.price.unwrap_or(0.0);
                let cur_p = curr.price.unwrap_or(0.0);
                if (new_p - cur_p).abs() > 0.01 {
                    return true;
                }
            }
        }
    }
    // All specified fields match current state, or no fields specified
    false
}

/// Determine the operation type based on changes
fn determine_operation(changes: &ItemChanges) -> &'static str {
    if changes.manual_discount_percent.is_some() {
        "APPLY_DISCOUNT"
    } else if changes.price.is_some() {
        "MODIFY_PRICE"
    } else if changes.quantity.is_some() {
        "MODIFY_QUANTITY"
    } else if changes.selected_options.is_some() || changes.selected_specification.is_some() {
        "MODIFY_OPTIONS"
    } else if changes.note.is_some() {
        "MODIFY_NOTE"
    } else {
        "MODIFY_ITEM"
    }
}

/// Calculate modification results, handling split scenario.
///
/// When `paid_qty > 0` and changes affect price/discount, the applier will split
/// the item into a frozen paid portion and a new unpaid portion. In this case,
/// generate a unique instance_id (with UUID suffix) for the new portion to avoid
/// hash collisions with previously frozen items that may share the same properties.
/// (Same pattern as comp: `{source}::comp::{uuid}`)
fn calculate_modification_results(
    item: &CartItemSnapshot,
    affected_qty: i32,
    changes: &ItemChanges,
    paid_qty: i32,
) -> Vec<ItemModificationResult> {
    let new_price = changes.price.unwrap_or(item.price);
    let new_discount = changes
        .manual_discount_percent
        .or(item.manual_discount_percent)
        .filter(|&d| d.abs() > 0.01);
    let new_options = changes
        .selected_options
        .as_ref()
        .or(item.selected_options.as_ref());
    let new_specification = changes
        .selected_specification
        .as_ref()
        .or(item.selected_specification.as_ref());

    // Generate base instance_id from item properties (deterministic hash)
    let base_id = generate_instance_id_from_parts(
        item.id,
        new_price,
        new_discount,
        &new_options.cloned(),
        &new_specification.cloned(),
    );

    // When item has paid portions AND price/discount is changing, the applier
    // will split: frozen paid portion keeps original instance_id, new unpaid
    // portion needs a unique ID to avoid collision with previously frozen items.
    let has_price_change =
        changes.price.is_some() || changes.manual_discount_percent.is_some();
    let new_instance_id = if paid_qty > 0 && has_price_change {
        format!("{}::mod::{}", base_id, uuid::Uuid::new_v4())
    } else {
        base_id
    };

    if affected_qty >= item.quantity {
        // Full modification
        vec![ItemModificationResult {
            instance_id: new_instance_id,
            quantity: item.quantity,
            price: new_price,
            manual_discount_percent: new_discount,
            action: "UPDATED".to_string(),
        }]
    } else {
        // Partial modification: split into unchanged + modified portions
        vec![
            // Unchanged portion (reduced quantity)
            ItemModificationResult {
                instance_id: item.instance_id.clone(),
                quantity: item.quantity - affected_qty,
                price: item.price,
                manual_discount_percent: item.manual_discount_percent,
                action: "UNCHANGED".to_string(),
            },
            // Modified portion (new item)
            ItemModificationResult {
                instance_id: new_instance_id,
                quantity: affected_qty,
                price: new_price,
                manual_discount_percent: new_discount,
                action: "CREATED".to_string(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orders::storage::OrderStorage;
    use crate::orders::traits::CommandContext;
    use shared::order::OrderSnapshot;

    fn create_test_metadata() -> CommandMetadata {
        CommandMetadata {
            command_id: "cmd-1".to_string(),
            operator_id: 1,
            operator_name: "Test User".to_string(),
            timestamp: 1234567890,
        }
    }

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

    fn create_active_order_with_item(order_id: &str, item: CartItemSnapshot) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items.push(item);
        snapshot
    }

    #[tokio::test]
    async fn test_modify_item_full_price_change() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create order with item
        let item = create_test_item("item-1", 1, "Test Product", 10.0, 2);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None, // Full modification
            changes: ItemChanges {
                price: Some(15.0),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.order_id, "order-1");
        assert_eq!(event.event_type, OrderEventType::ItemModified);

        if let EventPayload::ItemModified {
            operation,
            source,
            affected_quantity,
            changes,
            previous_values,
            results,
            ..
        } = &event.payload
        {
            assert_eq!(operation, "MODIFY_PRICE");
            assert_eq!(source.instance_id, "item-1");
            assert_eq!(*affected_quantity, 2);
            assert_eq!(changes.price, Some(15.0));
            assert_eq!(previous_values.price, Some(10.0));
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].action, "UPDATED");
            assert_eq!(results[0].price, 15.0);
            assert_eq!(results[0].quantity, 2);
        } else {
            panic!("Expected ItemModified payload");
        }
    }

    #[tokio::test]
    async fn test_modify_item_partial_creates_split() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create order with item quantity=5
        let item = create_test_item("item-1", 1, "Test Product", 10.0, 5);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: Some(2), // Partial: only 2 of 5
            changes: ItemChanges {
                manual_discount_percent: Some(10.0),
                ..Default::default()
            },
            authorizer_id: Some(1),
            authorizer_name: Some("Manager".to_string()),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::ItemModified {
            operation,
            affected_quantity,
            results,
            authorizer_id,
            authorizer_name,
            ..
        } = &events[0].payload
        {
            assert_eq!(operation, "APPLY_DISCOUNT");
            assert_eq!(*affected_quantity, 2);
            assert_eq!(results.len(), 2);

            // First result: unchanged portion
            assert_eq!(results[0].action, "UNCHANGED");
            assert_eq!(results[0].quantity, 3); // 5 - 2
            assert_eq!(results[0].instance_id, "item-1");
            assert_eq!(results[0].manual_discount_percent, None);

            // Second result: modified portion (new item)
            assert_eq!(results[1].action, "CREATED");
            assert_eq!(results[1].quantity, 2);
            assert_ne!(results[1].instance_id, "item-1"); // New instance_id
            assert_eq!(results[1].manual_discount_percent, Some(10.0));

            assert_eq!(*authorizer_id, Some(1));
            assert_eq!(authorizer_name.as_deref(), Some("Manager"));
        } else {
            panic!("Expected ItemModified payload");
        }
    }

    #[tokio::test]
    async fn test_modify_item_apply_discount() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", 1, "Test Product", 100.0, 1);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None,
            changes: ItemChanges {
                manual_discount_percent: Some(20.0),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::ItemModified {
            operation,
            previous_values,
            results,
            ..
        } = &events[0].payload
        {
            assert_eq!(operation, "APPLY_DISCOUNT");
            assert_eq!(previous_values.manual_discount_percent, None); // Was None before
            assert_eq!(results[0].manual_discount_percent, Some(20.0));
        } else {
            panic!("Expected ItemModified payload");
        }
    }

    #[tokio::test]
    async fn test_modify_item_not_found() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", 1, "Test Product", 10.0, 1);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "nonexistent".to_string(),
            affected_quantity: None,
            changes: ItemChanges {
                price: Some(15.0),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::ItemNotFound(_))));
    }

    #[tokio::test]
    async fn test_modify_item_insufficient_quantity() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", 1, "Test Product", 10.0, 3);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: Some(5), // More than available
            changes: ItemChanges {
                price: Some(15.0),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InsufficientQuantity)));
    }

    #[tokio::test]
    async fn test_modify_item_completed_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", 1, "Test Product", 10.0, 1);
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        snapshot.items.push(item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None,
            changes: ItemChanges {
                price: Some(15.0),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[tokio::test]
    async fn test_modify_item_voided_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", 1, "Test Product", 10.0, 1);
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        snapshot.items.push(item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None,
            changes: ItemChanges {
                price: Some(15.0),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyVoided(_))));
    }

    #[tokio::test]
    async fn test_modify_item_zero_affected_quantity_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", 1, "Test Product", 10.0, 3);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: Some(0),
            changes: ItemChanges {
                price: Some(15.0),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_modify_item_note() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", 1, "Test Product", 10.0, 1);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None,
            changes: ItemChanges {
                note: Some("Extra spicy".to_string()),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::ItemModified {
            operation, changes, ..
        } = &events[0].payload
        {
            assert_eq!(operation, "MODIFY_NOTE");
            assert_eq!(changes.note, Some("Extra spicy".to_string()));
        } else {
            panic!("Expected ItemModified payload");
        }
    }

    #[tokio::test]
    async fn test_determine_operation() {
        // Discount takes precedence
        assert_eq!(
            determine_operation(&ItemChanges {
                price: Some(10.0),
                manual_discount_percent: Some(5.0),
                ..Default::default()
            }),
            "APPLY_DISCOUNT"
        );

        // Price without discount
        assert_eq!(
            determine_operation(&ItemChanges {
                price: Some(10.0),
                ..Default::default()
            }),
            "MODIFY_PRICE"
        );

        // Quantity
        assert_eq!(
            determine_operation(&ItemChanges {
                quantity: Some(5),
                ..Default::default()
            }),
            "MODIFY_QUANTITY"
        );

        // Note
        assert_eq!(
            determine_operation(&ItemChanges {
                note: Some("test".to_string()),
                ..Default::default()
            }),
            "MODIFY_NOTE"
        );

        // Options
        assert_eq!(
            determine_operation(&ItemChanges {
                selected_options: Some(vec![]),
                ..Default::default()
            }),
            "MODIFY_OPTIONS"
        );

        // Specification
        assert_eq!(
            determine_operation(&ItemChanges {
                selected_specification: Some(shared::order::SpecificationInfo {
                    id: 1,
                    name: "Large".to_string(),

                    receipt_name: None,
                    price: None,
                }),
                ..Default::default()
            }),
            "MODIFY_OPTIONS"
        );

        // Empty changes
        assert_eq!(determine_operation(&ItemChanges::default()), "MODIFY_ITEM");
    }

    #[tokio::test]
    async fn test_modify_item_options() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", 1, "Test Product", 10.0, 1);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let new_options = vec![shared::order::ItemOption {
            attribute_id: 1,
            attribute_name: "Size".to_string(),
            option_idx: 1,
            option_name: "Large".to_string(),
            price_modifier: Some(2.0),
            quantity: 1,
        }];

        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None,
            changes: ItemChanges {
                selected_options: Some(new_options.clone()),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::ItemModified {
            operation,
            changes,
            previous_values,
            ..
        } = &events[0].payload
        {
            assert_eq!(operation, "MODIFY_OPTIONS");
            assert!(changes.selected_options.is_some());
            assert_eq!(changes.selected_options.as_ref().unwrap().len(), 1);
            assert!(previous_values.selected_options.is_none()); // Was None before
        } else {
            panic!("Expected ItemModified payload");
        }
    }

    #[tokio::test]
    async fn test_modify_item_specification() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", 1, "Test Product", 10.0, 1);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let new_spec = shared::order::SpecificationInfo {
            id: 1,
            name: "Large".to_string(),
            receipt_name: Some("L".to_string()),
            price: Some(15.0),
        };

        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None,
            changes: ItemChanges {
                selected_specification: Some(new_spec.clone()),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::ItemModified {
            operation,
            changes,
            previous_values,
            ..
        } = &events[0].payload
        {
            assert_eq!(operation, "MODIFY_OPTIONS");
            assert!(changes.selected_specification.is_some());
            assert_eq!(
                changes.selected_specification.as_ref().unwrap().name,
                "Large"
            );
            assert!(previous_values.selected_specification.is_none());
        } else {
            panic!("Expected ItemModified payload");
        }
    }

    // ---- No-op detection tests ----

    #[tokio::test]
    async fn test_modify_item_no_op_empty_changes_rejected() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", 1, "Test Product", 10.0, 2);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Send empty changes
        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None,
            changes: ItemChanges::default(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_modify_item_no_op_same_price_rejected() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", 1, "Test Product", 10.0, 1);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Send same price as current
        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None,
            changes: ItemChanges {
                price: Some(10.0),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_modify_item_no_op_discount_none_vs_zero_rejected() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Item has manual_discount_percent = None
        let item = create_test_item("item-1", 1, "Test Product", 10.0, 1);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Send discount = 0 (should be treated as same as None)
        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None,
            changes: ItemChanges {
                manual_discount_percent: Some(0.0),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_modify_item_no_op_same_options_rejected() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let opts = vec![shared::order::ItemOption {
            attribute_id: 1,
            attribute_name: "Size".to_string(),
            option_idx: 1,
            option_name: "Large".to_string(),
            price_modifier: Some(2.0),
            quantity: 1,
        }];

        let mut item = create_test_item("item-1", 1, "Test Product", 10.0, 1);
        item.selected_options = Some(opts.clone());
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Send same options
        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None,
            changes: ItemChanges {
                selected_options: Some(opts),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_modify_item_no_op_same_spec_rejected() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let spec = shared::order::SpecificationInfo {
            id: 0,
            name: "CCC".to_string(),
            receipt_name: None,
            price: Some(10.0),
        };

        let mut item = create_test_item("item-1", 1, "Test Product", 10.0, 1);
        item.selected_specification = Some(spec.clone());
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Send same spec
        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None,
            changes: ItemChanges {
                selected_specification: Some(spec),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[test]
    fn test_has_actual_changes_detects_real_changes() {
        let item = create_test_item("item-1", 1, "Test Product", 10.0, 2);

        // Different price → true
        assert!(has_actual_changes(
            &item,
            &ItemChanges { price: Some(15.0), ..Default::default() }
        ));

        // Different quantity → true
        assert!(has_actual_changes(
            &item,
            &ItemChanges { quantity: Some(5), ..Default::default() }
        ));

        // Different discount → true
        assert!(has_actual_changes(
            &item,
            &ItemChanges { manual_discount_percent: Some(10.0), ..Default::default() }
        ));

        // Adding options → true
        assert!(has_actual_changes(
            &item,
            &ItemChanges {
                selected_options: Some(vec![shared::order::ItemOption {
                    attribute_id: 1,
                    attribute_name: "Size".to_string(),
                    option_idx: 1,
                    option_name: "Large".to_string(),
                    price_modifier: None,
                    quantity: 1,
                }]),
                ..Default::default()
            }
        ));

        // Adding spec → true
        assert!(has_actual_changes(
            &item,
            &ItemChanges {
                selected_specification: Some(shared::order::SpecificationInfo {
                    id: 0,
                    name: "Large".to_string(),

                    receipt_name: None,
                    price: Some(15.0),
                }),
                ..Default::default()
            }
        ));
    }

    /// Test: changes.quantity is unpaid, not total.
    /// Item total=5, paid=2, unpaid=3. User changes unpaid 3→5.
    /// changes.quantity=5 must NOT be confused with item.quantity=5.
    #[tokio::test]
    async fn test_modify_paid_item_unpaid_quantity_semantics() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut item = create_test_item("item-1", 1, "Test Product", 5.0, 5);
        item.unpaid_quantity = 3; // 5 total - 2 paid = 3 unpaid
        let mut snapshot = create_active_order_with_item("order-1", item);
        snapshot.paid_item_quantities.insert("item-1".to_string(), 2);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // User changes unpaid from 3 to 5
        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None,
            changes: ItemChanges {
                quantity: Some(5), // new unpaid = 5 (same as item.quantity!)
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::ItemModified {
            changes,
            previous_values,
            ..
        } = &events[0].payload
        {
            // changes.quantity = 5 (new unpaid)
            assert_eq!(changes.quantity, Some(5));
            // previous_values.quantity = 3 (old unpaid, NOT 5 total)
            assert_eq!(previous_values.quantity, Some(3));
        } else {
            panic!("Expected ItemModified payload");
        }
    }

    // ---- Paid item unique ID tests ----

    /// BUG FIX: paid item + price/discount change → result uses unique instance_id
    /// (prevents collision when discount cycles back to a previously-used value)
    #[tokio::test]
    async fn test_modify_paid_item_discount_generates_unique_id() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", 1, "Test Product", 10.0, 10);
        let mut snapshot = create_active_order_with_item("order-1", item);
        snapshot.paid_item_quantities.insert("item-1".to_string(), 3);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None,
            changes: ItemChanges {
                manual_discount_percent: Some(50.0),
                ..Default::default()
            },
            authorizer_id: Some(2),
            authorizer_name: Some("Admin".to_string()),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::ItemModified { results, .. } = &events[0].payload {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].action, "UPDATED");
            // Must contain ::mod:: suffix for uniqueness
            assert!(
                results[0].instance_id.contains("::mod::"),
                "Paid item with price change should have unique instance_id, got: {}",
                results[0].instance_id
            );
        } else {
            panic!("Expected ItemModified payload");
        }
    }

    /// Non-price change on paid item → deterministic instance_id (no UUID suffix)
    #[tokio::test]
    async fn test_modify_paid_item_note_keeps_deterministic_id() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", 1, "Test Product", 10.0, 5);
        let mut snapshot = create_active_order_with_item("order-1", item);
        snapshot.paid_item_quantities.insert("item-1".to_string(), 2);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None,
            changes: ItemChanges {
                note: Some("Extra spicy".to_string()),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::ItemModified { results, .. } = &events[0].payload {
            assert_eq!(results.len(), 1);
            // Non-price change → no UUID suffix
            assert!(
                !results[0].instance_id.contains("::mod::"),
                "Non-price change should have deterministic instance_id, got: {}",
                results[0].instance_id
            );
        } else {
            panic!("Expected ItemModified payload");
        }
    }

    /// No paid items → deterministic instance_id regardless of change type
    #[tokio::test]
    async fn test_modify_unpaid_item_discount_keeps_deterministic_id() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", 1, "Test Product", 10.0, 5);
        let snapshot = create_active_order_with_item("order-1", item);
        // No paid_item_quantities
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None,
            changes: ItemChanges {
                manual_discount_percent: Some(50.0),
                ..Default::default()
            },
            authorizer_id: Some(2),
            authorizer_name: Some("Admin".to_string()),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::ItemModified { results, .. } = &events[0].payload {
            // No paid items → deterministic, no UUID suffix
            assert!(
                !results[0].instance_id.contains("::mod::"),
                "Unpaid item should have deterministic instance_id, got: {}",
                results[0].instance_id
            );
        } else {
            panic!("Expected ItemModified payload");
        }
    }

    // ---- Comped item protection tests ----

    #[tokio::test]
    async fn test_modify_comped_item_rejected() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut item = create_test_item("item-1", 1, "Comp Beer", 5.0, 1);
        item.is_comped = true;
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None,
            changes: ItemChanges {
                price: Some(10.0),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
        if let Err(OrderError::InvalidOperation(msg)) = result {
            assert!(msg.contains("comped"), "Error message should mention comped: {msg}");
        }
    }
}
