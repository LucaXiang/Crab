//! ApplyOrderDiscount + ApplyOrderSurcharge command handlers
//!
//! 订单级手动折扣和附加费操作。

use async_trait::async_trait;

use crate::orders::money::{recalculate_totals, to_decimal, to_f64};
use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use rust_decimal::prelude::*;
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// ApplyOrderDiscount action — 应用/清除订单级手动折扣
#[derive(Debug, Clone)]
pub struct ApplyOrderDiscountAction {
    pub order_id: String,
    pub discount_percent: Option<f64>,
    pub discount_fixed: Option<f64>,
    pub reason: Option<String>,
    pub authorizer_id: Option<String>,
    pub authorizer_name: Option<String>,
}

#[async_trait]
impl CommandHandler for ApplyOrderDiscountAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Load snapshot
        let mut snapshot = ctx.load_snapshot(&self.order_id)?;

        // 2. Validate: order must be Active
        if !matches!(snapshot.status, OrderStatus::Active) {
            return Err(OrderError::InvalidOperation(
                "Cannot apply discount on non-active order".to_string(),
            ));
        }

        // 3. Validate: percent 和 fixed 互斥
        if self.discount_percent.is_some() && self.discount_fixed.is_some() {
            return Err(OrderError::InvalidOperation(
                "discount_percent and discount_fixed are mutually exclusive".to_string(),
            ));
        }

        // 4. Validate: percent 范围 0-100
        if let Some(pct) = self.discount_percent
            && (!pct.is_finite() || !(0.0..=100.0).contains(&pct)) {
                return Err(OrderError::InvalidOperation(format!(
                    "discount_percent must be between 0 and 100, got {}",
                    pct
                )));
            }

        // 5. Validate: fixed 必须为正
        if let Some(fixed) = self.discount_fixed
            && (!fixed.is_finite() || fixed <= 0.0) {
                return Err(OrderError::InvalidOperation(format!(
                    "discount_fixed must be positive, got {}",
                    fixed
                )));
            }

        // 6. Record previous values
        let previous_discount_percent = snapshot.order_manual_discount_percent;
        let previous_discount_fixed = snapshot.order_manual_discount_fixed;

        // 7. Apply new discount to snapshot (for recalculate_totals)
        snapshot.order_manual_discount_percent = self.discount_percent;
        snapshot.order_manual_discount_fixed = self.discount_fixed;

        // 8. Recalculate totals
        recalculate_totals(&mut snapshot);

        // 9. Generate event
        let seq = ctx.next_sequence();
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::OrderDiscountApplied,
            EventPayload::OrderDiscountApplied {
                discount_percent: self.discount_percent,
                discount_fixed: self.discount_fixed,
                previous_discount_percent,
                previous_discount_fixed,
                reason: self.reason.clone(),
                authorizer_id: self.authorizer_id.clone(),
                authorizer_name: self.authorizer_name.clone(),
                subtotal: snapshot.subtotal,
                discount: snapshot.discount,
                total: snapshot.total,
            },
        );

        Ok(vec![event])
    }
}

/// ApplyOrderSurcharge action — 应用/清除订单级手动附加费
#[derive(Debug, Clone)]
pub struct ApplyOrderSurchargeAction {
    pub order_id: String,
    pub surcharge_percent: Option<f64>,
    pub surcharge_amount: Option<f64>,
    pub reason: Option<String>,
    pub authorizer_id: Option<String>,
    pub authorizer_name: Option<String>,
}

