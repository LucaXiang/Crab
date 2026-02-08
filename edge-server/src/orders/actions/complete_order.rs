//! CompleteOrder command handler
//!
//! Completes an order, validating payment sufficiency and generating receipt.

use async_trait::async_trait;
use rust_decimal::prelude::*;
use std::collections::HashMap;

use crate::orders::money::{is_payment_sufficient, to_decimal, to_f64};
use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::types::ServiceType;
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus, PaymentSummaryItem};

/// CompleteOrder action
#[derive(Debug, Clone)]
pub struct CompleteOrderAction {
    pub order_id: String,
    /// 服务类型（零售订单结单时确认：堂食/外带）
    pub service_type: Option<ServiceType>,
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

        // 3. Calculate payment summary and total paid using precise decimal arithmetic
        let mut payment_summary_map: HashMap<String, Decimal> = HashMap::new();
        let mut total_paid = Decimal::ZERO;
        for payment in &snapshot.payments {
            if !payment.cancelled {
                let amount = to_decimal(payment.amount);
                *payment_summary_map
                    .entry(payment.method.clone())
                    .or_insert(Decimal::ZERO) += amount;
                total_paid += amount;
            }
        }

        // 4. Validate payment is sufficient (using precise comparison with tolerance)
        let total_paid_f64 = to_f64(total_paid);
        if !is_payment_sufficient(total_paid_f64, snapshot.total) {
            return Err(OrderError::InvalidOperation(format!(
                "Payment insufficient: paid {:.2}, required {:.2}",
                total_paid_f64, snapshot.total
            )));
        }

        // 5. Convert payment summary to Vec<PaymentSummaryItem>
        let payment_summary: Vec<PaymentSummaryItem> = payment_summary_map
            .into_iter()
            .map(|(method, amount)| PaymentSummaryItem {
                method,
                amount: to_f64(amount),
            })
            .collect();

        // 6. Allocate sequence number
        let seq = ctx.next_sequence();

        // 7. Create event (receipt_number from snapshot, set at OpenTable)
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id,
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::OrderCompleted,
            EventPayload::OrderCompleted {
                receipt_number: snapshot.receipt_number.clone(),
                service_type: self.service_type,
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
    use shared::order::types::ServiceType;
    use shared::order::{OrderSnapshot, PaymentRecord};

    fn create_test_metadata() -> CommandMetadata {
        CommandMetadata {
            command_id: "cmd-1".to_string(),
            operator_id: 1,
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
            split_items: None,
            aa_shares: None,
            split_type: None,
        }
    }

    /// Helper: create an active order snapshot with receipt_number set
    fn create_active_snapshot(order_id: &str, receipt_number: &str) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.receipt_number = receipt_number.to_string();
        snapshot
    }

    #[tokio::test]
    async fn test_complete_order_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_snapshot("order-1", "RCP-001");
        snapshot.total = 100.0;
        snapshot.payments.push(create_payment_record("CASH", 100.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            service_type: Some(ServiceType::DineIn),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.order_id, "order-1");
        assert_eq!(event.event_type, OrderEventType::OrderCompleted);

        if let EventPayload::OrderCompleted {
            receipt_number,
            service_type,
            final_total,
            payment_summary,
        } = &event.payload
        {
            assert_eq!(receipt_number, "RCP-001");
            assert_eq!(*service_type, Some(ServiceType::DineIn));
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

        let mut snapshot = create_active_snapshot("order-1", "RCP-002");
        snapshot.total = 100.0;
        snapshot.payments.push(create_payment_record("CASH", 50.0));
        snapshot.payments.push(create_payment_record("CARD", 30.0));
        snapshot.payments.push(create_payment_record("CASH", 20.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            service_type: Some(ServiceType::DineIn),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::OrderCompleted {
            payment_summary, ..
        } = &events[0].payload
        {
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
            assert_eq!(cash_total, 70.0);
            assert_eq!(card_total, 30.0);
        } else {
            panic!("Expected OrderCompleted payload");
        }
    }

    #[tokio::test]
    async fn test_complete_order_excludes_cancelled_payments() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_snapshot("order-1", "RCP-003");
        snapshot.total = 100.0;
        snapshot.payments.push(create_payment_record("CASH", 100.0));
        let mut cancelled_payment = create_payment_record("CARD", 50.0);
        cancelled_payment.cancelled = true;
        snapshot.payments.push(cancelled_payment);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            service_type: Some(ServiceType::DineIn),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::OrderCompleted {
            payment_summary, ..
        } = &events[0].payload
        {
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

        let mut snapshot = create_active_snapshot("order-1", "RCP-004");
        snapshot.total = 100.0;
        snapshot.payments.push(create_payment_record("CASH", 50.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            service_type: Some(ServiceType::DineIn),
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

        let mut snapshot = create_active_snapshot("order-1", "RCP-005");
        snapshot.total = 100.0;
        snapshot.payments.push(create_payment_record("CASH", 99.995));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            service_type: Some(ServiceType::DineIn),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_complete_already_completed_order() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        snapshot.receipt_number = "RCP-006".to_string();
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            service_type: Some(ServiceType::DineIn),
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
        snapshot.receipt_number = "RCP-007".to_string();
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            service_type: Some(ServiceType::DineIn),
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
            service_type: Some(ServiceType::DineIn),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    #[tokio::test]
    async fn test_complete_order_with_overpayment() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_snapshot("order-1", "RCP-009");
        snapshot.total = 100.0;
        snapshot.payments.push(create_payment_record("CASH", 150.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            service_type: Some(ServiceType::DineIn),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

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

        let mut snapshot = create_active_snapshot("order-1", "RCP-010");
        snapshot.total = 0.0;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            service_type: Some(ServiceType::DineIn),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

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

    #[tokio::test]
    async fn test_complete_order_with_dine_in_service_type() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_snapshot("order-1", "RCP-011");
        snapshot.total = 50.0;
        snapshot.payments.push(create_payment_record("CASH", 50.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            service_type: Some(ServiceType::DineIn),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::OrderCompleted {
            service_type, ..
        } = &events[0].payload
        {
            assert_eq!(*service_type, Some(ServiceType::DineIn));
        } else {
            panic!("Expected OrderCompleted payload");
        }
    }

    #[tokio::test]
    async fn test_complete_order_with_takeout_service_type() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_snapshot("order-1", "RCP-012");
        snapshot.total = 30.0;
        snapshot.payments.push(create_payment_record("CARD", 30.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CompleteOrderAction {
            order_id: "order-1".to_string(),
            service_type: Some(ServiceType::Takeout),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::OrderCompleted {
            service_type, ..
        } = &events[0].payload
        {
            assert_eq!(*service_type, Some(ServiceType::Takeout));
        } else {
            panic!("Expected OrderCompleted payload");
        }
    }
}
