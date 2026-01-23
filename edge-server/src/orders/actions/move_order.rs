//! MoveOrder command handler
//!
//! Moves an order from one table to another, optionally changing zone.
//! Only applicable to orders in Active status.

use async_trait::async_trait;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// MoveOrder action
#[derive(Debug, Clone)]
pub struct MoveOrderAction {
    pub order_id: String,
    pub target_table_id: String,
    pub target_table_name: String,
    pub target_zone_id: Option<String>,
    pub target_zone_name: Option<String>,
}

#[async_trait]
impl CommandHandler for MoveOrderAction {
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

        // 3. Validate target table is not occupied by another order
        if let Some(existing_order_id) = ctx.find_active_order_for_table(&self.target_table_id)? {
            // Allow moving to the same table (no-op case)
            if existing_order_id != self.order_id {
                return Err(OrderError::TableOccupied(format!(
                    "目标桌台 {} 已被占用 (订单: {})",
                    self.target_table_name, existing_order_id
                )));
            }
        }

        // 4. Get source table info from snapshot
        let source_table_id = snapshot.table_id.clone().unwrap_or_default();
        let source_table_name = snapshot.table_name.clone().unwrap_or_default();

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
            OrderEventType::OrderMoved,
            EventPayload::OrderMoved {
                source_table_id,
                source_table_name,
                target_table_id: self.target_table_id.clone(),
                target_table_name: self.target_table_name.clone(),
                items: snapshot.items.clone(),
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

    fn create_active_order(order_id: &str) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.table_id = Some("table-1".to_string());
        snapshot.table_name = Some("Table 1".to_string());
        snapshot.zone_id = Some("zone-1".to_string());
        snapshot.zone_name = Some("Zone A".to_string());
        snapshot
    }

    #[tokio::test]
    async fn test_move_order_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MoveOrderAction {
            order_id: "order-1".to_string(),
            target_table_id: "table-2".to_string(),
            target_table_name: "Table 2".to_string(),
            target_zone_id: Some("zone-2".to_string()),
            target_zone_name: Some("Zone B".to_string()),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.order_id, "order-1");
        assert_eq!(event.event_type, OrderEventType::OrderMoved);

        if let EventPayload::OrderMoved {
            source_table_id,
            source_table_name,
            target_table_id,
            target_table_name,
            items,
        } = &event.payload
        {
            assert_eq!(source_table_id, "table-1");
            assert_eq!(source_table_name, "Table 1");
            assert_eq!(target_table_id, "table-2");
            assert_eq!(target_table_name, "Table 2");
            assert!(items.is_empty());
        } else {
            panic!("Expected OrderMoved payload");
        }
    }

    #[tokio::test]
    async fn test_move_order_with_items() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_order("order-1");
        let item = CartItemSnapshot {
            id: "product-1".to_string(),
            instance_id: "item-1".to_string(),
            name: "Coffee".to_string(),
            price: 10.0,
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
            line_total: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        };
        snapshot.items.push(item);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MoveOrderAction {
            order_id: "order-1".to_string(),
            target_table_id: "table-3".to_string(),
            target_table_name: "Table 3".to_string(),
            target_zone_id: None,
            target_zone_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::OrderMoved { items, .. } = &events[0].payload {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].instance_id, "item-1");
            assert_eq!(items[0].name, "Coffee");
        } else {
            panic!("Expected OrderMoved payload");
        }
    }

    #[tokio::test]
    async fn test_move_order_completed_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MoveOrderAction {
            order_id: "order-1".to_string(),
            target_table_id: "table-2".to_string(),
            target_table_name: "Table 2".to_string(),
            target_zone_id: None,
            target_zone_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[tokio::test]
    async fn test_move_order_voided_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MoveOrderAction {
            order_id: "order-1".to_string(),
            target_table_id: "table-2".to_string(),
            target_table_name: "Table 2".to_string(),
            target_zone_id: None,
            target_zone_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyVoided(_))));
    }

    #[tokio::test]
    async fn test_move_order_not_found_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MoveOrderAction {
            order_id: "nonexistent".to_string(),
            target_table_id: "table-2".to_string(),
            target_table_name: "Table 2".to_string(),
            target_zone_id: None,
            target_zone_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    #[tokio::test]
    async fn test_move_order_sequence_allocation() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MoveOrderAction {
            order_id: "order-1".to_string(),
            target_table_id: "table-2".to_string(),
            target_table_name: "Table 2".to_string(),
            target_zone_id: None,
            target_zone_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events[0].sequence, current_seq + 1);
    }

    #[tokio::test]
    async fn test_move_order_metadata_propagation() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MoveOrderAction {
            order_id: "order-1".to_string(),
            target_table_id: "table-2".to_string(),
            target_table_name: "Table 2".to_string(),
            target_zone_id: None,
            target_zone_name: None,
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
    async fn test_move_order_without_source_table() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        // No table_id or table_name set
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MoveOrderAction {
            order_id: "order-1".to_string(),
            target_table_id: "table-2".to_string(),
            target_table_name: "Table 2".to_string(),
            target_zone_id: None,
            target_zone_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::OrderMoved {
            source_table_id,
            source_table_name,
            ..
        } = &events[0].payload
        {
            // Default to empty string when source table info is None
            assert_eq!(source_table_id, "");
            assert_eq!(source_table_name, "");
        } else {
            panic!("Expected OrderMoved payload");
        }
    }

    #[tokio::test]
    async fn test_move_order_target_table_occupied_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create order-1 at table-1
        let mut snapshot1 = OrderSnapshot::new("order-1".to_string());
        snapshot1.status = OrderStatus::Active;
        snapshot1.table_id = Some("table-1".to_string());
        snapshot1.table_name = Some("Table 1".to_string());
        storage.store_snapshot(&txn, &snapshot1).unwrap();
        storage.mark_order_active(&txn, "order-1").unwrap();

        // Create order-2 at table-2
        let mut snapshot2 = OrderSnapshot::new("order-2".to_string());
        snapshot2.status = OrderStatus::Active;
        snapshot2.table_id = Some("table-2".to_string());
        snapshot2.table_name = Some("Table 2".to_string());
        storage.store_snapshot(&txn, &snapshot2).unwrap();
        storage.mark_order_active(&txn, "order-2").unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Try to move order-1 to table-2 (which is occupied by order-2)
        let action = MoveOrderAction {
            order_id: "order-1".to_string(),
            target_table_id: "table-2".to_string(),
            target_table_name: "Table 2".to_string(),
            target_zone_id: None,
            target_zone_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::TableOccupied(_))));
    }

    #[tokio::test]
    async fn test_move_order_to_same_table_succeeds() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create order-1 at table-1
        let mut snapshot1 = OrderSnapshot::new("order-1".to_string());
        snapshot1.status = OrderStatus::Active;
        snapshot1.table_id = Some("table-1".to_string());
        snapshot1.table_name = Some("Table 1".to_string());
        storage.store_snapshot(&txn, &snapshot1).unwrap();
        storage.mark_order_active(&txn, "order-1").unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Move order-1 to table-1 (same table - should succeed as a no-op)
        let action = MoveOrderAction {
            order_id: "order-1".to_string(),
            target_table_id: "table-1".to_string(),
            target_table_name: "Table 1".to_string(),
            target_zone_id: None,
            target_zone_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(result.is_ok());
    }
}
