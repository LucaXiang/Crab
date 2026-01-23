//! SplitOrder command handler
//!
//! Splits an order by creating a partial payment for selected items.
//! Tracks which items have been paid and updates the paid amount.
//!
//! Two modes:
//! - **Item-based split (菜品分单)**: `split_amount` is None, backend calculates from items
//! - **Amount-based split (金额分单)**: `split_amount` is provided by frontend

use async_trait::async_trait;

use crate::orders::money::{calculate_unit_price, to_f64};
use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use rust_decimal::Decimal;
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus, SplitItem};

/// SplitOrder action
#[derive(Debug, Clone)]
pub struct SplitOrderAction {
    pub order_id: String,
    /// Split amount (optional for item-based split, required for amount-based split)
    /// If None and items are provided, backend calculates from items
    pub split_amount: Option<f64>,
    pub payment_method: String,
    pub items: Vec<SplitItem>,
}

#[async_trait]
impl CommandHandler for SplitOrderAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Load existing snapshot
        let snapshot = ctx.load_snapshot(&self.order_id)?;

        // 2. Validate order status - must be Active
        match snapshot.status {
            OrderStatus::Active => {}
            OrderStatus::Completed => {
                return Err(OrderError::OrderAlreadyCompleted(self.order_id.clone()));
            }
            OrderStatus::Void => {
                return Err(OrderError::OrderAlreadyVoided(self.order_id.clone()));
            }
            _ => {
                return Err(OrderError::OrderNotFound(self.order_id.clone()));
            }
        }

        // 3. Validate items exist in order and have sufficient quantity
        // Also calculate split amount from items if not provided
        let mut calculated_amount = Decimal::ZERO;
        for split_item in &self.items {
            let order_item = snapshot
                .items
                .iter()
                .find(|i| i.instance_id == split_item.instance_id)
                .ok_or_else(|| OrderError::ItemNotFound(split_item.instance_id.clone()))?;

            // Check if there's enough unpaid quantity
            let paid_qty = snapshot
                .paid_item_quantities
                .get(&split_item.instance_id)
                .copied()
                .unwrap_or(0);
            let available_qty = order_item.quantity - paid_qty;

            if split_item.quantity > available_qty {
                return Err(OrderError::InsufficientQuantity);
            }

            // Calculate amount for this item: unit_price * split_quantity
            let unit_price = calculate_unit_price(order_item);
            calculated_amount += unit_price * Decimal::from(split_item.quantity);
        }

        // 4. Determine final split amount
        let final_split_amount = if let Some(amount) = self.split_amount {
            // Amount-based split: use provided amount
            if amount <= 0.0 {
                return Err(OrderError::InvalidAmount);
            }
            amount
        } else if !self.items.is_empty() {
            // Item-based split: use calculated amount
            let amount = to_f64(calculated_amount);
            if amount <= 0.0 {
                return Err(OrderError::InvalidAmount);
            }
            amount
        } else {
            // No amount and no items
            return Err(OrderError::InvalidAmount);
        };

        // 5. Validate: cannot overpay (split_amount <= remaining unpaid)
        let remaining_unpaid = snapshot.total - snapshot.paid_amount;
        if final_split_amount > remaining_unpaid + 0.01 {
            // Allow small tolerance for rounding
            return Err(OrderError::InvalidOperation(format!(
                "Split amount ({:.2}) exceeds remaining unpaid ({:.2})",
                final_split_amount, remaining_unpaid
            )));
        }

        // 6. Allocate sequence number
        let seq = ctx.next_sequence();

        // 7. Create event
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::OrderSplit,
            EventPayload::OrderSplit {
                split_amount: final_split_amount,
                payment_method: self.payment_method.clone(),
                items: self.items.clone(),
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

    fn create_active_order_with_items(order_id: &str) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.table_id = Some("dining_table:t1".to_string());
        snapshot.table_name = Some("Table 1".to_string());

        // Add items
        let item1 = CartItemSnapshot {
            id: "product:1".to_string(),
            instance_id: "item-1".to_string(),
            name: "Coffee".to_string(),
            price: 10.0,
            original_price: None,
            quantity: 3,
            unpaid_quantity: 3,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            surcharge: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            unit_price: None,
            line_total: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        };
        let item2 = CartItemSnapshot {
            id: "product:2".to_string(),
            instance_id: "item-2".to_string(),
            name: "Tea".to_string(),
            price: 8.0,
            original_price: None,
            quantity: 2,
            unpaid_quantity: 2,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            surcharge: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            unit_price: None,
            line_total: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        };
        snapshot.items.push(item1);
        snapshot.items.push(item2);
        snapshot.subtotal = 46.0; // 3*10 + 2*8
        snapshot.total = 46.0;

        snapshot
    }

    #[tokio::test]
    async fn test_split_order_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order_with_items("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = SplitOrderAction {
            order_id: "order-1".to_string(),
            split_amount: Some(20.0),
            payment_method: "cash".to_string(),
            items: vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 2,
            }],
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.order_id, "order-1");
        assert_eq!(event.event_type, OrderEventType::OrderSplit);

        if let EventPayload::OrderSplit {
            split_amount,
            payment_method,
            items,
        } = &event.payload
        {
            assert_eq!(*split_amount, 20.0);
            assert_eq!(payment_method, "cash");
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].instance_id, "item-1");
            assert_eq!(items[0].quantity, 2);
        } else {
            panic!("Expected OrderSplit payload");
        }
    }

    #[tokio::test]
    async fn test_split_order_multiple_items() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order_with_items("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = SplitOrderAction {
            order_id: "order-1".to_string(),
            split_amount: Some(28.0),
            payment_method: "card".to_string(),
            items: vec![
                SplitItem {
                    instance_id: "item-1".to_string(),
                    name: "Coffee".to_string(),
                    quantity: 2,
                },
                SplitItem {
                    instance_id: "item-2".to_string(),
                    name: "Tea".to_string(),
                    quantity: 1,
                },
            ],
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::OrderSplit { items, .. } = &events[0].payload {
            assert_eq!(items.len(), 2);
        } else {
            panic!("Expected OrderSplit payload");
        }
    }

    #[tokio::test]
    async fn test_split_order_invalid_amount_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order_with_items("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = SplitOrderAction {
            order_id: "order-1".to_string(),
            split_amount: Some(0.0), // Invalid
            payment_method: "cash".to_string(),
            items: vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 1,
            }],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidAmount)));
    }

    #[tokio::test]
    async fn test_split_order_negative_amount_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order_with_items("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = SplitOrderAction {
            order_id: "order-1".to_string(),
            split_amount: Some(-10.0), // Negative
            payment_method: "cash".to_string(),
            items: vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 1,
            }],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidAmount)));
    }

    #[tokio::test]
    async fn test_split_order_item_not_found_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order_with_items("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = SplitOrderAction {
            order_id: "order-1".to_string(),
            split_amount: Some(10.0),
            payment_method: "cash".to_string(),
            items: vec![SplitItem {
                instance_id: "nonexistent".to_string(),
                name: "Unknown".to_string(),
                quantity: 1,
            }],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::ItemNotFound(_))));
    }

    #[tokio::test]
    async fn test_split_order_insufficient_quantity_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order_with_items("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = SplitOrderAction {
            order_id: "order-1".to_string(),
            split_amount: Some(50.0),
            payment_method: "cash".to_string(),
            items: vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 5, // Only 3 available
            }],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InsufficientQuantity)));
    }

    #[tokio::test]
    async fn test_split_order_respects_paid_quantities() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_order_with_items("order-1");
        // Mark 2 of item-1 as already paid
        snapshot
            .paid_item_quantities
            .insert("item-1".to_string(), 2);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Try to split 2 more of item-1, but only 1 is unpaid
        let action = SplitOrderAction {
            order_id: "order-1".to_string(),
            split_amount: Some(20.0),
            payment_method: "cash".to_string(),
            items: vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 2, // Only 1 available (3 total - 2 paid)
            }],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InsufficientQuantity)));

        // But splitting 1 should work
        let action_valid = SplitOrderAction {
            order_id: "order-1".to_string(),
            split_amount: Some(10.0),
            payment_method: "cash".to_string(),
            items: vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 1, // Exactly 1 available
            }],
        };

        let mut ctx2 = CommandContext::new(&txn, &storage, current_seq);
        let events = action_valid.execute(&mut ctx2, &metadata).await.unwrap();
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_split_order_completed_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_order_with_items("order-1");
        snapshot.status = OrderStatus::Completed;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = SplitOrderAction {
            order_id: "order-1".to_string(),
            split_amount: Some(10.0),
            payment_method: "cash".to_string(),
            items: vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 1,
            }],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[tokio::test]
    async fn test_split_order_voided_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_order_with_items("order-1");
        snapshot.status = OrderStatus::Void;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = SplitOrderAction {
            order_id: "order-1".to_string(),
            split_amount: Some(10.0),
            payment_method: "cash".to_string(),
            items: vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 1,
            }],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyVoided(_))));
    }

    #[tokio::test]
    async fn test_split_order_not_found_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = SplitOrderAction {
            order_id: "nonexistent".to_string(),
            split_amount: Some(10.0),
            payment_method: "cash".to_string(),
            items: vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 1,
            }],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    #[tokio::test]
    async fn test_split_order_sequence_allocation() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order_with_items("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = SplitOrderAction {
            order_id: "order-1".to_string(),
            split_amount: Some(10.0),
            payment_method: "cash".to_string(),
            items: vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 1,
            }],
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events[0].sequence, current_seq + 1);
    }

    #[tokio::test]
    async fn test_split_order_metadata_propagation() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order_with_items("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = SplitOrderAction {
            order_id: "order-1".to_string(),
            split_amount: Some(10.0),
            payment_method: "cash".to_string(),
            items: vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 1,
            }],
        };

        let metadata = CommandMetadata {
            command_id: "test-cmd-123".to_string(),
            operator_id: "operator-456".to_string(),
            operator_name: "John Doe".to_string(),
            timestamp: 9999999999,
        };

        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events[0].command_id, "test-cmd-123");
        assert_eq!(events[0].operator_id, "operator-456");
        assert_eq!(events[0].operator_name, "John Doe");
    }

    #[tokio::test]
    async fn test_split_order_empty_items() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order_with_items("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Split with empty items is valid (payment without item tracking)
        let action = SplitOrderAction {
            order_id: "order-1".to_string(),
            split_amount: Some(10.0),
            payment_method: "cash".to_string(),
            items: vec![],
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::OrderSplit { items, .. } = &events[0].payload {
            assert!(items.is_empty());
        } else {
            panic!("Expected OrderSplit payload");
        }
    }
}
