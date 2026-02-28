//! CancelPayment command handler
//!
//! Cancels an existing payment on an order.

use shared::order::types::CommandErrorCode;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use crate::utils::validation::{MAX_NAME_LEN, MAX_NOTE_LEN, validate_order_optional_text};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus, SplitType};

/// CancelPayment action
#[derive(Debug, Clone)]
pub struct CancelPaymentAction {
    pub order_id: i64,
    pub payment_id: i64,
    pub reason: Option<String>,
    pub authorizer_id: Option<i64>,
    pub authorizer_name: Option<String>,
}

impl CommandHandler for CancelPaymentAction {
    fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Validate text lengths
        validate_order_optional_text(&self.reason, "reason", MAX_NOTE_LEN)?;
        validate_order_optional_text(&self.authorizer_name, "authorizer_name", MAX_NAME_LEN)?;

        // 2. Load existing snapshot
        let snapshot = ctx.load_snapshot(self.order_id)?;

        // 3. Validate order status - must be Active
        match snapshot.status {
            OrderStatus::Active => {}
            OrderStatus::Completed => {
                return Err(OrderError::OrderAlreadyCompleted(self.order_id));
            }
            OrderStatus::Void => {
                return Err(OrderError::OrderAlreadyVoided(self.order_id));
            }
            OrderStatus::Merged => {
                return Err(OrderError::InvalidOperation(
                    CommandErrorCode::OrderNotActive,
                    format!(
                        "Cannot cancel payment on order with status {:?}",
                        snapshot.status
                    ),
                ));
            }
        }

        // 4. Find the payment (must exist and not already cancelled)
        let payment = snapshot
            .payments
            .iter()
            .find(|p| p.payment_id == self.payment_id && !p.cancelled)
            .ok_or(OrderError::PaymentNotFound(self.payment_id))?;

        // 5. Check if this is an AA payment that would zero-out AA shares
        let is_aa_zero_out = if let (Some(SplitType::AaSplit), Some(shares)) =
            (&payment.split_type, payment.aa_shares)
        {
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
            other_active_aa_shares == 0 && shares > 0
        } else {
            false
        };

        let aa_total_for_cancel = snapshot.aa_total_shares;

        // 6. Allocate sequence number and create event
        let seq = ctx.next_sequence();

        let event = OrderEvent::new(
            seq,
            self.order_id,
            metadata.operator_id,
            metadata.operator_name.clone(),
            metadata.command_id,
            Some(metadata.timestamp),
            OrderEventType::PaymentCancelled,
            EventPayload::PaymentCancelled {
                payment_id: self.payment_id,
                method: payment.method.clone(),
                amount: payment.amount,
                reason: self.reason.clone(),
                authorizer_id: self.authorizer_id,
                authorizer_name: self.authorizer_name.clone(),
            },
        );

        let mut events = vec![event];

