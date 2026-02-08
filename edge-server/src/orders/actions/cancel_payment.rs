//! CancelPayment command handler
//!
//! Cancels an existing payment on an order.

use async_trait::async_trait;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus, SplitType};

/// CancelPayment action
#[derive(Debug, Clone)]
pub struct CancelPaymentAction {
    pub order_id: String,
    pub payment_id: String,
    pub reason: Option<String>,
    pub authorizer_id: Option<i64>,
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
            OrderStatus::Merged => {
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

        // 4. Check if this is an AA payment that would zero-out AA shares
        let is_aa_zero_out = payment.split_type == Some(SplitType::AaSplit)
            && payment.aa_shares.is_some()
            && {
                let shares = payment.aa_shares.unwrap();
                // Calculate remaining after this cancel
                let other_active_aa_shares: i32 = snapshot
                    .payments
                    .iter()
                    .filter(|p| {
                        !p.cancelled
                            && p.payment_id != self.payment_id
                            && p.split_type == Some(SplitType::AaSplit)
                    })
                    .filter_map(|p| p.aa_shares)
                    .sum();
                // If cancelling this leaves 0 active AA shares
                other_active_aa_shares == 0 && shares > 0
            };

        let aa_total_for_cancel = snapshot.aa_total_shares;

        // 5. Allocate sequence number and create event
        let seq = ctx.next_sequence();

        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id,
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::PaymentCancelled,
            EventPayload::PaymentCancelled {
                payment_id: self.payment_id.clone(),
                method: payment.method.clone(),
                amount: payment.amount,
                reason: self.reason.clone(),
                authorizer_id: self.authorizer_id,
                authorizer_name: self.authorizer_name.clone(),
            },
        );

        let mut events = vec![event];

        // 6. If AA zero-out, produce extra AaSplitCancelled event
        if is_aa_zero_out
            && let Some(total_shares) = aa_total_for_cancel
        {
            let seq2 = ctx.next_sequence();
            let cancel_event = OrderEvent::new(
                seq2,
                self.order_id.clone(),
                metadata.operator_id,
                metadata.operator_name.clone(),
                metadata.command_id.clone(),
                Some(metadata.timestamp),
                OrderEventType::AaSplitCancelled,
                EventPayload::AaSplitCancelled { total_shares },
            );
            events.push(cancel_event);
        }

        Ok(events)
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
            split_items: None,
            aa_shares: None,
            split_type: None,
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
            .push(create_payment_record("payment-1", "CARD", 50.0));
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
            assert_eq!(method, "CARD");
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
            .push(create_payment_record("payment-1", "CASH", 50.0));
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
            .push(create_payment_record("payment-1", "CASH", 50.0));
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
        let mut payment = create_payment_record("payment-1", "CASH", 50.0);
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
            .push(create_payment_record("payment-1", "CASH", 100.0));
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
            .push(create_payment_record("payment-1", "CASH", 50.0));
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
            .push(create_payment_record("payment-1", "CARD", 30.0));
        snapshot
            .payments
            .push(create_payment_record("payment-2", "CASH", 50.0));
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
            assert_eq!(method, "CASH");
            assert_eq!(*amount, 50.0);
        } else {
            panic!("Expected PaymentCancelled payload");
        }
    }

    // ========== AA cancel rollback tests ==========

    fn create_aa_payment(payment_id: &str, method: &str, amount: f64, shares: i32) -> PaymentRecord {
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
            split_items: None,
            aa_shares: Some(shares),
            split_type: Some(SplitType::AaSplit),
        }
    }

    fn create_amount_split_payment(payment_id: &str, method: &str, amount: f64) -> PaymentRecord {
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
            split_items: None,
            aa_shares: None,
            split_type: Some(SplitType::AmountSplit),
        }
    }

    /// 2 AA payments, cancel 1 → still 1 active → NO AaSplitCancelled event
    #[tokio::test]
    async fn test_cancel_one_of_two_aa_payments_stays_active() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 90.0;
        snapshot.paid_amount = 60.0;
        snapshot.aa_total_shares = Some(3);
        snapshot.aa_paid_shares = 2;
        snapshot.payments.push(create_aa_payment("aa-pay-1", "CASH", 30.0, 1));
        snapshot.payments.push(create_aa_payment("aa-pay-2", "CARD", 30.0, 1));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: "order-1".to_string(),
            payment_id: "aa-pay-2".to_string(),
            reason: Some("Wrong card".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        // Only PaymentCancelled, NO AaSplitCancelled (still 1 active AA payment)
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, OrderEventType::PaymentCancelled);
    }

    /// 2 AA payments, cancel both → zero-out → produces AaSplitCancelled
    #[tokio::test]
    async fn test_cancel_all_aa_payments_produces_aa_cancelled() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 90.0;
        snapshot.paid_amount = 30.0;
        snapshot.aa_total_shares = Some(3);
        snapshot.aa_paid_shares = 1;
        // Only one active AA payment left
        snapshot.payments.push(create_aa_payment("aa-pay-1", "CASH", 30.0, 1));
        // Second one was already cancelled
        let mut cancelled_pay = create_aa_payment("aa-pay-2", "CARD", 30.0, 1);
        cancelled_pay.cancelled = true;
        snapshot.payments.push(cancelled_pay);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: "order-1".to_string(),
            payment_id: "aa-pay-1".to_string(),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        // PaymentCancelled + AaSplitCancelled (all AA shares gone)
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, OrderEventType::PaymentCancelled);
        assert_eq!(events[1].event_type, OrderEventType::AaSplitCancelled);

        if let EventPayload::AaSplitCancelled { total_shares } = &events[1].payload {
            assert_eq!(*total_shares, 3);
        } else {
            panic!("Expected AaSplitCancelled payload");
        }
    }

    /// Cancel amount split payment: has_amount_split flag relies on applier;
    /// Action just produces PaymentCancelled event.
    #[tokio::test]
    async fn test_cancel_amount_split_payment_produces_event() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.paid_amount = 40.0;
        snapshot.has_amount_split = true;
        snapshot.payments.push(create_amount_split_payment("amt-pay-1", "CASH", 20.0));
        snapshot.payments.push(create_amount_split_payment("amt-pay-2", "CARD", 20.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Cancel one of two amount split payments
        let action = CancelPaymentAction {
            order_id: "order-1".to_string(),
            payment_id: "amt-pay-1".to_string(),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, OrderEventType::PaymentCancelled);
    }
}
