//! AddPayment command handler
//!
//! Adds a payment to an existing order.

use async_trait::async_trait;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus, PaymentInput};

/// AddPayment action
#[derive(Debug, Clone)]
pub struct AddPaymentAction {
    pub order_id: String,
    pub payment: PaymentInput,
}

#[async_trait]
impl CommandHandler for AddPaymentAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Load existing snapshot
        let snapshot = ctx.load_snapshot(&self.order_id)?;

        // 2. Validate order status - must be Active
        match snapshot.status {
            OrderStatus::Active => {} // OK - continue with payment
            OrderStatus::Completed => {
                return Err(OrderError::OrderAlreadyCompleted(self.order_id.clone()));
            }
            OrderStatus::Void => {
                return Err(OrderError::OrderAlreadyVoided(self.order_id.clone()));
            }
            OrderStatus::Moved | OrderStatus::Merged => {
                return Err(OrderError::InvalidOperation(format!(
                    "Cannot add payment to order with status {:?}",
                    snapshot.status
                )));
            }
        }

        // 3. Validate payment amount
        if self.payment.amount <= 0.0 {
            return Err(OrderError::InvalidAmount);
        }

        // 4. Allocate sequence number
        let seq = ctx.next_sequence();

        // 5. Generate payment_id
        let payment_id = uuid::Uuid::new_v4().to_string();

        // 6. Calculate change for cash payments
        let change = self
            .payment
            .tendered
            .map(|t| (t - self.payment.amount).max(0.0));

        // 7. Create event
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::PaymentAdded,
            EventPayload::PaymentAdded {
                payment_id,
                method: self.payment.method.clone(),
                amount: self.payment.amount,
                tendered: self.payment.tendered,
                change,
                note: self.payment.note.clone(),
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

    fn create_payment_input(method: &str, amount: f64) -> PaymentInput {
        PaymentInput {
            method: method.to_string(),
            amount,
            tendered: None,
            note: None,
        }
    }

    fn create_cash_payment_input(amount: f64, tendered: f64) -> PaymentInput {
        PaymentInput {
            method: "CASH".to_string(),
            amount,
            tendered: Some(tendered),
            note: None,
        }
    }

    #[tokio::test]
    async fn test_add_payment_generates_event() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        // Create and store an active order
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddPaymentAction {
            order_id: "order-1".to_string(),
            payment: create_payment_input("CARD", 50.0),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.order_id, "order-1");
        assert_eq!(event.event_type, OrderEventType::PaymentAdded);

        if let EventPayload::PaymentAdded {
            payment_id,
            method,
            amount,
            tendered,
            change,
            note,
        } = &event.payload
        {
            assert!(!payment_id.is_empty());
            assert_eq!(method, "CARD");
            assert_eq!(*amount, 50.0);
            assert!(tendered.is_none());
            assert!(change.is_none());
            assert!(note.is_none());
        } else {
            panic!("Expected PaymentAdded payload");
        }
    }

    #[tokio::test]
    async fn test_add_cash_payment_with_change() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 85.0;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddPaymentAction {
            order_id: "order-1".to_string(),
            payment: create_cash_payment_input(85.0, 100.0),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::PaymentAdded {
            tendered, change, ..
        } = &events[0].payload
        {
            assert_eq!(*tendered, Some(100.0));
            assert_eq!(*change, Some(15.0)); // 100 - 85 = 15
        } else {
            panic!("Expected PaymentAdded payload");
        }
    }

    #[tokio::test]
    async fn test_add_payment_to_completed_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddPaymentAction {
            order_id: "order-1".to_string(),
            payment: create_payment_input("CARD", 50.0),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[tokio::test]
    async fn test_add_payment_to_voided_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddPaymentAction {
            order_id: "order-1".to_string(),
            payment: create_payment_input("CARD", 50.0),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyVoided(_))));
    }

    #[tokio::test]
    async fn test_add_payment_to_nonexistent_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddPaymentAction {
            order_id: "nonexistent".to_string(),
            payment: create_payment_input("CARD", 50.0),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    #[tokio::test]
    async fn test_add_payment_with_zero_amount_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddPaymentAction {
            order_id: "order-1".to_string(),
            payment: create_payment_input("CASH", 0.0),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidAmount)));
    }

    #[tokio::test]
    async fn test_add_payment_with_negative_amount_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddPaymentAction {
            order_id: "order-1".to_string(),
            payment: create_payment_input("CASH", -10.0),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidAmount)));
    }

    #[tokio::test]
    async fn test_add_payment_with_note() {
        let storage = OrderStorage::open_in_memory().unwrap();

        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let payment = PaymentInput {
            method: "CARD".to_string(),
            amount: 50.0,
            tendered: None,
            note: Some("Visa ending in 1234".to_string()),
        };

        let action = AddPaymentAction {
            order_id: "order-1".to_string(),
            payment,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::PaymentAdded { note, .. } = &events[0].payload {
            assert_eq!(*note, Some("Visa ending in 1234".to_string()));
        } else {
            panic!("Expected PaymentAdded payload");
        }
    }
}
