//! CompleteOrder command handler
//!
//! Completes an order, validating payment sufficiency and generating receipt.

use async_trait::async_trait;
use std::collections::HashMap;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus, PaymentSummaryItem};

/// CompleteOrder action
#[derive(Debug, Clone)]
pub struct CompleteOrderAction {
    pub order_id: String,
    pub receipt_number: String,
}

#[async_trait]
impl CommandHandler for CompleteOrderAction {
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
            _ => {
                return Err(OrderError::InvalidOperation(format!(
                    "Cannot complete order in {:?} status",
                    snapshot.status
                )));
            }
        }

        // 3. Calculate payment summary and total paid
        let mut payment_summary_map: HashMap<String, f64> = HashMap::new();
        let mut total_paid = 0.0_f64;
        for payment in &snapshot.payments {
            if !payment.cancelled {
                *payment_summary_map
                    .entry(payment.method.clone())
                    .or_insert(0.0) += payment.amount;
                total_paid += payment.amount;
            }
        }

        // 4. Validate payment is sufficient (allow 0.01 tolerance for rounding)
        if total_paid < snapshot.total - 0.01 {
            return Err(OrderError::InvalidOperation(format!(
                "Payment insufficient: paid {:.2}, required {:.2}",
                total_paid, snapshot.total
            )));
        }

        // 5. Convert payment summary to Vec<PaymentSummaryItem>
        let payment_summary: Vec<PaymentSummaryItem> = payment_summary_map
            .into_iter()
            .map(|(method, amount)| PaymentSummaryItem { method, amount })
            .collect();

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
            OrderEventType::OrderCompleted,
            EventPayload::OrderCompleted {
                receipt_number: self.receipt_number.clone(),
                final_total: snapshot.total,
                payment_summary,
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

    fn create_payment_record(method: &str, amount: f64) -> PaymentRecord {
        PaymentRecord {
            payment_id: format!("pay-{}", uuid::Uuid::new_v4()),
            method: method.to_string(),
            amount,
            tendered: None,
            change: None,
            note: None,
            timestamp: 1234567890,
            cancelled: false,
            cancel_reason: None,
        }
    }

    #[tokio::test]
    async fn test_complete_order_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create an active order with sufficient payment
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.payments.push(create_payment_record("CASH", 100.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            receipt_number: "RCP-001".to_string(),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.order_id, "order-1");
        assert_eq!(event.event_type, OrderEventType::OrderCompleted);

        if let EventPayload::OrderCompleted {
            receipt_number,
            final_total,
            payment_summary,
        } = &event.payload
        {
            assert_eq!(receipt_number, "RCP-001");
            assert_eq!(*final_total, 100.0);
            assert_eq!(payment_summary.len(), 1);
            assert_eq!(payment_summary[0].method, "CASH");
            assert_eq!(payment_summary[0].amount, 100.0);
        } else {
            panic!("Expected OrderCompleted payload");
        }
    }

    #[tokio::test]
    async fn test_complete_order_with_multiple_payments() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.payments.push(create_payment_record("CASH", 50.0));
        snapshot.payments.push(create_payment_record("CARD", 30.0));
        snapshot.payments.push(create_payment_record("CASH", 20.0)); // Same method
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            receipt_number: "RCP-002".to_string(),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::OrderCompleted {
            payment_summary, ..
        } = &events[0].payload
        {
            // Should have 2 items (CASH and CARD merged)
            assert_eq!(payment_summary.len(), 2);

            let cash_total: f64 = payment_summary
                .iter()
                .filter(|p| p.method == "CASH")
                .map(|p| p.amount)
                .sum();
            let card_total: f64 = payment_summary
                .iter()
                .filter(|p| p.method == "CARD")
                .map(|p| p.amount)
                .sum();

            assert_eq!(cash_total, 70.0); // 50 + 20
            assert_eq!(card_total, 30.0);
        } else {
            panic!("Expected OrderCompleted payload");
        }
    }

    #[tokio::test]
    async fn test_complete_order_excludes_cancelled_payments() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.payments.push(create_payment_record("CASH", 100.0));
        // Add a cancelled payment
        let mut cancelled_payment = create_payment_record("CARD", 50.0);
        cancelled_payment.cancelled = true;
        snapshot.payments.push(cancelled_payment);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            receipt_number: "RCP-003".to_string(),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::OrderCompleted {
            payment_summary, ..
        } = &events[0].payload
        {
            // Should only have CASH (cancelled CARD excluded)
            assert_eq!(payment_summary.len(), 1);
            assert_eq!(payment_summary[0].method, "CASH");
            assert_eq!(payment_summary[0].amount, 100.0);
        } else {
            panic!("Expected OrderCompleted payload");
        }
    }

    #[tokio::test]
    async fn test_complete_order_insufficient_payment() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.payments.push(create_payment_record("CASH", 50.0)); // Only 50 paid
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            receipt_number: "RCP-004".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
        if let Err(OrderError::InvalidOperation(msg)) = result {
            assert!(msg.contains("Payment insufficient"));
        }
    }

    #[tokio::test]
    async fn test_complete_order_allows_small_rounding_difference() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        // Pay 99.995 (within 0.01 tolerance)
        snapshot.payments.push(create_payment_record("CASH", 99.995));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            receipt_number: "RCP-005".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        // Should succeed due to rounding tolerance
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_complete_already_completed_order() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            receipt_number: "RCP-006".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[tokio::test]
    async fn test_complete_voided_order() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            receipt_number: "RCP-007".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyVoided(_))));
    }

    #[tokio::test]
    async fn test_complete_nonexistent_order() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "nonexistent".to_string(),
            receipt_number: "RCP-008".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    #[tokio::test]
    async fn test_complete_order_with_overpayment() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 100.0;
        snapshot.payments.push(create_payment_record("CASH", 150.0)); // Overpaid
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            receipt_number: "RCP-009".to_string(),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        // Overpayment should be allowed
        assert_eq!(events.len(), 1);
        if let EventPayload::OrderCompleted {
            final_total,
            payment_summary,
            ..
        } = &events[0].payload
        {
            assert_eq!(*final_total, 100.0);
            assert_eq!(payment_summary[0].amount, 150.0);
        } else {
            panic!("Expected OrderCompleted payload");
        }
    }

    #[tokio::test]
    async fn test_complete_order_with_zero_total() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.total = 0.0; // Zero total (e.g., all items free)
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            receipt_number: "RCP-010".to_string(),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        // Should succeed with no payments needed
        assert_eq!(events.len(), 1);
        if let EventPayload::OrderCompleted {
            payment_summary, ..
        } = &events[0].payload
        {
            assert!(payment_summary.is_empty());
        } else {
            panic!("Expected OrderCompleted payload");
        }
    }
}
