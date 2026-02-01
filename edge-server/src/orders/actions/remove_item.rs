//! RemoveItem command handler
//!
//! Removes an item from an order by marking it as voided.
//! Supports both full removal and partial removal (by quantity).
//!
//! Note: Items are NOT physically deleted - they are marked as voided
//! for audit trail purposes.

use async_trait::async_trait;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// RemoveItem action
#[derive(Debug, Clone)]
pub struct RemoveItemAction {
    pub order_id: String,
    pub instance_id: String,
    pub quantity: Option<i32>,
    pub reason: Option<String>,
    pub authorizer_id: Option<String>,
    pub authorizer_name: Option<String>,
}

#[async_trait]
impl CommandHandler for RemoveItemAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Load existing snapshot
        let snapshot = ctx.load_snapshot(&self.order_id)?;

        // 2. Validate order status (must be Active)
        match snapshot.status {
            OrderStatus::Active => {}
            OrderStatus::Completed => {
                return Err(OrderError::OrderAlreadyCompleted(self.order_id.clone()));
            }
            OrderStatus::Void => {
                return Err(OrderError::OrderAlreadyVoided(self.order_id.clone()));
            }
            _ => {
                return Err(OrderError::InvalidOperation(format!(
                    "Cannot remove item from order with status: {:?}",
                    snapshot.status
                )));
            }
        }

        // 3. Find the item
        let item = snapshot
            .items
            .iter()
            .find(|i| i.instance_id == self.instance_id)
            .ok_or_else(|| OrderError::ItemNotFound(self.instance_id.clone()))?;

        // 4. Validate quantity (if specified)
        if let Some(qty) = self.quantity {
            if qty <= 0 {
                return Err(OrderError::InvalidOperation(
                    "quantity must be positive".to_string(),
                ));
            }
            if qty > item.quantity {
                return Err(OrderError::InsufficientQuantity);
            }
        }

        // 5. Allocate sequence number
        let seq = ctx.next_sequence();

        // 6. Create event
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::ItemRemoved,
            EventPayload::ItemRemoved {
                instance_id: self.instance_id.clone(),
                item_name: item.name.clone(),
                quantity: self.quantity,
                reason: self.reason.clone(),
                authorizer_id: self.authorizer_id.clone(),
                authorizer_name: self.authorizer_name.clone(),
            },
        );

        Ok(vec![event])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orders::storage::OrderStorage;
    use crate::orders::traits::CommandContext;
    use shared::order::{CartItemSnapshot, OrderSnapshot};

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

    fn create_active_order_with_item(order_id: &str, item: CartItemSnapshot) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items.push(item);
        snapshot
    }

    #[tokio::test]
    async fn test_remove_item_full() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create order with item
        let item = create_test_item("item-1", "product:p1", "Test Product", 10.0, 2);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RemoveItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            quantity: None, // Full removal
            reason: Some("Customer changed mind".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.order_id, "order-1");
        assert_eq!(event.event_type, OrderEventType::ItemRemoved);

        if let EventPayload::ItemRemoved {
            instance_id,
            item_name,
            quantity,
            reason,
            ..
        } = &event.payload
        {
            assert_eq!(instance_id, "item-1");
            assert_eq!(item_name, "Test Product");
            assert_eq!(*quantity, None);
            assert_eq!(reason.as_deref(), Some("Customer changed mind"));
        } else {
            panic!("Expected ItemRemoved payload");
        }
    }

    #[tokio::test]
    async fn test_remove_item_partial() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create order with item quantity=5
        let item = create_test_item("item-1", "product:p1", "Test Product", 10.0, 5);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RemoveItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            quantity: Some(2), // Partial: only remove 2 of 5
            reason: None,
            authorizer_id: Some("manager-1".to_string()),
            authorizer_name: Some("Manager".to_string()),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::ItemRemoved {
            instance_id,
            quantity,
            authorizer_id,
            authorizer_name,
            ..
        } = &events[0].payload
        {
            assert_eq!(instance_id, "item-1");
            assert_eq!(*quantity, Some(2));
            assert_eq!(authorizer_id.as_deref(), Some("manager-1"));
            assert_eq!(authorizer_name.as_deref(), Some("Manager"));
        } else {
            panic!("Expected ItemRemoved payload");
        }
    }

    #[tokio::test]
    async fn test_remove_item_not_found() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", "product:p1", "Test Product", 10.0, 1);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RemoveItemAction {
            order_id: "order-1".to_string(),
            instance_id: "nonexistent".to_string(),
            quantity: None,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::ItemNotFound(_))));
    }

    #[tokio::test]
    async fn test_remove_item_insufficient_quantity() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", "product:p1", "Test Product", 10.0, 3);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RemoveItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            quantity: Some(5), // More than available (3)
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InsufficientQuantity)));
    }

    #[tokio::test]
    async fn test_remove_item_zero_quantity_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", "product:p1", "Test Product", 10.0, 3);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RemoveItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            quantity: Some(0),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_remove_item_negative_quantity_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", "product:p1", "Test Product", 10.0, 3);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RemoveItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            quantity: Some(-1),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_remove_item_completed_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", "product:p1", "Test Product", 10.0, 1);
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        snapshot.items.push(item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RemoveItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            quantity: None,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[tokio::test]
    async fn test_remove_item_voided_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", "product:p1", "Test Product", 10.0, 1);
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        snapshot.items.push(item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RemoveItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            quantity: None,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyVoided(_))));
    }

    #[tokio::test]
    async fn test_remove_item_order_not_found() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RemoveItemAction {
            order_id: "nonexistent-order".to_string(),
            instance_id: "item-1".to_string(),
            quantity: None,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    #[tokio::test]
    async fn test_remove_item_with_all_fields() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", "product:p1", "Expensive Wine", 150.0, 1);
        let snapshot = create_active_order_with_item("order-1", item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RemoveItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            quantity: None,
            reason: Some("Wrong order".to_string()),
            authorizer_id: Some("manager-1".to_string()),
            authorizer_name: Some("Floor Manager".to_string()),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::ItemRemoved {
            instance_id,
            item_name,
            quantity,
            reason,
            authorizer_id,
            authorizer_name,
        } = &events[0].payload
        {
            assert_eq!(instance_id, "item-1");
            assert_eq!(item_name, "Expensive Wine");
            assert_eq!(*quantity, None);
            assert_eq!(reason.as_deref(), Some("Wrong order"));
            assert_eq!(authorizer_id.as_deref(), Some("manager-1"));
            assert_eq!(authorizer_name.as_deref(), Some("Floor Manager"));
        } else {
            panic!("Expected ItemRemoved payload");
        }
    }
}
