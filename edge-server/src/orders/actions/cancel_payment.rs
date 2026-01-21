//! CancelPayment command handler
//!
//! Cancels an existing payment on an order.

use async_trait::async_trait;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// CancelPayment action
#[derive(Debug, Clone)]
pub struct CancelPaymentAction {
    pub order_id: String,
    pub payment_id: String,
    pub reason: Option<String>,
    pub authorizer_id: Option<String>,
    pub authorizer_name: Option<String>,
}

#[async_trait]
impl CommandHandler for CancelPaymentAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Load existing snapshot
        let snapshot = ctx.load_snapshot(&self.order_id)?;

        // 2. Validate order status - must be Active
        match snapshot.status {
            OrderStatus::Active => {} // OK - continue with cancellation
            OrderStatus::Completed => {
                return Err(OrderError::OrderAlreadyCompleted(self.order_id.clone()));
            }
            OrderStatus::Void => {
                return Err(OrderError::OrderAlreadyVoided(self.order_id.clone()));
            }
            OrderStatus::Moved | OrderStatus::Merged => {
                return Err(OrderError::InvalidOperation(format!(
                    "Cannot cancel payment on order with status {:?}",
                    snapshot.status
                )));
            }
        }

        // 3. Find the payment (must exist and not already cancelled)
        let payment = snapshot
            .payments
            .iter()
            .find(|p| p.payment_id == self.payment_id && !p.cancelled)
            .ok_or_else(|| OrderError::PaymentNotFound(self.payment_id.clone()))?;

        // 4. Allocate sequence number
        let seq = ctx.next_sequence();

        // 5. Create event
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::PaymentCancelled,
            EventPayload::PaymentCancelled {
                payment_id: self.payment_id.clone(),
                method: payment.method.clone(),
                amount: payment.amount,
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
    use shared::order::{OrderSnapshot, PaymentRecord};

    fn create_test_metadata() -> CommandMetadata {
        CommandMetadata {
            command_id: "cmd-1".to_string(),
            operator_id: "user-1".to_string(),
            operator_name: "Test User".to_string(),
            timestamp: 1234567890,
        }
    }

    fn create_payment_record(payment_id: &str, method: &str, amount: f64) -> PaymentRecord {
        PaymentRecord {
            payment_id: payment_id.to_string(),
            method: method.to_string(),
            amount,
            tendered: None,
            change: None,
            note: None,
            timestamp: 1234567800,
            cancelled: false,
            cancel_reason: None,
        }
    }

    #[tokio::test]
    async fn test_cancel_payment_generates_event() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        // Create and store an active order with a payment
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.paid_amount = 50.0;
        snapshot
            .payments
            .push(create_payment_record("payment-1", "credit_card", 50.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: "order-1".to_string(),
            payment_id: "payment-1".to_string(),
            reason: Some("Customer changed mind".to_string()),
            authorizer_id: Some("manager-1".to_string()),
            authorizer_name: Some("Manager".to_string()),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.order_id, "order-1");
        assert_eq!(event.event_type, OrderEventType::PaymentCancelled);

        if let EventPayload::PaymentCancelled {
            payment_id,
            method,
            amount,
            reason,
            authorizer_id,
            authorizer_name,
        } = &event.payload
        {
            assert_eq!(payment_id, "payment-1");
            assert_eq!(method, "credit_card");
            assert_eq!(*amount, 50.0);
            assert_eq!(*reason, Some("Customer changed mind".to_string()));
            assert_eq!(*authorizer_id, Some("manager-1".to_string()));
            assert_eq!(*authorizer_name, Some("Manager".to_string()));
        } else {
            panic!("Expected PaymentCancelled payload");
        }
    }

    #[tokio::test]
    async fn test_cancel_payment_without_reason() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.paid_amount = 50.0;
        snapshot
            .payments
            .push(create_payment_record("payment-1", "cash", 50.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: "order-1".to_string(),
            payment_id: "payment-1".to_string(),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::PaymentCancelled {
            reason,
            authorizer_id,
            authorizer_name,
            ..
        } = &events[0].payload
        {
            assert!(reason.is_none());
            assert!(authorizer_id.is_none());
            assert!(authorizer_name.is_none());
        } else {
            panic!("Expected PaymentCancelled payload");
        }
    }

    #[tokio::test]
    async fn test_cancel_payment_nonexistent_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.paid_amount = 50.0;
        snapshot
            .payments
            .push(create_payment_record("payment-1", "cash", 50.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: "order-1".to_string(),
            payment_id: "nonexistent".to_string(),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::PaymentNotFound(_))));
    }

    #[tokio::test]
    async fn test_cancel_already_cancelled_payment_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.paid_amount = 0.0; // Already cancelled, so paid_amount is 0
        let mut payment = create_payment_record("payment-1", "cash", 50.0);
        payment.cancelled = true;
        payment.cancel_reason = Some("Previous cancellation".to_string());
        snapshot.payments.push(payment);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: "order-1".to_string(),
            payment_id: "payment-1".to_string(),
            reason: Some("Try again".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        // Should fail because payment is already cancelled
        assert!(matches!(result, Err(OrderError::PaymentNotFound(_))));
    }

    #[tokio::test]
    async fn test_cancel_payment_on_completed_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        snapshot.total = 100.0;
        snapshot.paid_amount = 100.0;
        snapshot
            .payments
            .push(create_payment_record("payment-1", "cash", 100.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: "order-1".to_string(),
            payment_id: "payment-1".to_string(),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[tokio::test]
    async fn test_cancel_payment_on_voided_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        snapshot
            .payments
            .push(create_payment_record("payment-1", "cash", 50.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: "order-1".to_string(),
            payment_id: "payment-1".to_string(),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyVoided(_))));
    }

    #[tokio::test]
    async fn test_cancel_payment_on_nonexistent_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: "nonexistent".to_string(),
            payment_id: "payment-1".to_string(),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    #[tokio::test]
    async fn test_cancel_specific_payment_from_multiple() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.paid_amount = 80.0;
        snapshot
            .payments
            .push(create_payment_record("payment-1", "credit_card", 30.0));
        snapshot
            .payments
            .push(create_payment_record("payment-2", "cash", 50.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Cancel the second payment
        let action = CancelPaymentAction {
            order_id: "order-1".to_string(),
            payment_id: "payment-2".to_string(),
            reason: Some("Wrong amount".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::PaymentCancelled {
            payment_id,
            method,
            amount,
            ..
        } = &events[0].payload
        {
            assert_eq!(payment_id, "payment-2");
            assert_eq!(method, "cash");
            assert_eq!(*amount, 50.0);
        } else {
            panic!("Expected PaymentCancelled payload");
        }
    }
}
