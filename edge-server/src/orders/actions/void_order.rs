//! VoidOrder command handler
//!
//! Voids an active order, optionally with a reason.

use async_trait::async_trait;

use crate::orders::money::{to_decimal, to_f64};
use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use rust_decimal::Decimal;
use shared::order::{EventPayload, LossReason, OrderEvent, OrderEventType, OrderStatus, VoidType};

/// VoidOrder action
#[derive(Debug, Clone)]
pub struct VoidOrderAction {
    pub order_id: String,
    pub void_type: VoidType,
    pub loss_reason: Option<LossReason>,
    pub loss_amount: Option<f64>,
    pub note: Option<String>,
    pub authorizer_id: Option<i64>,
    pub authorizer_name: Option<String>,
}

#[async_trait]
impl CommandHandler for VoidOrderAction {
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
            OrderStatus::Merged => {
                return Err(OrderError::InvalidOperation(format!(
                    "Cannot void order in {:?} status",
                    snapshot.status
                )));
            }
        }

        // 3. Sanitize loss fields based on void_type:
        //    - CANCELLED: 正常取消，无损失，强制清空 loss 字段
        //    - LOSS_SETTLED: 损失结算，自动计算未付金额作为损失
        let (loss_reason, loss_amount) = match self.void_type {
            VoidType::Cancelled => (None, None),
            VoidType::LossSettled => {
                let amount = self.loss_amount.unwrap_or_else(|| {
                    let remaining =
                        (to_decimal(snapshot.total) - to_decimal(snapshot.paid_amount))
                            .max(Decimal::ZERO);
                    to_f64(remaining)
                });
                (self.loss_reason.clone(), Some(amount))
            }
        };

        // 4. Allocate sequence number
        let seq = ctx.next_sequence();

        // 5. Create event
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id,
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::OrderVoided,
            EventPayload::OrderVoided {
                void_type: self.void_type.clone(),
                loss_reason,
                loss_amount,
                note: self.note.clone(),
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
    use shared::order::OrderSnapshot;

    fn create_test_metadata() -> CommandMetadata {
        CommandMetadata {
            command_id: "cmd-1".to_string(),
            operator_id: "user-1".to_string(),
            operator_name: "Test User".to_string(),
            timestamp: 1234567890,
        }
    }

    fn create_void_action(order_id: &str, note: Option<String>) -> VoidOrderAction {
        VoidOrderAction {
            order_id: order_id.to_string(),
            void_type: VoidType::Cancelled,
            loss_reason: None,
            loss_amount: None,
            note,
            authorizer_id: None,
            authorizer_name: None,
        }
    }

    #[tokio::test]
    async fn test_void_order_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create an active order
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = create_void_action("order-1", Some("Customer cancelled".to_string()));

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.order_id, "order-1");
        assert_eq!(event.event_type, OrderEventType::OrderVoided);

        if let EventPayload::OrderVoided {
            note,
            authorizer_id,
            authorizer_name,
            ..
        } = &event.payload
        {
            assert_eq!(*note, Some("Customer cancelled".to_string()));
            assert_eq!(*authorizer_id, None);
            assert_eq!(*authorizer_name, None);
        } else {
            panic!("Expected OrderVoided payload");
        }
    }

    #[tokio::test]
    async fn test_void_order_without_note() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = create_void_action("order-1", None);

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::OrderVoided { note, .. } = &events[0].payload {
            assert_eq!(*note, None);
        } else {
            panic!("Expected OrderVoided payload");
        }
    }

    #[tokio::test]
    async fn test_void_already_completed_order() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = create_void_action("order-1", None);

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[tokio::test]
    async fn test_void_already_voided_order() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = create_void_action("order-1", None);

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyVoided(_))));
    }

    #[tokio::test]
    async fn test_void_nonexistent_order() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = create_void_action("nonexistent", None);

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    #[tokio::test]
    async fn test_void_merged_order() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Merged;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = create_void_action("order-1", None);

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_void_order_with_items_and_payments() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create an order with items and payments
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.subtotal = 100.0;
        snapshot.paid_amount = 50.0; // Partial payment
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = create_void_action("order-1", Some("Order error".to_string()));

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        // Should succeed even with partial payment
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, OrderEventType::OrderVoided);
    }

    #[tokio::test]
    async fn test_void_order_event_metadata() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = create_void_action("order-1", None);

        let metadata = CommandMetadata {
            command_id: "cmd-void-1".to_string(),
            operator_id: "manager-1".to_string(),
            operator_name: "Manager".to_string(),
            timestamp: 9999999999,
        };

        let events = action.execute(&mut ctx, &metadata).await.unwrap();
        let event = &events[0];

        // Verify event metadata
        assert_eq!(event.command_id, "cmd-void-1");
        assert_eq!(event.operator_id, "manager-1");
        assert_eq!(event.operator_name, "Manager");
        // Note: event.timestamp is server-generated (now), client_timestamp is from metadata
        assert_eq!(event.client_timestamp, Some(9999999999));
    }

    #[tokio::test]
    async fn test_cancelled_void_strips_loss_fields() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Intentionally pass loss fields with CANCELLED — backend should strip them
        let action = VoidOrderAction {
            order_id: "order-1".to_string(),
            void_type: VoidType::Cancelled,
            loss_reason: Some(LossReason::CustomerFled),
            loss_amount: Some(50.0),
            note: Some("test".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::OrderVoided {
            void_type,
            loss_reason,
            loss_amount,
            ..
        } = &events[0].payload
        {
            assert_eq!(*void_type, VoidType::Cancelled);
            assert_eq!(*loss_reason, None, "CANCELLED should have no loss_reason");
            assert_eq!(*loss_amount, None, "CANCELLED should have no loss_amount");
        } else {
            panic!("Expected OrderVoided payload");
        }
    }

    #[tokio::test]
    async fn test_loss_settled_preserves_loss_fields() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.paid_amount = 60.0;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = VoidOrderAction {
            order_id: "order-1".to_string(),
            void_type: VoidType::LossSettled,
            loss_reason: Some(LossReason::CustomerFled),
            loss_amount: Some(40.0),
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::OrderVoided {
            void_type,
            loss_reason,
            loss_amount,
            ..
        } = &events[0].payload
        {
            assert_eq!(*void_type, VoidType::LossSettled);
            assert_eq!(*loss_reason, Some(LossReason::CustomerFled));
            assert_eq!(*loss_amount, Some(40.0));
        } else {
            panic!("Expected OrderVoided payload");
        }
    }

    #[tokio::test]
    async fn test_loss_settled_auto_calculates_loss_amount() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.paid_amount = 60.0;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // loss_amount = None → auto-calculate as total - paid_amount = 40
        let action = VoidOrderAction {
            order_id: "order-1".to_string(),
            void_type: VoidType::LossSettled,
            loss_reason: Some(LossReason::CustomerFled),
            loss_amount: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::OrderVoided {
            loss_amount, ..
        } = &events[0].payload
        {
            assert_eq!(*loss_amount, Some(40.0), "Should auto-calculate remaining as loss");
        } else {
            panic!("Expected OrderVoided payload");
        }
    }
}
