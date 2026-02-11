//! LinkMember command handler
//!
//! Links a member to an order and calculates MG discounts for existing items.
//! MG discount calculation is done here (CommandHandler = can access metadata),
//! results are stored in the event payload for the pure-function applier.

use async_trait::async_trait;
use std::collections::HashMap;

use shared::order::types::CommandErrorCode;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use crate::services::catalog_service::ProductMeta;
use shared::models::MgDiscountRule;
use shared::order::{EventPayload, MgItemDiscount, OrderEvent, OrderEventType, OrderStatus};

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
    /// Product metadata for MG rule scope matching (category_id)
    pub product_metadata: HashMap<i64, ProductMeta>,
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
                return Err(OrderError::InvalidOperation(CommandErrorCode::OrderNotActive, format!(
                    "Cannot link member on order with status: {:?}",
                    snapshot.status
                )));
            }
        }

        // 3. Block if a member is already linked (must unlink first)
        if snapshot.member_id.is_some() {
            return Err(OrderError::InvalidOperation(CommandErrorCode::MemberAlreadyLinked,
                "A member is already linked to this order. Unlink first.".to_string(),
            ));
        }

        // 4. Block during active split payments
        if snapshot.aa_total_shares.is_some() {
            return Err(OrderError::InvalidOperation(CommandErrorCode::AaSplitActive,
                "Cannot link member during AA split".to_string(),
            ));
        }
        if snapshot.has_amount_split {
            return Err(OrderError::InvalidOperation(CommandErrorCode::AmountSplitActive,
                "Cannot link member during amount split".to_string(),
            ));
        }

        // 5. Calculate MG discounts for existing items
        let mg_item_discounts: Vec<MgItemDiscount> = if self.mg_rules.is_empty() {
            vec![]
        } else {
            snapshot
                .items
                .iter()
                .filter(|item| !item.is_comped)
                .filter_map(|item| {
                    let category_id = self
                        .product_metadata
                        .get(&item.id)
                        .map(|m| m.category_id);
                    let result = crate::marketing::mg_calculator::calculate_mg_discount(
                        item.unit_price,
                        item.id,
                        category_id,
                        &self.mg_rules,
                    );
                    if result.applied_rules.is_empty() {
                        None
                    } else {
                        Some(MgItemDiscount {
                            instance_id: item.instance_id.clone(),
                            applied_mg_rules: result.applied_rules,
                        })
                    }
                })
                .collect()
        };

        // 6. Generate event
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
                mg_item_discounts,
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
    use shared::models::price_rule::{AdjustmentType, ProductScope};
    use shared::order::OrderSnapshot;

    fn create_test_metadata() -> CommandMetadata {
        CommandMetadata {
            command_id: "cmd-1".to_string(),
            operator_id: 1,
            operator_name: "Test User".to_string(),
            timestamp: 1234567890,
        }
    }

    fn make_mg_rule(
        id: i64,
        product_scope: ProductScope,
        target_id: Option<i64>,
        adjustment_type: AdjustmentType,
        adjustment_value: f64,
    ) -> MgDiscountRule {
        MgDiscountRule {
            id,
            marketing_group_id: 1,
            name: format!("rule_{}", id),
            display_name: format!("Rule {}", id),
            receipt_name: format!("R{}", id),
            product_scope,
            target_id,
            adjustment_type,
            adjustment_value,
            is_active: true,
            created_at: 0,
            updated_at: 0,
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
            product_metadata: HashMap::new(),
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
            mg_item_discounts,
        } = &event.payload
        {
            assert_eq!(*member_id, 42);
            assert_eq!(member_name, "Alice");
            assert_eq!(*marketing_group_id, 1);
            assert_eq!(marketing_group_name, "VIP");
            assert!(mg_item_discounts.is_empty());
        } else {
            panic!("Expected MemberLinked payload");
        }
    }

    #[tokio::test]
    async fn test_link_member_no_retroactive_mg_discounts() {
        // MG discounts are NOT applied retroactively to existing items.
        // Only items added after member link get MG discounts (via AddItems).
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items.push(shared::order::CartItemSnapshot {
            id: 100,
            instance_id: "inst-100".to_string(),
            name: "Steak".to_string(),
            price: 50.0,
            original_price: 50.0,
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
            unit_price: 50.0,
            line_total: 50.0,
            tax: 0.0,
            tax_rate: 0,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_id: None,
            category_name: None,
            is_comped: false,
        });
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = LinkMemberAction {
            order_id: "order-1".to_string(),
            member_id: 42,
            member_name: "Alice".to_string(),
            marketing_group_id: 1,
            marketing_group_name: "VIP".to_string(),
            mg_rules: vec![make_mg_rule(
                1,
                ProductScope::Global,
                None,
                AdjustmentType::Percentage,
                10.0,
            )],
            product_metadata: HashMap::new(),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        if let EventPayload::MemberLinked {
            mg_item_discounts, ..
        } = &events[0].payload
        {
            // Global 10% rule applies to existing item (unit_price=50 → discount=5)
            assert_eq!(mg_item_discounts.len(), 1);
            assert_eq!(mg_item_discounts[0].instance_id, "inst-100");
            assert_eq!(mg_item_discounts[0].applied_mg_rules.len(), 1);
            assert_eq!(mg_item_discounts[0].applied_mg_rules[0].calculated_amount, 5.0);
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
            product_metadata: HashMap::new(),
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
            product_metadata: HashMap::new(),
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
            product_metadata: HashMap::new(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(..))));
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
            product_metadata: HashMap::new(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(..))));
    }

    #[tokio::test]
    async fn test_link_member_blocked_when_already_linked() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.member_id = Some(10);
        snapshot.member_name = Some("Bob".to_string());
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
            product_metadata: HashMap::new(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(..))));
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
            product_metadata: HashMap::new(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }
}
