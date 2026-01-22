//! AddItems command handler
//!
//! Adds items to an existing order.

use async_trait::async_trait;

use crate::db::models::PriceRule;
use crate::orders::reducer::input_to_snapshot_with_rules;
use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{CartItemInput, EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// AddItems action
#[derive(Debug, Clone)]
pub struct AddItemsAction {
    pub order_id: String,
    pub items: Vec<CartItemInput>,
    /// Matched price rules for this order (from cache)
    pub rules: Vec<PriceRule>,
}

#[async_trait]
impl CommandHandler for AddItemsAction {
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

        // 3. Allocate sequence number
        let seq = ctx.next_sequence();

        // 4. Convert inputs to snapshots with generated instance_ids and price rules applied
        let rules_refs: Vec<&PriceRule> = self.rules.iter().collect();
        let item_snapshots: Vec<_> = self
            .items
            .iter()
            .map(|item| input_to_snapshot_with_rules(item, &rules_refs))
            .collect();

        // 5. Create event
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::ItemsAdded,
            EventPayload::ItemsAdded {
                items: item_snapshots,
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
    use shared::order::OrderSnapshot;

    fn create_test_metadata() -> CommandMetadata {
        CommandMetadata {
            command_id: "cmd-1".to_string(),
            operator_id: "user-1".to_string(),
            operator_name: "Test User".to_string(),
            timestamp: 1234567890,
        }
    }

    fn create_cart_item_input(
        product_id: &str,
        name: &str,
        price: f64,
        quantity: i32,
    ) -> CartItemInput {
        CartItemInput {
            product_id: product_id.to_string(),
            name: name.to_string(),
            price,
            original_price: None,
            quantity,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            surcharge: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        }
    }

    #[tokio::test]
    async fn test_add_items_generates_event() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        // Create and store an active order
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddItemsAction {
            order_id: "order-1".to_string(),
            items: vec![create_cart_item_input("prod-1", "Test Product", 10.0, 2)],
            rules: vec![],
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.order_id, "order-1");
        assert_eq!(event.event_type, OrderEventType::ItemsAdded);

        if let EventPayload::ItemsAdded { items } = &event.payload {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].id, "prod-1");
            assert_eq!(items[0].name, "Test Product");
            assert_eq!(items[0].price, 10.0);
            assert_eq!(items[0].quantity, 2);
            assert!(!items[0].instance_id.is_empty());
        } else {
            panic!("Expected ItemsAdded payload");
        }
    }

    #[tokio::test]
    async fn test_add_items_to_completed_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        // Create a completed order
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddItemsAction {
            order_id: "order-1".to_string(),
            items: vec![create_cart_item_input("prod-1", "Test", 10.0, 1)],
            rules: vec![],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[tokio::test]
    async fn test_add_items_to_voided_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        // Create a voided order
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddItemsAction {
            order_id: "order-1".to_string(),
            items: vec![create_cart_item_input("prod-1", "Test", 10.0, 1)],
            rules: vec![],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyVoided(_))));
    }

    #[tokio::test]
    async fn test_add_items_to_nonexistent_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddItemsAction {
            order_id: "nonexistent".to_string(),
            items: vec![create_cart_item_input("prod-1", "Test", 10.0, 1)],
            rules: vec![],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    #[tokio::test]
    async fn test_add_multiple_items() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddItemsAction {
            order_id: "order-1".to_string(),
            items: vec![
                create_cart_item_input("prod-1", "Product A", 10.0, 2),
                create_cart_item_input("prod-2", "Product B", 15.0, 1),
                create_cart_item_input("prod-3", "Product C", 5.0, 3),
            ],
            rules: vec![],
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::ItemsAdded { items } = &events[0].payload {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].id, "prod-1");
            assert_eq!(items[1].id, "prod-2");
            assert_eq!(items[2].id, "prod-3");
        } else {
            panic!("Expected ItemsAdded payload");
        }
    }
}
