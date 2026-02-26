//! ToggleRuleSkip command handler

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::types::CommandErrorCode;
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// ToggleRuleSkip action
#[derive(Debug, Clone)]
pub struct ToggleRuleSkipAction {
    pub order_id: String,
    pub rule_id: i64,
    pub skipped: bool,
}

impl CommandHandler for ToggleRuleSkipAction {
    fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Load snapshot
        let snapshot = ctx.load_snapshot(&self.order_id)?;

        // 2. Validate status
        if !matches!(snapshot.status, OrderStatus::Active) {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::OrderNotActive,
                "Cannot toggle rule on non-active order".to_string(),
            ));
        }

        // 2b. Block during active split payments (AA or amount split)
        // Changing rules would alter the total, making existing per-share
        // or per-split amounts inconsistent.
        if snapshot.aa_total_shares.is_some() {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::AaSplitActive,
                "Cannot toggle rule during AA split".to_string(),
            ));
        }
        if snapshot.has_amount_split {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::AmountSplitActive,
                "Cannot toggle rule during amount split".to_string(),
            ));
        }

        // 3. Find rule in the order and get its name
        let rule_name = snapshot
            .items
            .iter()
            .flat_map(|item| item.applied_rules.iter())
            .find(|r| r.rule_id == self.rule_id)
            .or_else(|| {
                snapshot
                    .order_applied_rules
                    .iter()
                    .find(|r| r.rule_id == self.rule_id)
            })
            .map(|r| r.name.clone());

        let Some(rule_name) = rule_name else {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::RuleNotFoundInOrder,
                format!("Rule {} not found in order", self.rule_id),
            ));
        };

        // 4. Generate event (actual toggle and recalculation will be done by applier)
        let seq = ctx.next_sequence();
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id,
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::RuleSkipToggled,
            EventPayload::RuleSkipToggled {
                rule_id: self.rule_id,
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
            operator_id: 1,
            operator_name: "Test User".to_string(),
            timestamp: 1234567890,
        }
    }

    fn create_test_applied_rule(rule_id: i64) -> AppliedRule {
        AppliedRule {
            rule_id,
            name: "test_rule".to_string(),
            receipt_name: Some("TEST".to_string()),

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

    fn create_test_item_with_rule(rule_id: i64) -> CartItemSnapshot {
        CartItemSnapshot {
            id: 1,
            instance_id: "item-1".to_string(),
            name: "Test Product".to_string(),
            price: 10.0,
            original_price: 0.0,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: 0.0,
            rule_surcharge_amount: 0.0,
            applied_rules: vec![create_test_applied_rule(rule_id)],
            applied_mg_rules: vec![],
            mg_discount_amount: 0.0,
            unit_price: 0.0,
            line_total: 0.0,
            tax: 0.0,
            tax_rate: 0,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_id: None,
            category_name: None,
            is_comped: false,
        }
    }

    #[test]
    fn test_toggle_rule_skip_item_rule_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create an active order with item that has applied rule
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items = vec![create_test_item_with_rule(1)];
        snapshot.subtotal = 10.0;
        snapshot.total = 10.0;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: 1,
            skipped: true,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.order_id, "order-1");
        assert_eq!(event.event_type, OrderEventType::RuleSkipToggled);

        if let EventPayload::RuleSkipToggled {
            rule_id, skipped, ..
        } = &event.payload
        {
            assert_eq!(*rule_id, 1);
            assert!(*skipped);
        } else {
            panic!("Expected RuleSkipToggled payload");
        }
    }

    #[test]
    fn test_toggle_rule_skip_order_rule_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create an active order with order-level applied rule
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.order_applied_rules = vec![create_test_applied_rule(100)];
        snapshot.subtotal = 100.0;
        snapshot.discount = 10.0;
        snapshot.total = 90.0;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: 100,
            skipped: true,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.event_type, OrderEventType::RuleSkipToggled);

        if let EventPayload::RuleSkipToggled {
            rule_id, skipped, ..
        } = &event.payload
        {
            assert_eq!(*rule_id, 100);
            assert!(*skipped);
        } else {
            panic!("Expected RuleSkipToggled payload");
        }
    }

    #[test]
    fn test_toggle_rule_skip_rule_not_found() {
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
            rule_id: 99999,
            skipped: true,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata);

        assert!(matches!(result, Err(OrderError::InvalidOperation(..))));
        if let Err(OrderError::InvalidOperation(_, msg)) = result {
            assert!(msg.contains("not found in order"));
        }
    }

    #[test]
    fn test_toggle_rule_skip_non_active_order() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create a completed order
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        snapshot.items = vec![create_test_item_with_rule(1)];
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: 1,
            skipped: true,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata);

        assert!(matches!(result, Err(OrderError::InvalidOperation(..))));
        if let Err(OrderError::InvalidOperation(_, msg)) = result {
            assert!(msg.contains("non-active order"));
        }
    }

    #[test]
    fn test_toggle_rule_skip_order_not_found() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ToggleRuleSkipAction {
            order_id: "nonexistent".to_string(),
            rule_id: 1,
            skipped: true,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata);

        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    #[test]
    fn test_toggle_rule_skip_unskip() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create an active order with item that has skipped rule
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        let mut rule = create_test_applied_rule(1);
        rule.skipped = true;
        snapshot.items = vec![CartItemSnapshot {
            id: 1,
            instance_id: "item-1".to_string(),
            name: "Test Product".to_string(),
            price: 10.0,
            original_price: 0.0,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: 0.0,
            rule_surcharge_amount: 0.0,
            applied_rules: vec![rule],
            applied_mg_rules: vec![],
            mg_discount_amount: 0.0,
            unit_price: 0.0,
            line_total: 0.0,
            tax: 0.0,
            tax_rate: 0,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_id: None,
            category_name: None,
            is_comped: false,
        }];
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Unskip the rule
        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: 1,
            skipped: false,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::RuleSkipToggled { skipped, .. } = &events[0].payload {
            assert!(!*skipped);
        } else {
            panic!("Expected RuleSkipToggled payload");
        }
    }

    #[test]
    fn test_toggle_rule_skip_event_metadata() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items = vec![create_test_item_with_rule(1)];
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: 1,
            skipped: true,
        };

        let metadata = CommandMetadata {
            command_id: "cmd-toggle-1".to_string(),
            operator_id: 100,
            operator_name: "Manager".to_string(),
            timestamp: 9999999999,
        };

        let events = action.execute(&mut ctx, &metadata).unwrap();
        let event = &events[0];

        assert_eq!(event.command_id, "cmd-toggle-1");
        assert_eq!(event.operator_id, 100);
        assert_eq!(event.operator_name, "Manager");
        assert_eq!(event.client_timestamp, Some(9999999999));
    }

    #[test]
    fn test_toggle_rule_skip_already_skipped_rule() {
        // Toggling skip on a rule that's already skipped should still succeed
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        let mut rule = create_test_applied_rule(1);
        rule.skipped = true; // already skipped
        snapshot.items = vec![CartItemSnapshot {
            id: 1,
            instance_id: "item-1".to_string(),
            name: "Test Product".to_string(),
            price: 10.0,
            original_price: 0.0,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: 0.0,
            rule_surcharge_amount: 0.0,
            applied_rules: vec![rule],
            applied_mg_rules: vec![],
            mg_discount_amount: 0.0,
            unit_price: 0.0,
            line_total: 0.0,
            tax: 0.0,
            tax_rate: 0,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_id: None,
            category_name: None,
            is_comped: false,
        }];
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Skip again (idempotent-ish)
        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: 1,
            skipped: true,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::RuleSkipToggled { skipped, .. } = &events[0].payload {
            assert!(*skipped);
        } else {
            panic!("Expected RuleSkipToggled payload");
        }
    }

    #[test]
    fn test_toggle_rule_skip_multiple_rules_on_item() {
        // Item has multiple rules, toggle one specific rule
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items = vec![CartItemSnapshot {
            id: 1,
            instance_id: "item-1".to_string(),
            name: "Test Product".to_string(),
            price: 10.0,
            original_price: 0.0,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: 0.0,
            rule_surcharge_amount: 0.0,
            applied_rules: vec![create_test_applied_rule(1), create_test_applied_rule(2)],
            applied_mg_rules: vec![],
            mg_discount_amount: 0.0,
            unit_price: 0.0,
            line_total: 0.0,
            tax: 0.0,
            tax_rate: 0,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_id: None,
            category_name: None,
            is_comped: false,
        }];
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Toggle only rule 2
        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: 2,
            skipped: true,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).unwrap();
        assert_eq!(events.len(), 1);

        if let EventPayload::RuleSkipToggled {
            rule_id, skipped, ..
        } = &events[0].payload
        {
            assert_eq!(*rule_id, 2);
            assert!(*skipped);
        } else {
            panic!("Expected RuleSkipToggled payload");
        }
    }

    #[test]
    fn test_toggle_rule_skip_rule_on_both_levels() {
        // Same rule_id exists at both item and order level → action should find it
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items = vec![create_test_item_with_rule(50)];
        snapshot.order_applied_rules = vec![create_test_applied_rule(50)];
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: 50,
            skipped: true,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_toggle_rule_skip_blocked_during_aa_split() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items = vec![create_test_item_with_rule(1)];
        snapshot.aa_total_shares = Some(3);
        snapshot.aa_paid_shares = 1;
        snapshot.paid_amount = 10.0;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: 1,
            skipped: true,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata);

        assert!(matches!(result, Err(OrderError::InvalidOperation(..))));
        if let Err(OrderError::InvalidOperation(_, msg)) = result {
            assert!(msg.contains("AA split"));
        }
    }

    #[test]
    fn test_toggle_rule_skip_blocked_during_amount_split() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items = vec![create_test_item_with_rule(1)];
        snapshot.has_amount_split = true;
        snapshot.paid_amount = 20.0;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ToggleRuleSkipAction {
            order_id: "order-1".to_string(),
            rule_id: 1,
            skipped: true,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata);

        assert!(matches!(result, Err(OrderError::InvalidOperation(..))));
        if let Err(OrderError::InvalidOperation(_, msg)) = result {
            assert!(msg.contains("amount split"));
        }
    }
}
