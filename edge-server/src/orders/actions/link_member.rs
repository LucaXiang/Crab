//! LinkMember command handler
//!
//! Links a member to an order and records the association.
//! MG discount calculation is done by the applier via recalculate_totals.

use async_trait::async_trait;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::models::MgDiscountRule;
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// LinkMember action
#[derive(Debug, Clone)]
pub struct LinkMemberAction {
    pub order_id: String,
    pub member_id: i64,
    pub member_name: String,
    pub marketing_group_id: i64,
    pub marketing_group_name: String,
    /// Active MG discount rules, injected by OrdersManager
    pub mg_rules: Vec<MgDiscountRule>,
}

#[async_trait]
impl CommandHandler for LinkMemberAction {
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
                return Err(OrderError::InvalidOperation(format!(
                    "Cannot link member on order with status: {:?}",
                    snapshot.status
                )));
            }
        }

        // 3. Block during active split payments
        if snapshot.aa_total_shares.is_some() {
            return Err(OrderError::InvalidOperation(
                "Cannot link member during AA split".to_string(),
            ));
        }
        if snapshot.has_amount_split {
            return Err(OrderError::InvalidOperation(
                "Cannot link member during amount split".to_string(),
            ));
        }

        // 4. Generate event
        let seq = ctx.next_sequence();
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id,
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::MemberLinked,
            EventPayload::MemberLinked {
                member_id: self.member_id,
                member_name: self.member_name.clone(),
                marketing_group_id: self.marketing_group_id,
                marketing_group_name: self.marketing_group_name.clone(),
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
    async fn test_link_member_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = LinkMemberAction {
            order_id: "order-1".to_string(),
            member_id: 42,
            member_name: "Alice".to_string(),
            marketing_group_id: 1,
            marketing_group_name: "VIP".to_string(),
            mg_rules: vec![],
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.order_id, "order-1");
        assert_eq!(event.event_type, OrderEventType::MemberLinked);

        if let EventPayload::MemberLinked {
            member_id,
            member_name,
            marketing_group_id,
            marketing_group_name,
        } = &event.payload
        {
            assert_eq!(*member_id, 42);
            assert_eq!(member_name, "Alice");
            assert_eq!(*marketing_group_id, 1);
            assert_eq!(marketing_group_name, "VIP");
        } else {
            panic!("Expected MemberLinked payload");
        }
    }

    #[tokio::test]
    async fn test_link_member_completed_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = LinkMemberAction {
            order_id: "order-1".to_string(),
            member_id: 42,
            member_name: "Alice".to_string(),
            marketing_group_id: 1,
            marketing_group_name: "VIP".to_string(),
            mg_rules: vec![],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[tokio::test]
    async fn test_link_member_voided_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = LinkMemberAction {
            order_id: "order-1".to_string(),
            member_id: 42,
            member_name: "Alice".to_string(),
            marketing_group_id: 1,
            marketing_group_name: "VIP".to_string(),
            mg_rules: vec![],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::OrderAlreadyVoided(_))));
    }

    #[tokio::test]
    async fn test_link_member_blocked_during_aa_split() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.aa_total_shares = Some(3);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = LinkMemberAction {
            order_id: "order-1".to_string(),
            member_id: 42,
            member_name: "Alice".to_string(),
            marketing_group_id: 1,
            marketing_group_name: "VIP".to_string(),
            mg_rules: vec![],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_link_member_blocked_during_amount_split() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.has_amount_split = true;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = LinkMemberAction {
            order_id: "order-1".to_string(),
            member_id: 42,
            member_name: "Alice".to_string(),
            marketing_group_id: 1,
            marketing_group_name: "VIP".to_string(),
            mg_rules: vec![],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_link_member_order_not_found() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = LinkMemberAction {
            order_id: "nonexistent".to_string(),
            member_id: 42,
            member_name: "Alice".to_string(),
            marketing_group_id: 1,
            marketing_group_name: "VIP".to_string(),
            mg_rules: vec![],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }
}
