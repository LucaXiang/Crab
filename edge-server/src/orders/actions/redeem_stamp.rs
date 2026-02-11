//! RedeemStamp command handler
//!
//! Emits a single StampRedeemed event. The applier adds the reward item
//! (always as a new comped line) and records it in snapshot.stamp_redemptions.
//! MemberUnlinked applier reverses any pending redemptions.
//! Stamps are consumed only on order completion (track_stamps_on_completion).

use async_trait::async_trait;

use crate::marketing::stamp_tracker::{self, StampItemInfo};
use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::models::{RewardStrategy, StampActivity, StampRewardTarget};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// Product info for the reward item (injected by OrdersManager)
#[derive(Debug, Clone)]
pub struct RewardProductInfo {
    pub product_id: i64,
    pub name: String,
    pub price: f64,
    pub tax_rate: i32,
    pub category_id: Option<i64>,
    pub category_name: Option<String>,
}

/// RedeemStamp action
#[derive(Debug, Clone)]
pub struct RedeemStampAction {
    pub order_id: String,
    pub stamp_activity_id: i64,
    /// Selection mode product_id (Eco/Gen selection or Designated)
    pub product_id: Option<i64>,
    /// Match mode: comp an existing item instead of adding a new one
    pub comp_existing_instance_id: Option<String>,
    /// Injected by OrdersManager
    pub activity: StampActivity,
    /// Injected by OrdersManager
    pub reward_targets: Vec<StampRewardTarget>,
    /// Product info for the reward (injected by OrdersManager)
    pub reward_product_info: Option<RewardProductInfo>,
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

        // 4. Check this activity hasn't already been redeemed in this order
        if snapshot
            .stamp_redemptions
            .iter()
            .any(|r| r.stamp_activity_id == self.stamp_activity_id)
        {
            return Err(OrderError::InvalidOperation(format!(
                "Stamp activity {} already redeemed in this order",
                self.stamp_activity_id
            )));
        }

        // 5. Resolve reward product info based on mode
        //
        // Three modes:
        // A) Match mode (comp_existing_instance_id = Some): comp an existing order item
        // B) Selection mode (product_id = Some, Eco/Gen): add a new item from reward_targets
        // C) Direct mode (Designated): add the designated product
        // D) Auto-match mode (Eco/Gen, no product_id, no comp_existing): find best match from order

        let comp_existing = self.comp_existing_instance_id.clone();

        let (info, reward_instance_id) = if let Some(ref existing_id) = comp_existing {
            // Match mode: comp an existing item
            let item = snapshot
                .items
                .iter()
                .find(|i| i.instance_id == *existing_id)
                .ok_or_else(|| {
                    OrderError::InvalidOperation(format!(
                        "Item {} not found in order",
                        existing_id
                    ))
                })?;

            if item.is_comped {
                return Err(OrderError::InvalidOperation(format!(
                    "Item {} is already comped",
                    existing_id
                )));
            }

            // Validate the item matches reward_targets
            let matches_target = self.reward_targets.iter().any(|t| match t.target_type {
                shared::models::StampTargetType::Product => t.target_id == item.id,
                shared::models::StampTargetType::Category => {
                    item.category_id == Some(t.target_id)
                }
            });
            if !matches_target {
                return Err(OrderError::InvalidOperation(
                    "Item does not match reward targets".to_string(),
                ));
            }

            let product_info = RewardProductInfo {
                product_id: item.id,
                name: item.name.clone(),
                price: item.original_price,
                tax_rate: item.tax_rate,
                category_id: item.category_id,
                category_name: item.category_name.clone(),
            };
            // Use existing item's instance_id as reward_instance_id
            (product_info, existing_id.clone())
        } else if self.product_id.is_some() && self.activity.reward_strategy != RewardStrategy::Designated {
            // Selection mode (Eco/Gen + explicit product_id): add a new item
            let info = self.reward_product_info.clone().ok_or_else(|| {
                OrderError::InvalidOperation(
                    "Product info not available for selection mode".to_string(),
                )
            })?;
            let rid = format!("stamp_reward::{}", metadata.command_id);
            (info, rid)
        } else if self.activity.reward_strategy == RewardStrategy::Designated {
            // Direct mode: designated product
            let product_id = self
                .product_id
                .or(self.activity.designated_product_id)
                .ok_or_else(|| {
                    OrderError::InvalidOperation(
                        "Designated strategy requires a product_id".to_string(),
                    )
                })?;
            let info = self.reward_product_info.clone().ok_or_else(|| {
                OrderError::InvalidOperation(format!(
                    "Product info not available for designated product {}",
                    product_id
                ))
            })?;
            let rid = format!("stamp_reward::{}", metadata.command_id);
            (info, rid)
        } else {
            // Auto-match mode (Eco/Gen, no explicit selection): find best match from order
            let items_with_category: Vec<StampItemInfo<'_>> = snapshot
                .items
                .iter()
                .map(|item| StampItemInfo {
                    item,
                    category_id: item.category_id,
                })
                .collect();

            let found_id = stamp_tracker::find_reward_item(
                &items_with_category,
                &self.reward_targets,
                &self.activity.reward_strategy,
            )
            .ok_or_else(|| {
                OrderError::InvalidOperation(
                    "No qualifying item found for stamp reward".to_string(),
                )
            })?;

            let item = snapshot
                .items
                .iter()
                .find(|i| i.instance_id == found_id)
                .ok_or_else(|| {
                    OrderError::InvalidOperation(
                        "Reward item not found in snapshot".to_string(),
                    )
                })?;

            let product_info = RewardProductInfo {
                product_id: item.id,
                name: item.name.clone(),
                price: item.original_price,
                tax_rate: item.tax_rate,
                category_id: item.category_id,
                category_name: item.category_name.clone(),
            };
            let rid = format!("stamp_reward::{}", metadata.command_id);
            (product_info, rid)
        };