#[async_trait]
impl CommandHandler for ApplyOrderSurchargeAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Load snapshot
        let mut snapshot = ctx.load_snapshot(&self.order_id)?;

        // 2. Validate: order must be Active
        if !matches!(snapshot.status, OrderStatus::Active) {
            return Err(OrderError::InvalidOperation(
                "Cannot apply surcharge on non-active order".to_string(),
            ));
        }

        // 3. Validate: percent 和 fixed 互斥
        if self.surcharge_percent.is_some() && self.surcharge_amount.is_some() {
            return Err(OrderError::InvalidOperation(
                "surcharge_percent and surcharge_amount are mutually exclusive".to_string(),
            ));
        }

        // 4. Validate: percent 范围 0-100
        if let Some(pct) = self.surcharge_percent
            && (!pct.is_finite() || pct <= 0.0 || pct > 100.0) {
                return Err(OrderError::InvalidOperation(format!(
                    "surcharge_percent must be between 0 and 100, got {}",
                    pct
                )));
            }

        // 5. Validate: surcharge_amount 必须为正（如果有值）
        if let Some(amount) = self.surcharge_amount
            && (!amount.is_finite() || amount <= 0.0) {
                return Err(OrderError::InvalidOperation(format!(
                    "surcharge_amount must be positive, got {}",
                    amount
                )));
            }

        // 6. Record previous values
        let previous_surcharge_percent = snapshot.order_manual_surcharge_percent;
        let previous_surcharge_amount = snapshot.order_manual_surcharge_fixed;

        // 7. Apply new surcharge to snapshot (for recalculate_totals)
        snapshot.order_manual_surcharge_percent = self.surcharge_percent;
        snapshot.order_manual_surcharge_fixed = self.surcharge_amount;

        // 8. Recalculate totals
        recalculate_totals(&mut snapshot);

        // 9. Generate event
        let seq = ctx.next_sequence();
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::OrderSurchargeApplied,
            EventPayload::OrderSurchargeApplied {
                surcharge_percent: self.surcharge_percent,
                surcharge_amount: self.surcharge_amount,
                previous_surcharge_percent,
                previous_surcharge_amount,
                reason: self.reason.clone(),
                authorizer_id: self.authorizer_id.clone(),
                authorizer_name: self.authorizer_name.clone(),
                subtotal: snapshot.subtotal,
                surcharge: to_f64(
                    snapshot.order_rule_surcharge_amount.map(to_decimal).unwrap_or(Decimal::ZERO)
                        + snapshot.order_manual_surcharge_percent
                            .map(|p| to_decimal(snapshot.subtotal) * to_decimal(p) / Decimal::ONE_HUNDRED)
                            .unwrap_or(Decimal::ZERO)
                        + snapshot.order_manual_surcharge_fixed.map(to_decimal).unwrap_or(Decimal::ZERO),
                ),
                total: snapshot.total,
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
    use shared::order::{CartItemSnapshot, OrderSnapshot};

    fn create_test_metadata() -> CommandMetadata {
        CommandMetadata {
            command_id: "cmd-1".to_string(),
            operator_id: "user-1".to_string(),
            operator_name: "Test User".to_string(),
            timestamp: 1234567890,
        }
    }

    fn create_test_item(price: f64, quantity: i32) -> CartItemSnapshot {
        CartItemSnapshot {
            id: "product:p1".to_string(),
            instance_id: "item-1".to_string(),
            name: "Test Product".to_string(),
            price,
            original_price: Some(price),
            quantity,
            unpaid_quantity: quantity,
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

    fn setup_active_order(storage: &OrderStorage, order_id: &str, items: Vec<CartItemSnapshot>) -> OrderSnapshot {
        let txn = storage.begin_write().unwrap();
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items = items;
        recalculate_totals(&mut snapshot);
        storage.store_snapshot(&txn, &snapshot).unwrap();
        txn.commit().unwrap();
        snapshot
    }

    // ==========================================================
    // ApplyOrderDiscount tests
    // ==========================================================

    #[tokio::test]
    async fn test_apply_percentage_discount() {
        let storage = OrderStorage::open_in_memory().unwrap();
        setup_active_order(&storage, "order-1", vec![create_test_item(100.0, 1)]);

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ApplyOrderDiscountAction {
            order_id: "order-1".to_string(),
            discount_percent: Some(10.0),
            discount_fixed: None,
            reason: Some("VIP customer".to_string()),
            authorizer_id: Some("mgr-1".to_string()),
            authorizer_name: Some("Manager".to_string()),
        };

        let events = action.execute(&mut ctx, &create_test_metadata()).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, OrderEventType::OrderDiscountApplied);

        if let EventPayload::OrderDiscountApplied {
            discount_percent,
            discount_fixed,
            previous_discount_percent,
            previous_discount_fixed,
            subtotal,
            discount,
            total,
            ..
        } = &events[0].payload
        {
            assert_eq!(*discount_percent, Some(10.0));
            assert_eq!(*discount_fixed, None);
            assert_eq!(*previous_discount_percent, None);
            assert_eq!(*previous_discount_fixed, None);
            assert_eq!(*subtotal, 100.0);
            assert_eq!(*discount, 10.0);
            assert_eq!(*total, 90.0);
        } else {
            panic!("Expected OrderDiscountApplied payload");
        }
    }

    #[tokio::test]
    async fn test_apply_fixed_discount() {
        let storage = OrderStorage::open_in_memory().unwrap();
        setup_active_order(&storage, "order-1", vec![create_test_item(100.0, 1)]);

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ApplyOrderDiscountAction {
            order_id: "order-1".to_string(),
            discount_percent: None,
            discount_fixed: Some(25.0),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let events = action.execute(&mut ctx, &create_test_metadata()).await.unwrap();

        if let EventPayload::OrderDiscountApplied {
            discount_fixed,
            subtotal,
            discount,
            total,
            ..
        } = &events[0].payload
        {
            assert_eq!(*discount_fixed, Some(25.0));
            assert_eq!(*subtotal, 100.0);
            assert_eq!(*discount, 25.0);
            assert_eq!(*total, 75.0);
        } else {
            panic!("Expected OrderDiscountApplied payload");
        }
    }

    #[tokio::test]
    async fn test_clear_discount() {
        let storage = OrderStorage::open_in_memory().unwrap();
        // 先设置一个已有折扣的订单
        let txn = storage.begin_write().unwrap();
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items = vec![create_test_item(100.0, 1)];
        snapshot.order_manual_discount_percent = Some(10.0);
        recalculate_totals(&mut snapshot);
        storage.store_snapshot(&txn, &snapshot).unwrap();
        txn.commit().unwrap();

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // 清除折扣：两个都为 None
        let action = ApplyOrderDiscountAction {
            order_id: "order-1".to_string(),
            discount_percent: None,
            discount_fixed: None,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let events = action.execute(&mut ctx, &create_test_metadata()).await.unwrap();

        if let EventPayload::OrderDiscountApplied {
            discount_percent,
            discount_fixed,
            previous_discount_percent,
            subtotal,
            discount,
            total,
            ..
        } = &events[0].payload
        {
            assert_eq!(*discount_percent, None);
            assert_eq!(*discount_fixed, None);
            assert_eq!(*previous_discount_percent, Some(10.0));
            assert_eq!(*subtotal, 100.0);
            assert_eq!(*discount, 0.0);
            assert_eq!(*total, 100.0);
        } else {
            panic!("Expected OrderDiscountApplied payload");
        }
    }

    #[tokio::test]
    async fn test_discount_percent_and_fixed_mutual_exclusion() {
        let storage = OrderStorage::open_in_memory().unwrap();
        setup_active_order(&storage, "order-1", vec![create_test_item(100.0, 1)]);

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ApplyOrderDiscountAction {
            order_id: "order-1".to_string(),
            discount_percent: Some(10.0),
            discount_fixed: Some(20.0), // 两者同时设置
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let result = action.execute(&mut ctx, &create_test_metadata()).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
        if let Err(OrderError::InvalidOperation(msg)) = result {
            assert!(msg.contains("mutually exclusive"));
        }
    }

    #[tokio::test]
    async fn test_discount_percent_negative_rejected() {
        let storage = OrderStorage::open_in_memory().unwrap();
        setup_active_order(&storage, "order-1", vec![create_test_item(100.0, 1)]);

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);
        let action = ApplyOrderDiscountAction {
            order_id: "order-1".to_string(),
            discount_percent: Some(-5.0),
            discount_fixed: None,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };
        let result = action.execute(&mut ctx, &create_test_metadata()).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_discount_percent_over_100_rejected() {
        let storage = OrderStorage::open_in_memory().unwrap();
        setup_active_order(&storage, "order-1", vec![create_test_item(100.0, 1)]);

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);
        let action = ApplyOrderDiscountAction {
            order_id: "order-1".to_string(),
            discount_percent: Some(101.0),
            discount_fixed: None,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };
        let result = action.execute(&mut ctx, &create_test_metadata()).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_discount_fixed_must_be_positive() {
        let storage = OrderStorage::open_in_memory().unwrap();
        setup_active_order(&storage, "order-1", vec![create_test_item(100.0, 1)]);

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ApplyOrderDiscountAction {
            order_id: "order-1".to_string(),
            discount_percent: None,
            discount_fixed: Some(-10.0),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let result = action.execute(&mut ctx, &create_test_metadata()).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_discount_on_non_active_order() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Completed;
        snapshot.items = vec![create_test_item(100.0, 1)];
        storage.store_snapshot(&txn, &snapshot).unwrap();
        txn.commit().unwrap();

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ApplyOrderDiscountAction {
            order_id: "order-1".to_string(),
            discount_percent: Some(10.0),
            discount_fixed: None,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let result = action.execute(&mut ctx, &create_test_metadata()).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_discount_order_not_found() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ApplyOrderDiscountAction {
            order_id: "nonexistent".to_string(),
            discount_percent: Some(10.0),
            discount_fixed: None,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let result = action.execute(&mut ctx, &create_test_metadata()).await;
        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    // ==========================================================
    // ApplyOrderSurcharge tests
    // ==========================================================

    #[tokio::test]
    async fn test_apply_surcharge() {
        let storage = OrderStorage::open_in_memory().unwrap();
        setup_active_order(&storage, "order-1", vec![create_test_item(100.0, 1)]);

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ApplyOrderSurchargeAction {
            order_id: "order-1".to_string(),
            surcharge_percent: None,
            surcharge_amount: Some(15.0),
            reason: Some("Large party".to_string()),
            authorizer_id: Some("mgr-1".to_string()),
            authorizer_name: Some("Manager".to_string()),
        };

        let events = action.execute(&mut ctx, &create_test_metadata()).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, OrderEventType::OrderSurchargeApplied);

        if let EventPayload::OrderSurchargeApplied {
            surcharge_amount,
            previous_surcharge_amount,
            subtotal,
            surcharge,
            total,
            ..
        } = &events[0].payload
        {
            assert_eq!(*surcharge_amount, Some(15.0));
            assert_eq!(*previous_surcharge_amount, None);
            assert_eq!(*subtotal, 100.0);
            assert_eq!(*surcharge, 15.0);
            assert_eq!(*total, 115.0);
        } else {
            panic!("Expected OrderSurchargeApplied payload");
        }
    }

    #[tokio::test]
    async fn test_clear_surcharge() {
        let storage = OrderStorage::open_in_memory().unwrap();
        // 先设置一个已有附加费的订单
        let txn = storage.begin_write().unwrap();
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items = vec![create_test_item(100.0, 1)];
        snapshot.order_manual_surcharge_fixed = Some(15.0);
        recalculate_totals(&mut snapshot);
        storage.store_snapshot(&txn, &snapshot).unwrap();
        txn.commit().unwrap();

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ApplyOrderSurchargeAction {
            order_id: "order-1".to_string(),
            surcharge_percent: None,
            surcharge_amount: None, // 清除
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let events = action.execute(&mut ctx, &create_test_metadata()).await.unwrap();

        if let EventPayload::OrderSurchargeApplied {
            surcharge_amount,
            previous_surcharge_amount,
            subtotal,
            surcharge,
            total,
            ..
        } = &events[0].payload
        {
            assert_eq!(*surcharge_amount, None);
            assert_eq!(*previous_surcharge_amount, Some(15.0));
            assert_eq!(*subtotal, 100.0);
            assert_eq!(*surcharge, 0.0);
            assert_eq!(*total, 100.0);
        } else {
            panic!("Expected OrderSurchargeApplied payload");
        }
    }

    #[tokio::test]
    async fn test_surcharge_must_be_positive() {
        let storage = OrderStorage::open_in_memory().unwrap();
        setup_active_order(&storage, "order-1", vec![create_test_item(100.0, 1)]);

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ApplyOrderSurchargeAction {
            order_id: "order-1".to_string(),
            surcharge_percent: None,
            surcharge_amount: Some(-5.0),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let result = action.execute(&mut ctx, &create_test_metadata()).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_surcharge_on_non_active_order() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Void;
        snapshot.items = vec![create_test_item(100.0, 1)];
        storage.store_snapshot(&txn, &snapshot).unwrap();
        txn.commit().unwrap();

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ApplyOrderSurchargeAction {
            order_id: "order-1".to_string(),
            surcharge_percent: None,
            surcharge_amount: Some(10.0),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let result = action.execute(&mut ctx, &create_test_metadata()).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_surcharge_order_not_found() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ApplyOrderSurchargeAction {
            order_id: "nonexistent".to_string(),
            surcharge_percent: None,
            surcharge_amount: Some(10.0),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let result = action.execute(&mut ctx, &create_test_metadata()).await;
        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }

    // ==========================================================
    // Discount + Surcharge coexistence tests
    // ==========================================================

    #[tokio::test]
    async fn test_discount_and_surcharge_coexistence() {
        let storage = OrderStorage::open_in_memory().unwrap();
        // 先设置订单有附加费
        let txn = storage.begin_write().unwrap();
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.items = vec![create_test_item(100.0, 2)]; // 200 subtotal
        snapshot.order_manual_surcharge_fixed = Some(20.0);
        recalculate_totals(&mut snapshot);
        storage.store_snapshot(&txn, &snapshot).unwrap();
        txn.commit().unwrap();

        // 再加折扣
        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ApplyOrderDiscountAction {
            order_id: "order-1".to_string(),
            discount_percent: Some(10.0), // 10% of 200 = 20 discount
            discount_fixed: None,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let events = action.execute(&mut ctx, &create_test_metadata()).await.unwrap();

        if let EventPayload::OrderDiscountApplied {
            subtotal,
            discount,
            total,
            ..
        } = &events[0].payload
        {
            // subtotal = 200 (items), discount = 20 (10% of 200), surcharge = 20
            // total = 200 - 20 + 20 = 200
            assert_eq!(*subtotal, 200.0);
            assert_eq!(*discount, 20.0);
            assert_eq!(*total, 200.0);
        } else {
            panic!("Expected OrderDiscountApplied payload");
        }
    }

    #[tokio::test]
    async fn test_discount_nan_rejected() {
        let storage = OrderStorage::open_in_memory().unwrap();
        setup_active_order(&storage, "order-1", vec![create_test_item(100.0, 1)]);

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ApplyOrderDiscountAction {
            order_id: "order-1".to_string(),
            discount_percent: Some(f64::NAN),
            discount_fixed: None,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let result = action.execute(&mut ctx, &create_test_metadata()).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_surcharge_nan_rejected() {
        let storage = OrderStorage::open_in_memory().unwrap();
        setup_active_order(&storage, "order-1", vec![create_test_item(100.0, 1)]);

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ApplyOrderSurchargeAction {
            order_id: "order-1".to_string(),
            surcharge_percent: None,
            surcharge_amount: Some(f64::NAN),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let result = action.execute(&mut ctx, &create_test_metadata()).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    #[tokio::test]
    async fn test_discount_event_metadata() {
        let storage = OrderStorage::open_in_memory().unwrap();
        setup_active_order(&storage, "order-1", vec![create_test_item(100.0, 1)]);

        let txn = storage.begin_write().unwrap();
        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = ApplyOrderDiscountAction {
            order_id: "order-1".to_string(),
            discount_percent: Some(5.0),
            discount_fixed: None,
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let metadata = CommandMetadata {
            command_id: "cmd-discount-1".to_string(),
            operator_id: "manager-1".to_string(),
            operator_name: "Manager".to_string(),
            timestamp: 9999999999,
        };

        let events = action.execute(&mut ctx, &metadata).await.unwrap();
        let event = &events[0];

        assert_eq!(event.command_id, "cmd-discount-1");
        assert_eq!(event.operator_id, "manager-1");
        assert_eq!(event.operator_name, "Manager");
        assert_eq!(event.client_timestamp, Some(9999999999));
        assert_eq!(event.order_id, "order-1");
    }
}
