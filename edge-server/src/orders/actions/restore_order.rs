//! RestoreOrder command handler
//!
//! Restores a voided order back to Active status.

use async_trait::async_trait;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// RestoreOrder action
#[derive(Debug, Clone)]
pub struct RestoreOrderAction {
    pub order_id: String,
}

#[async_trait]
impl CommandHandler for RestoreOrderAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Load existing snapshot
        let snapshot = ctx.load_snapshot(&self.order_id)?;

        // 2. Validate order status (must be Void)
        if snapshot.status != OrderStatus::Void {
            return Err(OrderError::InvalidOperation(format!(
                "Can only restore voided orders, current status: {:?}",
                snapshot.status
            )));
        }

        // 3. Allocate sequence number
        let seq = ctx.next_sequence();

        // 4. Create event
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::OrderRestored,
            EventPayload::OrderRestored {},
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

    #[tokio::test]
    async fn test_restore_order_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create a voided order
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RestoreOrderAction {
            order_id: "order-1".to_string(),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.order_id, "order-1");
        assert_eq!(event.event_type, OrderEventType::OrderRestored);

        if let EventPayload::OrderRestored {} = &event.payload {
            // Success
        } else {
            panic!("Expected OrderRestored payload");
        }
    }

    #[tokio::test]
    async fn test_restore_active_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RestoreOrderAction {
            order_id: "order-1".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
        if let Err(OrderError::InvalidOperation(msg)) = result {
            assert!(msg.contains("Can only restore voided orders"));
        }
    }

    #[tokio::test]
    async fn test_restore_completed_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RestoreOrderAction {
            order_id: "order-1".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_restore_nonexistent_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RestoreOrderAction {
            order_id: "nonexistent".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    #[tokio::test]
    async fn test_restore_moved_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Moved;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RestoreOrderAction {
            order_id: "order-1".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_restore_merged_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Merged;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RestoreOrderAction {
            order_id: "order-1".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_restore_order_preserves_data() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create a voided order with data
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        snapshot.table_id = Some("dining_table:t1".to_string());
        snapshot.table_name = Some("Table 1".to_string());
        snapshot.total = 150.0;
        snapshot.paid_amount = 50.0;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RestoreOrderAction {
            order_id: "order-1".to_string(),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        // Event generated successfully - data preservation is handled by applier
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, OrderEventType::OrderRestored);
    }

    #[tokio::test]
    async fn test_restore_order_event_metadata() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RestoreOrderAction {
            order_id: "order-1".to_string(),
        };

        let metadata = CommandMetadata {
            command_id: "cmd-restore-1".to_string(),
            operator_id: "manager-1".to_string(),
            operator_name: "Manager".to_string(),
            timestamp: 9999999999,
        };

        let events = action.execute(&mut ctx, &metadata).await.unwrap();
        let event = &events[0];

        // Verify event metadata
        assert_eq!(event.command_id, "cmd-restore-1");
        assert_eq!(event.operator_id, "manager-1");
        assert_eq!(event.operator_name, "Manager");
        assert_eq!(event.client_timestamp, Some(9999999999));
    }
}