        let comp_qty = self.activity.reward_quantity;

        let strategy_str = serde_json::to_value(&self.activity.reward_strategy)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();

        // 6. Generate single StampRedeemed event
        let event = OrderEvent::new(
            ctx.next_sequence(),
            self.order_id.clone(),
            metadata.operator_id,
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::StampRedeemed,
            EventPayload::StampRedeemed {
                stamp_activity_id: self.stamp_activity_id,
                stamp_activity_name: self.activity.display_name.clone(),
                reward_instance_id,
                reward_strategy: strategy_str,
                product_id: info.product_id,
                product_name: info.name.clone(),
                original_price: info.price,
                quantity: comp_qty,
                tax_rate: info.tax_rate,
                category_id: info.category_id,
                category_name: info.category_name.clone(),
                comp_existing_instance_id: comp_existing,
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
    use shared::models::StampTargetType;
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

    fn create_test_item(product_id: i64, instance_id: &str, price: f64, category_id: Option<i64>) -> CartItemSnapshot {
        CartItemSnapshot {
            id: product_id,
            instance_id: instance_id.to_string(),
            name: format!("Product {}", product_id),
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
            mg_discount_amount: 0.0,
            unit_price: price,
            line_total: price,
            tax: 0.0,
            tax_rate: 10,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_id,
            category_name: category_id.map(|id| format!("Cat-{}", id)),
            is_comped: false,
        }
    }

    #[tokio::test]
    async fn test_redeem_stamp_designated_emits_single_event() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.member_id = Some(42);
        snapshot.member_name = Some("Alice".to_string());
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let mut activity = create_test_activity(RewardStrategy::Designated);
        activity.designated_product_id = Some(100);

        let action = RedeemStampAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 1,
            product_id: None,
            comp_existing_instance_id: None,
            activity,
            reward_targets: vec![],
            reward_product_info: Some(RewardProductInfo {
                product_id: 100,
                name: "Coffee".to_string(),
                price: 3.50,
                tax_rate: 10,
                category_id: None,
                category_name: Some("Drinks".to_string()),
            }),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        // Single event
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, OrderEventType::StampRedeemed);

        if let EventPayload::StampRedeemed {
            stamp_activity_id,
            stamp_activity_name,
            reward_instance_id,
            reward_strategy,
            product_id,
            product_name,
            original_price,
            quantity,
            tax_rate,
            category_id: _,
            category_name,
            comp_existing_instance_id: _,
        } = &events[0].payload
        {
            assert_eq!(*stamp_activity_id, 1);
            assert_eq!(stamp_activity_name, "Coffee Card");
            assert!(reward_instance_id.starts_with("stamp_reward::"));
            assert_eq!(reward_strategy, "DESIGNATED");
            assert_eq!(*product_id, 100);
            assert_eq!(product_name, "Coffee");
            assert!((original_price - 3.50).abs() < f64::EPSILON);
            assert_eq!(*quantity, 1);
            assert_eq!(*tax_rate, 10);
            assert_eq!(category_name.as_deref(), Some("Drinks"));
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
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RedeemStampAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 1,
            product_id: None,
            comp_existing_instance_id: None,
            activity: create_test_activity(RewardStrategy::Economizador),
            reward_targets: vec![],
            reward_product_info: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_redeem_stamp_no_product_info_fails() {
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
            comp_existing_instance_id: None,
            activity,
            reward_targets: vec![],
            reward_product_info: None, // No info → error
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_redeem_stamp_economizador_category_match() {
        // Economizador picks the cheapest item matching a category reward target
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.member_id = Some(42);
        snapshot.member_name = Some("Alice".to_string());
        // Category 100: two items at different prices; Category 200: one item
        snapshot.items.push(create_test_item(1, "inst-1", 20.0, Some(100)));
        snapshot.items.push(create_test_item(2, "inst-2", 5.0, Some(100)));
        snapshot.items.push(create_test_item(3, "inst-3", 8.0, Some(200)));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RedeemStampAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 1,
            product_id: None,
            comp_existing_instance_id: None,
            activity: create_test_activity(RewardStrategy::Economizador),
            reward_targets: vec![StampRewardTarget {
                id: 1,
                stamp_activity_id: 1,
                target_type: StampTargetType::Category,
                target_id: 100, // Only category 100
            }],
            reward_product_info: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::StampRedeemed { product_id, original_price, category_id, .. } = &events[0].payload {
            // Should pick cheapest in category 100 → product 2 at $5.0
            assert_eq!(*product_id, 2);
            assert!((original_price - 5.0).abs() < f64::EPSILON);
            assert_eq!(*category_id, Some(100));
        } else {
            panic!("Expected StampRedeemed payload");
        }
    }

    #[tokio::test]
    async fn test_redeem_stamp_generoso_category_match() {
        // Generoso picks the most expensive item matching a category reward target
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.member_id = Some(42);
        snapshot.member_name = Some("Alice".to_string());
        snapshot.items.push(create_test_item(1, "inst-1", 20.0, Some(100)));
        snapshot.items.push(create_test_item(2, "inst-2", 5.0, Some(100)));
        snapshot.items.push(create_test_item(3, "inst-3", 50.0, Some(200)));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RedeemStampAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 1,
            product_id: None,
            comp_existing_instance_id: None,
            activity: create_test_activity(RewardStrategy::Generoso),
            reward_targets: vec![StampRewardTarget {
                id: 1,
                stamp_activity_id: 1,
                target_type: StampTargetType::Category,
                target_id: 100,
            }],
            reward_product_info: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::StampRedeemed { product_id, original_price, category_id, .. } = &events[0].payload {
            // Should pick most expensive in category 100 → product 1 at $20.0
            assert_eq!(*product_id, 1);
            assert!((original_price - 20.0).abs() < f64::EPSILON);
            assert_eq!(*category_id, Some(100));
        } else {
            panic!("Expected StampRedeemed payload");
        }
    }

    #[tokio::test]
    async fn test_redeem_stamp_product_target_match() {
        // Product-type reward target matches by product_id
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.member_id = Some(42);
        snapshot.member_name = Some("Alice".to_string());
        snapshot.items.push(create_test_item(10, "inst-1", 15.0, Some(100)));
        snapshot.items.push(create_test_item(20, "inst-2", 25.0, Some(100)));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RedeemStampAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 1,
            product_id: None,
            comp_existing_instance_id: None,
            activity: create_test_activity(RewardStrategy::Economizador),
            reward_targets: vec![StampRewardTarget {
                id: 1,
                stamp_activity_id: 1,
                target_type: StampTargetType::Product,
                target_id: 20, // Only product 20
            }],
            reward_product_info: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::StampRedeemed { product_id, original_price, .. } = &events[0].payload {
            // Only product 20 matches, regardless of strategy
            assert_eq!(*product_id, 20);
            assert!((original_price - 25.0).abs() < f64::EPSILON);
        } else {
            panic!("Expected StampRedeemed payload");
        }
    }

    #[tokio::test]
    async fn test_redeem_stamp_no_matching_category_fails() {
        // No items match the reward target category → error
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.member_id = Some(42);
        snapshot.member_name = Some("Alice".to_string());
        snapshot.items.push(create_test_item(1, "inst-1", 10.0, Some(100)));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RedeemStampAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 1,
            product_id: None,
            comp_existing_instance_id: None,
            activity: create_test_activity(RewardStrategy::Economizador),
            reward_targets: vec![StampRewardTarget {
                id: 1,
                stamp_activity_id: 1,
                target_type: StampTargetType::Category,
                target_id: 999, // No item has this category
            }],
            reward_product_info: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_redeem_stamp_skips_comped_items() {
        // Comped items should not be eligible for reward
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.member_id = Some(42);
        snapshot.member_name = Some("Alice".to_string());
        let mut comped_item = create_test_item(1, "inst-1", 5.0, Some(100));
        comped_item.is_comped = true;
        snapshot.items.push(comped_item);
        snapshot.items.push(create_test_item(2, "inst-2", 15.0, Some(100)));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RedeemStampAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 1,
            product_id: None,
            comp_existing_instance_id: None,
            activity: create_test_activity(RewardStrategy::Economizador),
            reward_targets: vec![StampRewardTarget {
                id: 1,
                stamp_activity_id: 1,
                target_type: StampTargetType::Category,
                target_id: 100,
            }],
            reward_product_info: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::StampRedeemed { product_id, .. } = &events[0].payload {
            // Comped item (product 1) skipped, picks product 2
            assert_eq!(*product_id, 2);
        } else {
            panic!("Expected StampRedeemed payload");
        }
    }

    #[tokio::test]
    async fn test_redeem_stamp_completed_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        snapshot.member_id = Some(42);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RedeemStampAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 1,
            product_id: None,
            comp_existing_instance_id: None,
            activity: create_test_activity(RewardStrategy::Economizador),
            reward_targets: vec![],
            reward_product_info: None,
        };

        let result = action.execute(&mut ctx, &create_test_metadata()).await;
        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[tokio::test]
    async fn test_redeem_stamp_voided_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        snapshot.member_id = Some(42);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RedeemStampAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 1,
            product_id: None,
            comp_existing_instance_id: None,
            activity: create_test_activity(RewardStrategy::Economizador),
            reward_targets: vec![],
            reward_product_info: None,
        };

        let result = action.execute(&mut ctx, &create_test_metadata()).await;
        assert!(matches!(result, Err(OrderError::OrderAlreadyVoided(_))));
    }

    #[tokio::test]
    async fn test_redeem_stamp_category_id_propagated_to_event() {
        // Verify category_id from snapshot item flows through to the event payload
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.member_id = Some(42);
        snapshot.member_name = Some("Alice".to_string());
        snapshot.items.push(create_test_item(1, "inst-1", 10.0, Some(777)));
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = RedeemStampAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 1,
            product_id: None,
            comp_existing_instance_id: None,
            activity: create_test_activity(RewardStrategy::Generoso),
            reward_targets: vec![StampRewardTarget {
                id: 1,
                stamp_activity_id: 1,
                target_type: StampTargetType::Category,
                target_id: 777,
            }],
            reward_product_info: None,
        };

        let events = action.execute(&mut ctx, &create_test_metadata()).await.unwrap();

        if let EventPayload::StampRedeemed { category_id, category_name, .. } = &events[0].payload {
            assert_eq!(*category_id, Some(777));
            assert_eq!(category_name.as_deref(), Some("Cat-777"));
        } else {
            panic!("Expected StampRedeemed payload");
        }
    }

