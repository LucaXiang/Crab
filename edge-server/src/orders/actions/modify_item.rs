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
    pub authorizer_id: Option<String>,
    pub authorizer_name: Option<String>,
}

#[async_trait]
impl CommandHandler for ModifyItemAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Load existing snapshot
        let snapshot = ctx.load_snapshot(&self.order_id)?;

        // 2. Validate order status
        match snapshot.status {
            OrderStatus::Completed => {
                return Err(OrderError::OrderAlreadyCompleted(self.order_id.clone()));
            }
            OrderStatus::Void => {
                return Err(OrderError::OrderAlreadyVoided(self.order_id.clone()));
            }
            _ => {}
        }

        // 3. Find the item
        let item = snapshot
            .items
            .iter()
            .find(|i| i.instance_id == self.instance_id)
            .ok_or_else(|| OrderError::ItemNotFound(self.instance_id.clone()))?;

        // 4. Validate affected quantity
        let affected_qty = self.affected_quantity.unwrap_or(item.quantity);
        if affected_qty <= 0 {
            return Err(OrderError::InvalidOperation(
                "affected_quantity must be positive".to_string(),
            ));
        }
        if affected_qty > item.quantity {
            return Err(OrderError::InsufficientQuantity);
        }

        // 5. Calculate previous values for audit trail
        let previous_values = ItemChanges {
            price: if self.changes.price.is_some() {
                Some(item.price)
            } else {
                None
            },
            quantity: if self.changes.quantity.is_some() {
                Some(item.quantity)
            } else {
                None
            },
            manual_discount_percent: if self.changes.manual_discount_percent.is_some() {
                item.manual_discount_percent
            } else {
                None
            },
            surcharge: if self.changes.surcharge.is_some() {
                item.surcharge
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

        // 6. Determine operation type for audit
        let operation = determine_operation(&self.changes);

        // 7. Calculate modification results (handle split scenario)
        let results = calculate_modification_results(item, affected_qty, &self.changes);

        // 8. Allocate sequence number
        let seq = ctx.next_sequence();

        // 9. Create event
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::ItemModified,
            EventPayload::ItemModified {
                operation: operation.to_string(),
                source: Box::new(item.clone()),
                affected_quantity: affected_qty,
                changes: self.changes.clone(),
                previous_values,
                results,
                authorizer_id: self.authorizer_id.clone(),
                authorizer_name: self.authorizer_name.clone(),
            },
        );

        Ok(vec![event])
    }
}

/// Determine the operation type based on changes
fn determine_operation(changes: &ItemChanges) -> &'static str {
    if changes.manual_discount_percent.is_some() {
        "APPLY_DISCOUNT"
    } else if changes.price.is_some() {
        "MODIFY_PRICE"
    } else if changes.quantity.is_some() {
        "MODIFY_QUANTITY"
    } else if changes.surcharge.is_some() {
        "APPLY_SURCHARGE"
    } else if changes.selected_options.is_some() || changes.selected_specification.is_some() {
        "MODIFY_OPTIONS"
    } else if changes.note.is_some() {
        "MODIFY_NOTE"
    } else {
        "MODIFY_ITEM"
    }
}

/// Calculate modification results, handling split scenario
fn calculate_modification_results(
    item: &CartItemSnapshot,
    affected_qty: i32,
    changes: &ItemChanges,
) -> Vec<ItemModificationResult> {
    if affected_qty >= item.quantity {
        // Full modification: update entire item in place
        vec![ItemModificationResult {
            instance_id: item.instance_id.clone(),
            quantity: item.quantity,
            price: changes.price.unwrap_or(item.price),
            manual_discount_percent: changes.manual_discount_percent.or(item.manual_discount_percent),
            action: "UPDATED".to_string(),
        }]
    } else {
        // Partial modification: split into unchanged + modified portions
        let new_price = changes.price.unwrap_or(item.price);
        let new_discount = changes.manual_discount_percent.or(item.manual_discount_percent);
        let new_options = changes
            .selected_options
            .as_ref()
            .or(item.selected_options.as_ref());
        let new_specification = changes
            .selected_specification
            .as_ref()
            .or(item.selected_specification.as_ref());

        // Generate new instance_id for the modified portion
        let new_instance_id = generate_instance_id_from_parts(
            &item.id,
            new_price,
            new_discount,
            &new_options.cloned(),
            &new_specification.cloned(),
        );

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
            operator_id: "user-1".to_string(),
            operator_name: "Test User".to_string(),
            timestamp: 1234567890,
        }
    }

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
            note: None,
            authorizer_id: None,
            authorizer_name: None,
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
        let item = create_test_item("item-1", "prod-1", "Test Product", 10.0, 2);
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
        let item = create_test_item("item-1", "prod-1", "Test Product", 10.0, 5);
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
            authorizer_id: Some("manager-1".to_string()),
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

            assert_eq!(authorizer_id.as_deref(), Some("manager-1"));
            assert_eq!(authorizer_name.as_deref(), Some("Manager"));
        } else {
            panic!("Expected ItemModified payload");
        }
    }

    #[tokio::test]
    async fn test_modify_item_apply_discount() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", "prod-1", "Test Product", 100.0, 1);
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

        let item = create_test_item("item-1", "prod-1", "Test Product", 10.0, 1);
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

        let item = create_test_item("item-1", "prod-1", "Test Product", 10.0, 3);
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

        let item = create_test_item("item-1", "prod-1", "Test Product", 10.0, 1);
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

        let item = create_test_item("item-1", "prod-1", "Test Product", 10.0, 1);
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

        let item = create_test_item("item-1", "prod-1", "Test Product", 10.0, 3);
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
    async fn test_modify_item_surcharge() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", "prod-1", "Test Product", 10.0, 1);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ModifyItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            affected_quantity: None,
            changes: ItemChanges {
                surcharge: Some(5.0),
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
            assert_eq!(operation, "APPLY_SURCHARGE");
            assert_eq!(changes.surcharge, Some(5.0));
            assert_eq!(previous_values.surcharge, None);
        } else {
            panic!("Expected ItemModified payload");
        }
    }

    #[tokio::test]
    async fn test_modify_item_note() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", "prod-1", "Test Product", 10.0, 1);
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

        // Surcharge
        assert_eq!(
            determine_operation(&ItemChanges {
                surcharge: Some(3.0),
                ..Default::default()
            }),
            "APPLY_SURCHARGE"
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
                    id: "spec-1".to_string(),
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

        let item = create_test_item("item-1", "prod-1", "Test Product", 10.0, 1);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let new_options = vec![shared::order::ItemOption {
            attribute_id: "attr-1".to_string(),
            attribute_name: "Size".to_string(),
            option_idx: 1,
            option_name: "Large".to_string(),
            price_modifier: Some(2.0),
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

        let item = create_test_item("item-1", "prod-1", "Test Product", 10.0, 1);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let new_spec = shared::order::SpecificationInfo {
            id: "spec-1".to_string(),
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
}
