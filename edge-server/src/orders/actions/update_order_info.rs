//! UpdateOrderInfo command handler
//!
//! Updates order metadata such as guest count, table name, receipt number, etc.
//! Only applicable to orders in Active status.

use async_trait::async_trait;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use crate::utils::validation::{MAX_NAME_LEN, validate_order_optional_text};
use shared::order::types::CommandErrorCode;
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// UpdateOrderInfo action
/// Note: receipt_number is immutable (set at OpenTable), not updatable here
#[derive(Debug, Clone)]
pub struct UpdateOrderInfoAction {
    pub order_id: String,
    pub guest_count: Option<i32>,
    pub table_name: Option<String>,
    pub is_pre_payment: Option<bool>,
}

#[async_trait]
impl CommandHandler for UpdateOrderInfoAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Validate text lengths
        validate_order_optional_text(&self.table_name, "table_name", MAX_NAME_LEN)?;

        // 2. Load existing snapshot
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

        // 3. Validate that at least one field is being updated
        if self.guest_count.is_none() && self.table_name.is_none() && self.is_pre_payment.is_none()
        {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::NoFieldsToUpdate,
                "No fields to update".to_string(),
            ));
        }

        // 4. Validate guest_count if provided
        if let Some(count) = self.guest_count
            && count < 1
        {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::InvalidGuestCount,
                "guest_count must be at least 1".to_string(),
            ));
        }

        // 5. Allocate sequence number
        let seq = ctx.next_sequence();

        // 6. Create event
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id,
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::OrderInfoUpdated,
            EventPayload::OrderInfoUpdated {
                guest_count: self.guest_count,
                table_name: self.table_name.clone(),
                is_pre_payment: self.is_pre_payment,
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
            operator_id: 1,
            operator_name: "Test User".to_string(),
            timestamp: 1234567890,
        }
    }

    fn create_active_order(order_id: &str) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.guest_count = 2;
        snapshot.table_name = Some("Table 1".to_string());
        snapshot
    }

    #[tokio::test]
    async fn test_update_order_info_guest_count() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UpdateOrderInfoAction {
            order_id: "order-1".to_string(),
            guest_count: Some(4),
            table_name: None,
            is_pre_payment: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.order_id, "order-1");
        assert_eq!(event.event_type, OrderEventType::OrderInfoUpdated);

        if let EventPayload::OrderInfoUpdated {
            guest_count,
            table_name,
            is_pre_payment,
        } = &event.payload
        {
            assert_eq!(*guest_count, Some(4));
            assert_eq!(*table_name, None);
            assert_eq!(*is_pre_payment, None);
        } else {
            panic!("Expected OrderInfoUpdated payload");
        }
    }

    #[tokio::test]
    async fn test_update_order_info_multiple_fields() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UpdateOrderInfoAction {
            order_id: "order-1".to_string(),
            guest_count: Some(6),
            table_name: Some("VIP Room".to_string()),
            is_pre_payment: Some(true),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::OrderInfoUpdated {
            guest_count,
            table_name,
            is_pre_payment,
        } = &events[0].payload
        {
            assert_eq!(*guest_count, Some(6));
            assert_eq!(table_name.as_deref(), Some("VIP Room"));
            assert_eq!(*is_pre_payment, Some(true));
        } else {
            panic!("Expected OrderInfoUpdated payload");
        }
    }

    #[tokio::test]
    async fn test_update_order_info_no_fields_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UpdateOrderInfoAction {
            order_id: "order-1".to_string(),
            guest_count: None,
            table_name: None,
            is_pre_payment: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(..))));
    }

    #[tokio::test]
    async fn test_update_order_info_invalid_guest_count_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UpdateOrderInfoAction {
            order_id: "order-1".to_string(),
            guest_count: Some(0),
            table_name: None,
            is_pre_payment: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(..))));
    }

    #[tokio::test]
    async fn test_update_order_info_negative_guest_count_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UpdateOrderInfoAction {
            order_id: "order-1".to_string(),
            guest_count: Some(-1),
            table_name: None,
            is_pre_payment: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(..))));
    }

    #[tokio::test]
    async fn test_update_order_info_completed_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UpdateOrderInfoAction {
            order_id: "order-1".to_string(),
            guest_count: Some(4),
            table_name: None,
            is_pre_payment: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[tokio::test]
    async fn test_update_order_info_voided_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UpdateOrderInfoAction {
            order_id: "order-1".to_string(),
            guest_count: Some(4),
            table_name: None,
            is_pre_payment: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyVoided(_))));
    }

    #[tokio::test]
    async fn test_update_order_info_order_not_found_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UpdateOrderInfoAction {
            order_id: "nonexistent".to_string(),
            guest_count: Some(4),
            table_name: None,
            is_pre_payment: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    #[tokio::test]
    async fn test_update_order_info_table_name_only() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UpdateOrderInfoAction {
            order_id: "order-1".to_string(),
            guest_count: None,
            table_name: Some("New Table".to_string()),
            is_pre_payment: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::OrderInfoUpdated {
            guest_count,
            table_name,
            is_pre_payment,
        } = &events[0].payload
        {
            assert_eq!(*guest_count, None);
            assert_eq!(table_name.as_deref(), Some("New Table"));
            assert_eq!(*is_pre_payment, None);
        } else {
            panic!("Expected OrderInfoUpdated payload");
        }
    }

    #[tokio::test]
    async fn test_update_order_info_is_pre_payment_only() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UpdateOrderInfoAction {
            order_id: "order-1".to_string(),
            guest_count: None,
            table_name: None,
            is_pre_payment: Some(true),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::OrderInfoUpdated { is_pre_payment, .. } = &events[0].payload {
            assert_eq!(*is_pre_payment, Some(true));
        } else {
            panic!("Expected OrderInfoUpdated payload");
        }
    }

    #[tokio::test]
    async fn test_update_order_info_sequence_allocation() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UpdateOrderInfoAction {
            order_id: "order-1".to_string(),
            guest_count: Some(3),
            table_name: None,
            is_pre_payment: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events[0].sequence, current_seq + 1);
    }

    #[tokio::test]
    async fn test_update_order_info_metadata_propagation() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UpdateOrderInfoAction {
            order_id: "order-1".to_string(),
            guest_count: Some(5),
            table_name: None,
            is_pre_payment: None,
        };

        let metadata = CommandMetadata {
            command_id: "test-cmd-123".to_string(),
            operator_id: 456,
            operator_name: "John Doe".to_string(),
            timestamp: 9999999999,
        };

        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events[0].command_id, "test-cmd-123");
        assert_eq!(events[0].operator_id, 456);
        assert_eq!(events[0].operator_name, "John Doe");
    }
}