    #[tokio::test]
    async fn test_redeem_stamp_duplicate_activity_rejected() {
        // Same stamp activity cannot be redeemed twice in the same order
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.member_id = Some(42);
        snapshot.member_name = Some("Alice".to_string());
        // Simulate a previous redemption already recorded
        snapshot.stamp_redemptions.push(shared::order::StampRedemptionState {
            stamp_activity_id: 1,
            reward_instance_id: "stamp_reward::prev-cmd".to_string(),
            is_comp_existing: false,
        });
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let mut activity = create_test_activity(RewardStrategy::Designated);
        activity.designated_product_id = Some(100);

        let action = RedeemStampAction {
            order_id: "order-1".to_string(),
            stamp_activity_id: 1, // Same activity as already redeemed
            product_id: None,
            comp_existing_instance_id: None,
            activity,
            reward_targets: vec![],
            reward_product_info: Some(RewardProductInfo {
                product_id: 100,
                name: "Coffee".to_string(),
                price: 3.50,
                tax_rate: 10,
                category_id: None,
                category_name: None,
            }),
        };

        let result = action.execute(&mut ctx, &create_test_metadata()).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));

        // Verify error message mentions "already redeemed"
        if let Err(OrderError::InvalidOperation(msg)) = result {
            assert!(msg.contains("already redeemed"), "Expected 'already redeemed' in: {msg}");
        }
    }
}
