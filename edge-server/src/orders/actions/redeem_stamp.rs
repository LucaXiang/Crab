//! RedeemStamp command handler
//!
//! Redeems a stamp reward for a member, comping a qualifying item.
//! The stamp activity, reward targets, and strategy are injected by OrdersManager.

use async_trait::async_trait;

use crate::marketing::stamp_tracker::{self, StampItemInfo};
use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::models::{RewardStrategy, StampActivity, StampRewardTarget};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// RedeemStamp action
#[derive(Debug, Clone)]
pub struct RedeemStampAction {
    pub order_id: String,
    pub stamp_activity_id: i64,
    /// Designated product_id (only for Designated strategy)
    pub product_id: Option<i64>,
    /// Injected by OrdersManager
    pub activity: StampActivity,
    /// Injected by OrdersManager
    pub reward_targets: Vec<StampRewardTarget>,
}

#[async_trait]
impl CommandHandler for RedeemStampAction {
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
                    "Cannot redeem stamp on order with status: {:?}",
                    snapshot.status
                )));
            }
        }

        // 3. Must have a member linked
        if snapshot.member_id.is_none() {
            return Err(OrderError::InvalidOperation(
                "Must have a member linked to redeem stamps".to_string(),
            ));
        }

        // 4. Find reward item based on strategy
        let items_with_category: Vec<StampItemInfo<'_>> = snapshot
            .items
            .iter()
            .map(|item| StampItemInfo {
                item,
                category_id: None, // category_id not available in snapshot
            })
            .collect();

        let reward_item_id = match self.activity.reward_strategy {
            RewardStrategy::Designated => {
                // For Designated strategy, find item by product_id
                let product_id = self.product_id.or(self.activity.designated_product_id).ok_or_else(|| {
                    OrderError::InvalidOperation(
                        "Designated strategy requires a product_id".to_string(),
                    )
                })?;
                snapshot
                    .items
                    .iter()
                    .find(|i| i.id == product_id && !i.is_comped)
                    .map(|i| i.instance_id.clone())
                    .ok_or_else(|| {
                        OrderError::InvalidOperation(format!(
                            "Designated product {} not found or already comped",
                            product_id
                        ))
                    })?
            }
            _ => {
                // Economizador or Generoso: use stamp_tracker
                stamp_tracker::find_reward_item(
                    &items_with_category,
                    &self.reward_targets,
                    &self.activity.reward_strategy,
                )
                .ok_or_else(|| {
                    OrderError::InvalidOperation(
                        "No qualifying item found for stamp reward".to_string(),
                    )
                })?
            }
        };

        // 5. Generate StampRedeemed event
        let seq = ctx.next_sequence();
        let strategy_str = serde_json::to_value(&self.activity.reward_strategy)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();

        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id,
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::StampRedeemed,
            EventPayload::StampRedeemed {
                stamp_activity_id: self.stamp_activity_id,
                stamp_activity_name: self.activity.display_name.clone(),
                reward_item_id,
                reward_strategy: strategy_str,
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
    use shared::models::StampRewardTarget;
    use shared::order::{CartItemSnapshot, OrderSnapshot};

    fn create_test_metadata() -> CommandMetadata {
        CommandMetadata {
            command_id: "cmd-1".to_string(),
            operator_id: 1,
            operator_name: "Test User".to_string(),
            timestamp: 1234567890,
        }
    }

    fn create_test_activity(strategy: RewardStrategy) -> StampActivity {
        StampActivity {
            id: 1,
            marketing_group_id: 1,
            name: "coffee_card".to_string(),
            display_name: "Coffee Card".to_string(),
            stamps_required: 10,
            reward_quantity: 1,
            reward_strategy: strategy,
            designated_product_id: None,
            is_cyclic: true,
            is_active: true,
            created_at: 0,
            updated_at: 0,
        }
    }

    fn create_test_item(
        product_id: i64,
        instance_id: &str,
        name: &str,
        price: f64,
    ) -> CartItemSnapshot {
        CartItemSnapshot {
            id: product_id,
            instance_id: instance_id.to_string(),
            name: name.to_string(),
            price,
            original_price: price,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: 0.0,
            rule_surcharge_amount: 0.0,
            applied_rules: vec![],
            applied_mg_rules: vec![],
            unit_price: price,
            line_total: price,
            tax: 0.0,
            tax_rate: 0,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
        }
    }

    #[tokio::test]
    async fn test_redeem_stamp_designated_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.member_id = Some(42);
        snapshot.member_name = Some("Alice".to_string());
        snapshot.items.push(create_test_item(100, "inst-1", "Coffee", 3.50));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let mut activity = create_test_activity(RewardStrategy::Designated);
        activity.designated_product_id = Some(100);

        let action = RedeemStampAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 1,
            product_id: None,
            activity,
            reward_targets: vec![],
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, OrderEventType::StampRedeemed);

        if let EventPayload::StampRedeemed {
            stamp_activity_id,
            stamp_activity_name,
            reward_item_id,
            reward_strategy,
        } = &events[0].payload
        {
            assert_eq!(*stamp_activity_id, 1);
            assert_eq!(stamp_activity_name, "Coffee Card");
            assert_eq!(reward_item_id, "inst-1");
            assert_eq!(reward_strategy, "DESIGNATED");
        } else {
            panic!("Expected StampRedeemed payload");
        }
    }

    #[tokio::test]
    async fn test_redeem_stamp_no_member_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        // No member linked
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RedeemStampAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 1,
            product_id: None,
            activity: create_test_activity(RewardStrategy::Economizador),
            reward_targets: vec![],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_redeem_stamp_economizador_no_matching_item_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.member_id = Some(42);
        // Add item that won't match any reward target
        snapshot.items.push(create_test_item(999, "inst-1", "Something", 5.0));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RedeemStampAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 1,
            product_id: None,
            activity: create_test_activity(RewardStrategy::Economizador),
            reward_targets: vec![StampRewardTarget {
                id: 1,
                stamp_activity_id: 1,
                target_type: shared::models::StampTargetType::Product,
                target_id: 100, // doesn't match product 999
            }],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_redeem_stamp_designated_product_not_found_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.member_id = Some(42);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let mut activity = create_test_activity(RewardStrategy::Designated);
        activity.designated_product_id = Some(100);

        let action = RedeemStampAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 1,
            product_id: None,
            activity,
            reward_targets: vec![],
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }
}
