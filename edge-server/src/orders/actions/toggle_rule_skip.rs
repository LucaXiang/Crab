//! ToggleRuleSkip command handler

use async_trait::async_trait;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// ToggleRuleSkip action
#[derive(Debug, Clone)]
pub struct ToggleRuleSkipAction {
    pub order_id: String,
    pub rule_id: String,
    pub skipped: bool,
}

#[async_trait]
impl CommandHandler for ToggleRuleSkipAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Load snapshot
        let snapshot = ctx.load_snapshot(&self.order_id)?;

        // 2. Validate status
        if !matches!(snapshot.status, OrderStatus::Active) {
            return Err(OrderError::InvalidOperation(
                "Cannot toggle rule on non-active order".to_string(),
            ));
        }

        // 3. Find rule in the order and get its name
        let rule_name = snapshot
            .items
            .iter()
            .filter_map(|item| item.applied_rules.as_ref())
            .flatten()
            .find(|r| r.rule_id == self.rule_id)
            .or_else(|| {
                snapshot
                    .order_applied_rules
                    .as_ref()
                    .and_then(|rules| rules.iter().find(|r| r.rule_id == self.rule_id))
            })
            .map(|r| r.display_name.clone());

        let Some(rule_name) = rule_name else {
            return Err(OrderError::InvalidOperation(format!(
                "Rule {} not found in order",
                self.rule_id
            )));
        };

        // 4. Generate event (actual toggle and recalculation will be done by applier)
        let seq = ctx.next_sequence();
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::RuleSkipToggled,
            EventPayload::RuleSkipToggled {
                rule_id: self.rule_id.clone(),
                rule_name,
                skipped: self.skipped,
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
    use shared::models::price_rule::{AdjustmentType, ProductScope, RuleType};
    use shared::order::{AppliedRule, CartItemSnapshot, OrderSnapshot};

    fn create_test_metadata() -> CommandMetadata {
        CommandMetadata {
            command_id: "cmd-1".to_string(),
            operator_id: "user-1".to_string(),
            operator_name: "Test User".to_string(),
            timestamp: 1234567890,
        }
    }

    fn create_test_applied_rule(rule_id: &str) -> AppliedRule {
        AppliedRule {
            rule_id: rule_id.to_string(),
            name: "test_rule".to_string(),
            display_name: "Test Rule".to_string(),
            receipt_name: "TEST".to_string(),
            rule_type: RuleType::Discount,
            adjustment_type: AdjustmentType::Percentage,
            product_scope: ProductScope::Global,
            zone_scope: "zone:all".to_string(),
            adjustment_value: 10.0,
            calculated_amount: 5.0,
            is_stackable: true,
            is_exclusive: false,
            skipped: false,
        }
    }

    fn create_test_item_with_rule(rule_id: &str) -> CartItemSnapshot {
        CartItemSnapshot {
            id: "product:p1".to_string(),
            instance_id: "item-1".to_string(),
            name: "Test Product".to_string(),
            price: 10.0,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: Some(vec![create_test_applied_rule(rule_id)]),
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
    async fn test_toggle_rule_skip_item_rule_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create an active order with item that has applied rule
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items = vec![create_test_item_with_rule("rule-1")];
        snapshot.subtotal = 10.0;
        snapshot.total = 10.0;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: "rule-1".to_string(),
            skipped: true,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.order_id, "order-1");
        assert_eq!(event.event_type, OrderEventType::RuleSkipToggled);

        if let EventPayload::RuleSkipToggled {
            rule_id,
            skipped,
            ..
        } = &event.payload
        {
            assert_eq!(*rule_id, "rule-1");
            assert!(*skipped);
        } else {
            panic!("Expected RuleSkipToggled payload");
        }
    }

    #[tokio::test]
    async fn test_toggle_rule_skip_order_rule_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create an active order with order-level applied rule
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.order_applied_rules = Some(vec![create_test_applied_rule("order-rule-1")]);
        snapshot.subtotal = 100.0;
        snapshot.discount = 10.0;
        snapshot.total = 90.0;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: "order-rule-1".to_string(),
            skipped: true,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.event_type, OrderEventType::RuleSkipToggled);

        if let EventPayload::RuleSkipToggled {
            rule_id,
            skipped,
            ..
        } = &event.payload
        {
            assert_eq!(*rule_id, "order-rule-1");
            assert!(*skipped);
        } else {
            panic!("Expected RuleSkipToggled payload");
        }
    }

    #[tokio::test]
    async fn test_toggle_rule_skip_rule_not_found() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create an active order without rules
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: "nonexistent-rule".to_string(),
            skipped: true,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
        if let Err(OrderError::InvalidOperation(msg)) = result {
            assert!(msg.contains("not found in order"));
        }
    }

    #[tokio::test]
    async fn test_toggle_rule_skip_non_active_order() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create a completed order
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        snapshot.items = vec![create_test_item_with_rule("rule-1")];
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: "rule-1".to_string(),
            skipped: true,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
        if let Err(OrderError::InvalidOperation(msg)) = result {
            assert!(msg.contains("non-active order"));
        }
    }

    #[tokio::test]
    async fn test_toggle_rule_skip_order_not_found() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ToggleRuleSkipAction {
            order_id: "nonexistent".to_string(),
            rule_id: "rule-1".to_string(),
            skipped: true,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    #[tokio::test]
    async fn test_toggle_rule_skip_unskip() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create an active order with item that has skipped rule
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        let mut rule = create_test_applied_rule("rule-1");
        rule.skipped = true;
        snapshot.items = vec![CartItemSnapshot {
            id: "product:p1".to_string(),
            instance_id: "item-1".to_string(),
            name: "Test Product".to_string(),
            price: 10.0,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: Some(vec![rule]),
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
        is_comped: false,
        }];
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Unskip the rule
        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: "rule-1".to_string(),
            skipped: false,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::RuleSkipToggled { skipped, .. } = &events[0].payload {
            assert!(!*skipped);
        } else {
            panic!("Expected RuleSkipToggled payload");
        }
    }

    #[tokio::test]
    async fn test_toggle_rule_skip_event_metadata() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items = vec![create_test_item_with_rule("rule-1")];
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: "rule-1".to_string(),
            skipped: true,
        };

        let metadata = CommandMetadata {
            command_id: "cmd-toggle-1".to_string(),
            operator_id: "manager-1".to_string(),
            operator_name: "Manager".to_string(),
            timestamp: 9999999999,
        };

        let events = action.execute(&mut ctx, &metadata).await.unwrap();
        let event = &events[0];

        assert_eq!(event.command_id, "cmd-toggle-1");
        assert_eq!(event.operator_id, "manager-1");
        assert_eq!(event.operator_name, "Manager");
        assert_eq!(event.client_timestamp, Some(9999999999));
    }

    #[tokio::test]
    async fn test_toggle_rule_skip_already_skipped_rule() {
        // Toggling skip on a rule that's already skipped should still succeed
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        let mut rule = create_test_applied_rule("rule-1");
        rule.skipped = true; // already skipped
        snapshot.items = vec![CartItemSnapshot {
            id: "product:p1".to_string(),
            instance_id: "item-1".to_string(),
            name: "Test Product".to_string(),
            price: 10.0,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: Some(vec![rule]),
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
        }];
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Skip again (idempotent-ish)
        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: "rule-1".to_string(),
            skipped: true,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::RuleSkipToggled { skipped, .. } = &events[0].payload {
            assert!(*skipped);
        } else {
            panic!("Expected RuleSkipToggled payload");
        }
    }

    #[tokio::test]
    async fn test_toggle_rule_skip_multiple_rules_on_item() {
        // Item has multiple rules, toggle one specific rule
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items = vec![CartItemSnapshot {
            id: "product:p1".to_string(),
            instance_id: "item-1".to_string(),
            name: "Test Product".to_string(),
            price: 10.0,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: Some(vec![
                create_test_applied_rule("rule-1"),
                create_test_applied_rule("rule-2"),
            ]),
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
        }];
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Toggle only rule-2
        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: "rule-2".to_string(),
            skipped: true,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();
        assert_eq!(events.len(), 1);

        if let EventPayload::RuleSkipToggled { rule_id, skipped, .. } = &events[0].payload {
            assert_eq!(*rule_id, "rule-2");
            assert!(*skipped);
        } else {
            panic!("Expected RuleSkipToggled payload");
        }
    }

    #[tokio::test]
    async fn test_toggle_rule_skip_rule_on_both_levels() {
        // Same rule_id exists at both item and order level → action should find it
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items = vec![create_test_item_with_rule("shared-rule")];
        snapshot.order_applied_rules = Some(vec![create_test_applied_rule("shared-rule")]);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: "shared-rule".to_string(),
            skipped: true,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();
        assert_eq!(events.len(), 1);
    }
}
