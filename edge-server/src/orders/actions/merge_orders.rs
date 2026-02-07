//! MergeOrders command handler
//!
//! Merges items and payments from source order into target order.
//! Source order is marked as Merged status. Generates two events:
//! - OrderMergedOut for the source order
//! - OrderMerged for the target order

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::orders::money::to_decimal;
use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// MergeOrders action
#[derive(Debug, Clone)]
pub struct MergeOrdersAction {
    pub source_order_id: String,
    pub target_order_id: String,
    pub authorizer_id: Option<String>,
    pub authorizer_name: Option<String>,
}

#[async_trait]
impl CommandHandler for MergeOrdersAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Load source snapshot
        let source_snapshot = ctx.load_snapshot(&self.source_order_id)?;

        // 2. Validate source order status - must be Active
        match source_snapshot.status {
            OrderStatus::Active => {}
            OrderStatus::Completed => {
                return Err(OrderError::OrderAlreadyCompleted(
                    self.source_order_id.clone(),
                ));
            }
            OrderStatus::Void => {
                return Err(OrderError::OrderAlreadyVoided(self.source_order_id.clone()));
            }
            OrderStatus::Merged => {
                return Err(OrderError::InvalidOperation(format!(
                    "Source order {} is already merged",
                    self.source_order_id
                )));
            }
            _ => {
                return Err(OrderError::OrderNotFound(self.source_order_id.clone()));
            }
        }

        // 3. Load target snapshot
        let target_snapshot = ctx.load_snapshot(&self.target_order_id)?;

        // 4. Validate target order status - must be Active
        match target_snapshot.status {
            OrderStatus::Active => {}
            OrderStatus::Completed => {
                return Err(OrderError::OrderAlreadyCompleted(
                    self.target_order_id.clone(),
                ));
            }
            OrderStatus::Void => {
                return Err(OrderError::OrderAlreadyVoided(self.target_order_id.clone()));
            }
            OrderStatus::Merged => {
                return Err(OrderError::InvalidOperation(format!(
                    "Target order {} is already merged",
                    self.target_order_id
                )));
            }
            _ => {
                return Err(OrderError::OrderNotFound(self.target_order_id.clone()));
            }
        }

        // 5. Cannot merge order into itself
        if self.source_order_id == self.target_order_id {
            return Err(OrderError::InvalidOperation(
                "Cannot merge order into itself".to_string(),
            ));
        }

        // 6. Reject if either order has payments
        if to_decimal(source_snapshot.paid_amount) > Decimal::ZERO {
            return Err(OrderError::InvalidOperation(
                "存在支付记录的订单不能合并".to_string(),
            ));
        }
        if to_decimal(target_snapshot.paid_amount) > Decimal::ZERO {
            return Err(OrderError::InvalidOperation(
                "目标订单存在支付记录，不能合并".to_string(),
            ));
        }

        // 7. Reject if either order has active AA split
        if source_snapshot.aa_total_shares.is_some() {
            return Err(OrderError::InvalidOperation(
                "源订单存在 AA 分单，不能合并".to_string(),
            ));
        }
        if target_snapshot.aa_total_shares.is_some() {
            return Err(OrderError::InvalidOperation(
                "目标订单存在 AA 分单，不能合并".to_string(),
            ));
        }

        // 9. Extract table info
        let source_table_id = source_snapshot.table_id.clone().unwrap_or_default();
        let source_table_name = source_snapshot.table_name.clone().unwrap_or_default();
        let target_table_id = target_snapshot.table_id.clone().unwrap_or_default();
        let target_table_name = target_snapshot.table_name.clone().unwrap_or_default();

        // 10. Allocate sequence numbers for both events
        let seq1 = ctx.next_sequence();
        let seq2 = ctx.next_sequence();

        // 11. Create OrderMergedOut event for source order
        let event1 = OrderEvent::new(
            seq1,
            self.source_order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::OrderMergedOut,
            EventPayload::OrderMergedOut {
                target_table_id: target_table_id.clone(),
                target_table_name: target_table_name.clone(),
                reason: None,
                authorizer_id: self.authorizer_id.clone(),
                authorizer_name: self.authorizer_name.clone(),
            },
        );

        // 12. Create OrderMerged event for target order (includes source items)
        let event2 = OrderEvent::new(
            seq2,
            self.target_order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::OrderMerged,
            EventPayload::OrderMerged {
                source_table_id,
                source_table_name,
                items: source_snapshot.items.clone(),
                payments: source_snapshot.payments.clone(),
                paid_item_quantities: source_snapshot.paid_item_quantities.clone(),
                paid_amount: source_snapshot.paid_amount,
                has_amount_split: source_snapshot.has_amount_split,
                aa_total_shares: source_snapshot.aa_total_shares,
                aa_paid_shares: source_snapshot.aa_paid_shares,
                authorizer_id: self.authorizer_id.clone(),
                authorizer_name: self.authorizer_name.clone(),
            },
        );

        Ok(vec![event1, event2])
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

    fn create_active_order(order_id: &str, table_id: &str, table_name: &str) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.table_id = Some(table_id.to_string());
        snapshot.table_name = Some(table_name.to_string());
        snapshot
    }

    fn create_test_item(instance_id: &str, name: &str) -> CartItemSnapshot {
        CartItemSnapshot {
            id: "product:1".to_string(),
            instance_id: instance_id.to_string(),
            name: name.to_string(),
            price: 10.0,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
        is_comped: false,
        }
    }

    #[tokio::test]
    async fn test_merge_orders_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create source and target orders
        let source = create_active_order("source-1", "dining_table:t1", "Table 1");
        let target = create_active_order("target-1", "dining_table:t2", "Table 2");
        storage.store_snapshot(&txn, &source).unwrap();
        storage.store_snapshot(&txn, &target).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MergeOrdersAction {
            source_order_id: "source-1".to_string(),
            target_order_id: "target-1".to_string(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 2);

        // First event: OrderMergedOut for source
        assert_eq!(events[0].order_id, "source-1");
        assert_eq!(events[0].event_type, OrderEventType::OrderMergedOut);
        if let EventPayload::OrderMergedOut {
            target_table_id,
            target_table_name,
            ..
        } = &events[0].payload
        {
            assert_eq!(target_table_id, "dining_table:t2");
            assert_eq!(target_table_name, "Table 2");
        } else {
            panic!("Expected OrderMergedOut payload");
        }

        // Second event: OrderMerged for target
        assert_eq!(events[1].order_id, "target-1");
        assert_eq!(events[1].event_type, OrderEventType::OrderMerged);
        if let EventPayload::OrderMerged {
            source_table_id,
            source_table_name,
            items,
            payments,
            paid_amount,
            ..
        } = &events[1].payload
        {
            assert_eq!(source_table_id, "dining_table:t1");
            assert_eq!(source_table_name, "Table 1");
            assert!(items.is_empty());
            assert!(payments.is_empty());
            assert_eq!(*paid_amount, 0.0);
        } else {
            panic!("Expected OrderMerged payload");
        }
    }

    #[tokio::test]
    async fn test_merge_orders_with_items() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut source = create_active_order("source-1", "dining_table:t1", "Table 1");
        source.items.push(create_test_item("item-1", "Coffee"));
        source.items.push(create_test_item("item-2", "Tea"));

        let target = create_active_order("target-1", "dining_table:t2", "Table 2");
        storage.store_snapshot(&txn, &source).unwrap();
        storage.store_snapshot(&txn, &target).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MergeOrdersAction {
            source_order_id: "source-1".to_string(),
            target_order_id: "target-1".to_string(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        // Check items and payment state are included in OrderMerged event
        if let EventPayload::OrderMerged { items, payments, paid_amount, .. } = &events[1].payload {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].instance_id, "item-1");
            assert_eq!(items[0].name, "Coffee");
            assert_eq!(items[1].instance_id, "item-2");
            assert_eq!(items[1].name, "Tea");
            assert!(payments.is_empty());
            assert_eq!(*paid_amount, 0.0);
        } else {
            panic!("Expected OrderMerged payload");
        }
    }

    #[tokio::test]
    async fn test_merge_orders_source_not_found() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let target = create_active_order("target-1", "dining_table:t2", "Table 2");
        storage.store_snapshot(&txn, &target).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MergeOrdersAction {
            source_order_id: "nonexistent".to_string(),
            target_order_id: "target-1".to_string(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    #[tokio::test]
    async fn test_merge_orders_target_not_found() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let source = create_active_order("source-1", "dining_table:t1", "Table 1");
        storage.store_snapshot(&txn, &source).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MergeOrdersAction {
            source_order_id: "source-1".to_string(),
            target_order_id: "nonexistent".to_string(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    #[tokio::test]
    async fn test_merge_orders_source_completed_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut source = OrderSnapshot::new("source-1".to_string());
        source.status = OrderStatus::Completed;
        let target = create_active_order("target-1", "dining_table:t2", "Table 2");
        storage.store_snapshot(&txn, &source).unwrap();
        storage.store_snapshot(&txn, &target).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MergeOrdersAction {
            source_order_id: "source-1".to_string(),
            target_order_id: "target-1".to_string(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[tokio::test]
    async fn test_merge_orders_source_voided_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut source = OrderSnapshot::new("source-1".to_string());
        source.status = OrderStatus::Void;
        let target = create_active_order("target-1", "dining_table:t2", "Table 2");
        storage.store_snapshot(&txn, &source).unwrap();
        storage.store_snapshot(&txn, &target).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MergeOrdersAction {
            source_order_id: "source-1".to_string(),
            target_order_id: "target-1".to_string(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyVoided(_))));
    }

    #[tokio::test]
    async fn test_merge_orders_source_already_merged_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut source = OrderSnapshot::new("source-1".to_string());
        source.status = OrderStatus::Merged;
        let target = create_active_order("target-1", "dining_table:t2", "Table 2");
        storage.store_snapshot(&txn, &source).unwrap();
        storage.store_snapshot(&txn, &target).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MergeOrdersAction {
            source_order_id: "source-1".to_string(),
            target_order_id: "target-1".to_string(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_merge_orders_target_completed_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let source = create_active_order("source-1", "dining_table:t1", "Table 1");
        let mut target = OrderSnapshot::new("target-1".to_string());
        target.status = OrderStatus::Completed;
        storage.store_snapshot(&txn, &source).unwrap();
        storage.store_snapshot(&txn, &target).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MergeOrdersAction {
            source_order_id: "source-1".to_string(),
            target_order_id: "target-1".to_string(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[tokio::test]
    async fn test_merge_orders_target_voided_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let source = create_active_order("source-1", "dining_table:t1", "Table 1");
        let mut target = OrderSnapshot::new("target-1".to_string());
        target.status = OrderStatus::Void;
        storage.store_snapshot(&txn, &source).unwrap();
        storage.store_snapshot(&txn, &target).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MergeOrdersAction {
            source_order_id: "source-1".to_string(),
            target_order_id: "target-1".to_string(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyVoided(_))));
    }

    #[tokio::test]
    async fn test_merge_orders_into_self_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let order = create_active_order("order-1", "dining_table:t1", "Table 1");
        storage.store_snapshot(&txn, &order).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MergeOrdersAction {
            source_order_id: "order-1".to_string(),
            target_order_id: "order-1".to_string(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_merge_orders_sequence_allocation() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let source = create_active_order("source-1", "dining_table:t1", "Table 1");
        let target = create_active_order("target-1", "dining_table:t2", "Table 2");
        storage.store_snapshot(&txn, &source).unwrap();
        storage.store_snapshot(&txn, &target).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MergeOrdersAction {
            source_order_id: "source-1".to_string(),
            target_order_id: "target-1".to_string(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        // Events should have consecutive sequence numbers
        assert_eq!(events[0].sequence, current_seq + 1);
        assert_eq!(events[1].sequence, current_seq + 2);
    }

    #[tokio::test]
    async fn test_merge_orders_metadata_propagation() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let source = create_active_order("source-1", "dining_table:t1", "Table 1");
        let target = create_active_order("target-1", "dining_table:t2", "Table 2");
        storage.store_snapshot(&txn, &source).unwrap();
        storage.store_snapshot(&txn, &target).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MergeOrdersAction {
            source_order_id: "source-1".to_string(),
            target_order_id: "target-1".to_string(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = CommandMetadata {
            command_id: "test-cmd-123".to_string(),
            operator_id: "operator-456".to_string(),
            operator_name: "John Doe".to_string(),
            timestamp: 9999999999,
        };

        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        // Both events should have same metadata
        for event in &events {
            assert_eq!(event.command_id, "test-cmd-123");
            assert_eq!(event.operator_id, "operator-456");
            assert_eq!(event.operator_name, "John Doe");
        }
    }

    #[tokio::test]
    async fn test_merge_orders_without_table_info() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut source = OrderSnapshot::new("source-1".to_string());
        source.status = OrderStatus::Active;
        // No table info

        let mut target = OrderSnapshot::new("target-1".to_string());
        target.status = OrderStatus::Active;
        // No table info

        storage.store_snapshot(&txn, &source).unwrap();
        storage.store_snapshot(&txn, &target).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MergeOrdersAction {
            source_order_id: "source-1".to_string(),
            target_order_id: "target-1".to_string(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        // Should use empty strings for missing table info
        if let EventPayload::OrderMergedOut {
            target_table_id,
            target_table_name,
            ..
        } = &events[0].payload
        {
            assert_eq!(target_table_id, "");
            assert_eq!(target_table_name, "");
        } else {
            panic!("Expected OrderMergedOut payload");
        }

        if let EventPayload::OrderMerged {
            source_table_id,
            source_table_name,
            payments,
            paid_amount,
            ..
        } = &events[1].payload
        {
            assert_eq!(source_table_id, "");
            assert_eq!(source_table_name, "");
            assert!(payments.is_empty());
            assert_eq!(*paid_amount, 0.0);
        } else {
            panic!("Expected OrderMerged payload");
        }
    }

    #[tokio::test]
    async fn test_merge_orders_source_has_payment_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut source = create_active_order("source-1", "dining_table:t1", "Table 1");
        source.paid_amount = 5.0;
        let target = create_active_order("target-1", "dining_table:t2", "Table 2");
        storage.store_snapshot(&txn, &source).unwrap();
        storage.store_snapshot(&txn, &target).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MergeOrdersAction {
            source_order_id: "source-1".to_string(),
            target_order_id: "target-1".to_string(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_merge_orders_target_has_payment_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let source = create_active_order("source-1", "dining_table:t1", "Table 1");
        let mut target = create_active_order("target-1", "dining_table:t2", "Table 2");
        target.paid_amount = 15.0;
        storage.store_snapshot(&txn, &source).unwrap();
        storage.store_snapshot(&txn, &target).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MergeOrdersAction {
            source_order_id: "source-1".to_string(),
            target_order_id: "target-1".to_string(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_merge_rejects_source_with_aa_split() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut source = create_active_order("source-1", "dining_table:t1", "Table 1");
        source.aa_total_shares = Some(3);
        let target = create_active_order("target-1", "dining_table:t2", "Table 2");
        storage.store_snapshot(&txn, &source).unwrap();
        storage.store_snapshot(&txn, &target).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MergeOrdersAction {
            source_order_id: "source-1".to_string(),
            target_order_id: "target-1".to_string(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let result = action.execute(&mut ctx, &create_test_metadata()).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
        if let Err(OrderError::InvalidOperation(msg)) = result {
            assert!(msg.contains("AA 分单"));
        }
    }

    #[tokio::test]
    async fn test_merge_rejects_target_with_aa_split() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let source = create_active_order("source-1", "dining_table:t1", "Table 1");
        let mut target = create_active_order("target-1", "dining_table:t2", "Table 2");
        target.aa_total_shares = Some(2);
        storage.store_snapshot(&txn, &source).unwrap();
        storage.store_snapshot(&txn, &target).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = MergeOrdersAction {
            source_order_id: "source-1".to_string(),
            target_order_id: "target-1".to_string(),
            authorizer_id: None,
            authorizer_name: None,
        };

        let result = action.execute(&mut ctx, &create_test_metadata()).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
        if let Err(OrderError::InvalidOperation(msg)) = result {
            assert!(msg.contains("AA 分单"));
        }
    }
}
