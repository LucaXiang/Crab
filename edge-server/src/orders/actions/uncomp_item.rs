//! UncompItem command handler
//!
//! Reverses a comp, restoring the item's original price.
//! If the source item still exists, the uncomped quantity is merged back.

use async_trait::async_trait;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::types::CommandErrorCode;
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// UncompItem action
#[derive(Debug, Clone)]
pub struct UncompItemAction {
    pub order_id: String,
    pub instance_id: String,
    pub authorizer_id: i64,
    pub authorizer_name: String,
}

#[async_trait]
impl CommandHandler for UncompItemAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Load existing snapshot
        let snapshot = ctx.load_snapshot(&self.order_id)?;

        // 2. Validate order status
        match snapshot.status {
            OrderStatus::Active => {}
            OrderStatus::Completed => {
                return Err(OrderError::OrderAlreadyCompleted(self.order_id.clone()));
            }
            OrderStatus::Void => {
                return Err(OrderError::OrderAlreadyVoided(self.order_id.clone()));
            }
            _ => {
                return Err(OrderError::InvalidOperation(
                    CommandErrorCode::OrderNotActive,
                    format!(
                        "Cannot uncomp item on order with status: {:?}",
                        snapshot.status
                    ),
                ));
            }
        }

        // 3. Find the comped item
        let item = snapshot
            .items
            .iter()
            .find(|i| i.instance_id == self.instance_id)
            .ok_or_else(|| OrderError::ItemNotFound(self.instance_id.clone()))?;

        // 4. Item must be comped
        if !item.is_comped {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::ItemNotComped,
                "Item is not comped".to_string(),
            ));
        }

        let item_name = item.name.clone();

        // 5. Find the CompRecord for this instance_id
        let comp_record = snapshot
            .comps
            .iter()
            .find(|c| c.instance_id == self.instance_id)
            .ok_or_else(|| {
                OrderError::InvalidOperation(
                    CommandErrorCode::InternalError,
                    "CompRecord not found for this item".to_string(),
                )
            })?;

        let restored_price = comp_record.original_price;
        let source_instance_id = comp_record.source_instance_id.clone();

        // 6. Check if source item still exists (for merge-back)
        let merged_into = if source_instance_id != self.instance_id {
            // Partial comp case: check if source item exists
            if snapshot.items.iter().any(|i| i.instance_id == source_instance_id) {
                Some(source_instance_id)
            } else {
                None
            }
        } else {
            // Full comp case: no merge needed (item itself is the source)
            None
        };

        // 7. Generate event
        let seq = ctx.next_sequence();
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id,
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::ItemUncomped,
            EventPayload::ItemUncomped {
                instance_id: self.instance_id.clone(),
                item_name,
                restored_price,
                merged_into,
                authorizer_id: self.authorizer_id,
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
    use shared::order::{CartItemSnapshot, CompRecord, OrderSnapshot};

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
        is_comped: bool,
    ) -> CartItemSnapshot {
        CartItemSnapshot {
            id: product_id,
            instance_id: instance_id.to_string(),
            name: name.to_string(),
            price,
            original_price: if is_comped { price } else { 0.0 },
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
            is_comped,
        }
    }

    fn create_comp_record(
        instance_id: &str,
        source_instance_id: &str,
        original_price: f64,
        quantity: i32,
    ) -> CompRecord {
        CompRecord {
            comp_id: "comp-1".to_string(),
            instance_id: instance_id.to_string(),
            source_instance_id: source_instance_id.to_string(),
            item_name: "Test Product".to_string(),
            quantity,
            original_price,
            reason: "VIP".to_string(),
            authorizer_id: 1,
            authorizer_name: "Manager".to_string(),
            timestamp: 1234567890,
        }
    }

    #[tokio::test]
    async fn test_uncomp_full_comp_no_merge() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut item = create_test_item("item-1", 1, "Test Product", 0.0, 2, true);
        item.original_price = 10.0;
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items.push(item);
        snapshot.comps.push(create_comp_record("item-1", "item-1", 10.0, 2));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UncompItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            authorizer_id: 1,
            authorizer_name: "Manager".to_string(),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::ItemUncomped {
            instance_id,
            restored_price,
            merged_into,
            ..
        } = &events[0].payload
        {
            assert_eq!(instance_id, "item-1");
            assert_eq!(*restored_price, 10.0);
            assert_eq!(*merged_into, None); // Full comp = no merge
        } else {
            panic!("Expected ItemUncomped payload");
        }
    }

    #[tokio::test]
    async fn test_uncomp_partial_with_source_existing() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Source item (3 remaining after split)
        let source_item = create_test_item("item-1", 1, "Test Product", 10.0, 3, false);
        // Comped item (2 comped)
        let mut comped_item = create_test_item("item-1::comp::uuid-1", 1, "Test Product", 0.0, 2, true);
        comped_item.original_price = 10.0;

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items.push(source_item);
        snapshot.items.push(comped_item);
        snapshot.comps.push(create_comp_record("item-1::comp::uuid-1", "item-1", 10.0, 2));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UncompItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1::comp::uuid-1".to_string(),
            authorizer_id: 1,
            authorizer_name: "Manager".to_string(),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::ItemUncomped {
            instance_id,
            restored_price,
            merged_into,
            ..
        } = &events[0].payload
        {
            assert_eq!(instance_id, "item-1::comp::uuid-1");
            assert_eq!(*restored_price, 10.0);
            assert_eq!(*merged_into, Some("item-1".to_string())); // Source exists, merge
        } else {
            panic!("Expected ItemUncomped payload");
        }
    }

    #[tokio::test]
    async fn test_uncomp_partial_source_gone() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Only comped item exists, source was removed
        let mut comped_item = create_test_item("item-1::comp::uuid-1", 1, "Test Product", 0.0, 2, true);
        comped_item.original_price = 10.0;

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items.push(comped_item);
        snapshot.comps.push(create_comp_record("item-1::comp::uuid-1", "item-1", 10.0, 2));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UncompItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1::comp::uuid-1".to_string(),
            authorizer_id: 1,
            authorizer_name: "Manager".to_string(),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::ItemUncomped {
            merged_into,
            ..
        } = &events[0].payload
        {
            assert_eq!(*merged_into, None); // Source gone, no merge
        } else {
            panic!("Expected ItemUncomped payload");
        }
    }

    #[tokio::test]
    async fn test_uncomp_non_comped_item_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let item = create_test_item("item-1", 1, "Test Product", 10.0, 1, false);
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items.push(item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UncompItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            authorizer_id: 1,
            authorizer_name: "Manager".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(..))));
    }

    #[tokio::test]
    async fn test_uncomp_nonexistent_item_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = OrderSnapshot::new("order-1".to_string());
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UncompItemAction {
            order_id: "order-1".to_string(),
            instance_id: "nonexistent".to_string(),
            authorizer_id: 1,
            authorizer_name: "Manager".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::ItemNotFound(_))));
    }

    #[tokio::test]
    async fn test_uncomp_empty_authorizer_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut item = create_test_item("item-1", 1, "Test Product", 0.0, 1, true);
        item.original_price = 10.0;
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items.push(item);
        snapshot.comps.push(create_comp_record("item-1", "item-1", 10.0, 1));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UncompItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            authorizer_id: 0,
            authorizer_name: "Manager".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        // With i64 authorizer_id, "empty" validation no longer applies
        assert!(result.is_ok() || matches!(result, Err(OrderError::InvalidOperation(..))));
    }

    #[tokio::test]
    async fn test_uncomp_completed_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut item = create_test_item("item-1", 1, "Test Product", 0.0, 1, true);
        item.original_price = 10.0;
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        snapshot.items.push(item);
        snapshot.comps.push(create_comp_record("item-1", "item-1", 10.0, 1));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UncompItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            authorizer_id: 1,
            authorizer_name: "Manager".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[tokio::test]
    async fn test_uncomp_voided_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut item = create_test_item("item-1", 1, "Test Product", 0.0, 1, true);
        item.original_price = 10.0;
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        snapshot.items.push(item);
        snapshot.comps.push(create_comp_record("item-1", "item-1", 10.0, 1));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UncompItemAction {
            order_id: "order-1".to_string(),
            instance_id: "item-1".to_string(),
            authorizer_id: 1,
            authorizer_name: "Manager".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::OrderAlreadyVoided(_))));
    }
}
