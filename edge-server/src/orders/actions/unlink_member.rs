//! UnlinkMember command handler
//!
//! Unlinks a member from an order, clearing member info and MG discounts.

use async_trait::async_trait;

use shared::order::types::CommandErrorCode;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// UnlinkMember action
#[derive(Debug, Clone)]
pub struct UnlinkMemberAction {
    pub order_id: String,
}

#[async_trait]
impl CommandHandler for UnlinkMemberAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Load snapshot
        let snapshot = ctx.load_snapshot(&self.order_id)?;

        // 2. Validate order status
        match snapshot.status {
            OrderStatus::Active => {}
            OrderStatus::Completed => {
                return Err(OrderError::OrderAlreadyCompleted(self.order_id.clone()));
            }
            OrderStatus::Void => {
                return Err(OrderError::OrderAlreadyVoided(self.order_id.clone()));
            }
            _ => {
                return Err(OrderError::InvalidOperation(CommandErrorCode::OrderNotActive, format!(
                    "Cannot unlink member on order with status: {:?}",
                    snapshot.status
                )));
            }
        }

        // 3. Must have a member linked
        if snapshot.member_id.is_none() {
            return Err(OrderError::InvalidOperation(CommandErrorCode::NoMemberLinked,
                "No member linked to this order".to_string(),
            ));
        }

        // 4. Block during active split payments
        if snapshot.aa_total_shares.is_some() {
            return Err(OrderError::InvalidOperation(CommandErrorCode::AaSplitActive,
                "Cannot unlink member during AA split".to_string(),
            ));
        }
        if snapshot.has_amount_split {
            return Err(OrderError::InvalidOperation(CommandErrorCode::AmountSplitActive,
                "Cannot unlink member during amount split".to_string(),
            ));
        }

        // 5. Extract previous member info
        // SAFETY: checked is_none() above
        let previous_member_id = snapshot.member_id.unwrap();
        let previous_member_name = snapshot.member_name.clone().unwrap_or_default();

        // 6. Generate event
        let seq = ctx.next_sequence();
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id,
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::MemberUnlinked,
            EventPayload::MemberUnlinked {
                previous_member_id,
                previous_member_name,
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

    #[tokio::test]
    async fn test_unlink_member_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.member_id = Some(42);
        snapshot.member_name = Some("Alice".to_string());
        snapshot.marketing_group_id = Some(1);
        snapshot.marketing_group_name = Some("VIP".to_string());
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UnlinkMemberAction {
            order_id: "order-1".to_string(),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.event_type, OrderEventType::MemberUnlinked);

        if let EventPayload::MemberUnlinked {
            previous_member_id,
            previous_member_name,
        } = &event.payload
        {
            assert_eq!(*previous_member_id, 42);
            assert_eq!(previous_member_name, "Alice");
        } else {
            panic!("Expected MemberUnlinked payload");
        }
    }

    #[tokio::test]
    async fn test_unlink_member_no_member_linked_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        // No member linked
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UnlinkMemberAction {
            order_id: "order-1".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(..))));
    }

    #[tokio::test]
    async fn test_unlink_member_completed_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        snapshot.member_id = Some(42);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UnlinkMemberAction {
            order_id: "order-1".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[tokio::test]
    async fn test_unlink_member_blocked_during_aa_split() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.member_id = Some(42);
        snapshot.member_name = Some("Alice".to_string());
        snapshot.aa_total_shares = Some(3);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = UnlinkMemberAction {
            order_id: "order-1".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(..))));
    }
}
