//! CancelStampRedemption command handler
//!
//! Emits a single StampRedemptionCancelled event. The applier removes the reward
//! item and the stamp_redemption record from the snapshot.

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::types::CommandErrorCode;
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// CancelStampRedemption action
#[derive(Debug, Clone)]
pub struct CancelStampRedemptionAction {
    pub order_id: String,
    pub stamp_activity_id: i64,
}

impl CommandHandler for CancelStampRedemptionAction {
    fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        let snapshot = ctx.load_snapshot(&self.order_id)?;

        // Validate order status
        match snapshot.status {
            OrderStatus::Active => {}
            OrderStatus::Completed => {
                return Err(OrderError::OrderAlreadyCompleted(self.order_id.clone()));
            }
            OrderStatus::Void => {
                return Err(OrderError::OrderAlreadyVoided(self.order_id.clone()));
            }
            _ => {
                return Err(OrderError::InvalidOperation(
                    CommandErrorCode::OrderNotActive,
                    format!(
                        "Cannot cancel stamp redemption on order with status: {:?}",
                        snapshot.status
                    ),
                ));
            }
        }

        // Find the redemption record
        let redemption = snapshot
            .stamp_redemptions
            .iter()
            .find(|r| r.stamp_activity_id == self.stamp_activity_id)
            .ok_or_else(|| {
                OrderError::InvalidOperation(
                    CommandErrorCode::StampRedemptionNotFound,
                    format!(
                        "No stamp redemption found for activity {} in this order",
                        self.stamp_activity_id
                    ),
                )
            })?;

        // Get the stamp activity name from the reward item
        let reward_item = snapshot
            .items
            .iter()
            .find(|i| i.instance_id == redemption.reward_instance_id);

        let stamp_activity_name = reward_item.map(|i| i.name.clone()).unwrap_or_default();

        let reward_instance_id = redemption.reward_instance_id.clone();
        let is_comp_existing = redemption.is_comp_existing;
        let comp_source_instance_id = redemption.comp_source_instance_id.clone();

        let event = OrderEvent::new(
            ctx.next_sequence(),
            self.order_id.clone(),
            metadata.operator_id,
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::StampRedemptionCancelled,
            EventPayload::StampRedemptionCancelled {
                stamp_activity_id: self.stamp_activity_id,
                stamp_activity_name,
                reward_instance_id,
                is_comp_existing,
                comp_source_instance_id,
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
    use shared::order::{CartItemSnapshot, OrderSnapshot, StampRedemptionState};

    fn create_test_metadata() -> CommandMetadata {
        CommandMetadata {
            command_id: "cmd-1".to_string(),
            operator_id: 1,
            operator_name: "Test User".to_string(),
            timestamp: 1234567890,
        }
    }

    fn create_reward_item(instance_id: &str) -> CartItemSnapshot {
        CartItemSnapshot {
            id: 100,
            instance_id: instance_id.to_string(),
            name: "Coffee".to_string(),
            price: 0.0,
            original_price: 3.50,
            quantity: 1,
            unpaid_quantity: 0,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: 0.0,
            rule_surcharge_amount: 0.0,
            applied_rules: vec![],
            applied_mg_rules: vec![],
            mg_discount_amount: 0.0,
            unit_price: 0.0,
            line_total: 0.0,
            tax: 0.0,
            tax_rate: 10,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_id: None,
            category_name: Some("Drinks".to_string()),
            is_comped: true,
        }
    }

    #[test]
    fn test_cancel_stamp_redemption_emits_event() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.member_id = Some(42);
        snapshot
            .items
            .push(create_reward_item("stamp_reward::prev-cmd"));
        snapshot.stamp_redemptions.push(StampRedemptionState {
            stamp_activity_id: 1,
            reward_instance_id: "stamp_reward::prev-cmd".to_string(),
            is_comp_existing: false,
            comp_source_instance_id: None,
        });
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelStampRedemptionAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 1,
        };

        let events = action.execute(&mut ctx, &create_test_metadata()).unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].event_type,
            OrderEventType::StampRedemptionCancelled
        );

        if let EventPayload::StampRedemptionCancelled {
            stamp_activity_id,
            reward_instance_id,
            ..
        } = &events[0].payload
        {
            assert_eq!(*stamp_activity_id, 1);
            assert_eq!(reward_instance_id, "stamp_reward::prev-cmd");
        } else {
            panic!("Expected StampRedemptionCancelled payload");
        }
    }

    #[test]
    fn test_cancel_stamp_redemption_no_redemption_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.member_id = Some(42);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelStampRedemptionAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 999,
        };

        let result = action.execute(&mut ctx, &create_test_metadata());
        assert!(matches!(result, Err(OrderError::InvalidOperation(..))));
    }

    #[test]
    fn test_cancel_stamp_redemption_completed_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = CancelStampRedemptionAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 1,
        };

        let result = action.execute(&mut ctx, &create_test_metadata());
        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }
}