        // 7. If AA zero-out, produce extra AaSplitCancelled event
        if is_aa_zero_out && let Some(total_shares) = aa_total_for_cancel {
            let seq2 = ctx.next_sequence();
            let cancel_event = OrderEvent::new(
                seq2,
                self.order_id,
                metadata.operator_id,
                metadata.operator_name.clone(),
                metadata.command_id,
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

    const ORDER_1: i64 = 1001;
    const PAYMENT_1: i64 = 2001;
    const PAYMENT_2: i64 = 2002;
    const AA_PAY_1: i64 = 3001;
    const AA_PAY_2: i64 = 3002;
    const AMT_PAY_1: i64 = 4001;
    const AMT_PAY_2: i64 = 4002;

    fn create_test_metadata() -> CommandMetadata {
        CommandMetadata {
            command_id: 1,
            operator_id: 1,
            operator_name: "Test User".to_string(),
            timestamp: 1234567890,
        }
    }

    fn create_payment_record(payment_id: i64, method: &str, amount: f64) -> PaymentRecord {
        PaymentRecord {
            payment_id,
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

    #[test]
    fn test_cancel_payment_generates_event() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new(ORDER_1);
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.paid_amount = 50.0;
        snapshot
            .payments
            .push(create_payment_record(PAYMENT_1, "CARD", 50.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: ORDER_1,
            payment_id: PAYMENT_1,
            reason: Some("Customer changed mind".to_string()),
            authorizer_id: Some(1),
            authorizer_name: Some("Manager".to_string()),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.order_id, ORDER_1);
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
            assert_eq!(*payment_id, PAYMENT_1);
            assert_eq!(method, "CARD");
            assert_eq!(*amount, 50.0);
            assert_eq!(*reason, Some("Customer changed mind".to_string()));
            assert_eq!(*authorizer_id, Some(1));
            assert_eq!(*authorizer_name, Some("Manager".to_string()));
        } else {
            panic!("Expected PaymentCancelled payload");
        }
    }

    #[test]
    fn test_cancel_payment_without_reason() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new(ORDER_1);
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.paid_amount = 50.0;
        snapshot
            .payments
            .push(create_payment_record(PAYMENT_1, "CASH", 50.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: ORDER_1,
            payment_id: PAYMENT_1,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).unwrap();

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

    #[test]
    fn test_cancel_payment_nonexistent_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new(ORDER_1);
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.paid_amount = 50.0;
        snapshot
            .payments
            .push(create_payment_record(PAYMENT_1, "CASH", 50.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: ORDER_1,
            payment_id: 9999,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata);
        assert!(matches!(result, Err(OrderError::PaymentNotFound(_))));
    }

    #[test]
    fn test_cancel_already_cancelled_payment_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new(ORDER_1);
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.paid_amount = 0.0;
        let mut payment = create_payment_record(PAYMENT_1, "CASH", 50.0);
        payment.cancelled = true;
        payment.cancel_reason = Some("Previous cancellation".to_string());
        snapshot.payments.push(payment);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: ORDER_1,
            payment_id: PAYMENT_1,
            reason: Some("Try again".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata);
        assert!(matches!(result, Err(OrderError::PaymentNotFound(_))));
    }

    #[test]
    fn test_cancel_payment_on_completed_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new(ORDER_1);
        snapshot.status = OrderStatus::Completed;
        snapshot.total = 100.0;
        snapshot.paid_amount = 100.0;
        snapshot
            .payments
            .push(create_payment_record(PAYMENT_1, "CASH", 100.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: ORDER_1,
            payment_id: PAYMENT_1,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata);
        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[test]
    fn test_cancel_payment_on_voided_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new(ORDER_1);
        snapshot.status = OrderStatus::Void;
        snapshot
            .payments
            .push(create_payment_record(PAYMENT_1, "CASH", 50.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: ORDER_1,
            payment_id: PAYMENT_1,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata);
        assert!(matches!(result, Err(OrderError::OrderAlreadyVoided(_))));
    }

    #[test]
    fn test_cancel_payment_on_nonexistent_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: 9999,
            payment_id: PAYMENT_1,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata);
        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    #[test]
    fn test_cancel_specific_payment_from_multiple() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new(ORDER_1);
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.paid_amount = 80.0;
        snapshot
            .payments
            .push(create_payment_record(PAYMENT_1, "CARD", 30.0));
        snapshot
            .payments
            .push(create_payment_record(PAYMENT_2, "CASH", 50.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: ORDER_1,
            payment_id: PAYMENT_2,
            reason: Some("Wrong amount".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::PaymentCancelled {
            payment_id,
            method,
            amount,
            ..
        } = &events[0].payload
        {
            assert_eq!(*payment_id, PAYMENT_2);
            assert_eq!(method, "CASH");
            assert_eq!(*amount, 50.0);
        } else {
            panic!("Expected PaymentCancelled payload");
        }
    }

    fn create_aa_payment(payment_id: i64, method: &str, amount: f64, shares: i32) -> PaymentRecord {
        PaymentRecord {
            payment_id,
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

    fn create_amount_split_payment(payment_id: i64, method: &str, amount: f64) -> PaymentRecord {
        PaymentRecord {
            payment_id,
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

    #[test]
    fn test_cancel_one_of_two_aa_payments_stays_active() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new(ORDER_1);
        snapshot.status = OrderStatus::Active;
        snapshot.total = 90.0;
        snapshot.paid_amount = 60.0;
        snapshot.aa_total_shares = Some(3);
        snapshot.aa_paid_shares = 2;
        snapshot
            .payments
            .push(create_aa_payment(AA_PAY_1, "CASH", 30.0, 1));
        snapshot
            .payments
            .push(create_aa_payment(AA_PAY_2, "CARD", 30.0, 1));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: ORDER_1,
            payment_id: AA_PAY_2,
            reason: Some("Wrong card".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, OrderEventType::PaymentCancelled);
    }

    #[test]
    fn test_cancel_all_aa_payments_produces_aa_cancelled() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new(ORDER_1);
        snapshot.status = OrderStatus::Active;
        snapshot.total = 90.0;
        snapshot.paid_amount = 30.0;
        snapshot.aa_total_shares = Some(3);
        snapshot.aa_paid_shares = 1;
        snapshot
            .payments
            .push(create_aa_payment(AA_PAY_1, "CASH", 30.0, 1));
        let mut cancelled_pay = create_aa_payment(AA_PAY_2, "CARD", 30.0, 1);
        cancelled_pay.cancelled = true;
        snapshot.payments.push(cancelled_pay);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: ORDER_1,
            payment_id: AA_PAY_1,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, OrderEventType::PaymentCancelled);
        assert_eq!(events[1].event_type, OrderEventType::AaSplitCancelled);

        if let EventPayload::AaSplitCancelled { total_shares } = &events[1].payload {
            assert_eq!(*total_shares, 3);
        } else {
            panic!("Expected AaSplitCancelled payload");
        }
    }

    #[test]
    fn test_cancel_amount_split_payment_produces_event() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new(ORDER_1);
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.paid_amount = 40.0;
        snapshot.has_amount_split = true;
        snapshot
            .payments
            .push(create_amount_split_payment(AMT_PAY_1, "CASH", 20.0));
        snapshot
            .payments
            .push(create_amount_split_payment(AMT_PAY_2, "CARD", 20.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelPaymentAction {
            order_id: ORDER_1,
            payment_id: AMT_PAY_1,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, OrderEventType::PaymentCancelled);
    }
}
