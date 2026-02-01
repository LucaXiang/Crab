//! Split order command handlers
//!
//! Four independent action handlers for split payments:
//! - **SplitByItems** (菜品分单): items provided, backend calculates amount
//! - **SplitByAmount** (金额分单): amount provided, no item tracking
//! - **StartAASplit** (AA 开始): lock headcount + pay first share
//! - **PayAASplit** (AA 后续支付): pay additional shares

use async_trait::async_trait;

use crate::orders::money::{calculate_unit_price, to_decimal, to_f64, MONEY_TOLERANCE};
use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use rust_decimal::Decimal;
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus, SplitItem};

// ============================================================================
// Shared validation
// ============================================================================

fn validate_active_order(
    snapshot: &shared::order::OrderSnapshot,
    order_id: &str,
) -> Result<(), OrderError> {
    match snapshot.status {
        OrderStatus::Active => Ok(()),
        OrderStatus::Completed => Err(OrderError::OrderAlreadyCompleted(order_id.to_string())),
        OrderStatus::Void => Err(OrderError::OrderAlreadyVoided(order_id.to_string())),
        _ => Err(OrderError::OrderNotFound(order_id.to_string())),
    }
}

/// Validate that a specific split mode is allowed given the current snapshot state.
fn validate_split_mode_allowed(
    snapshot: &shared::order::OrderSnapshot,
    mode: SplitMode,
) -> Result<(), OrderError> {
    // AA mode active → only AA payments allowed
    if snapshot.aa_total_shares.is_some() {
        if !matches!(mode, SplitMode::Aa) {
            return Err(OrderError::InvalidOperation(
                "AA split is active. Only AA share payments are allowed".to_string(),
            ));
        }
        return Ok(());
    }

    // Amount split active → block item split only, allow AA
    if snapshot.has_amount_split {
        if matches!(mode, SplitMode::Item) {
            return Err(OrderError::InvalidOperation(
                "Item-based split is disabled while amount-based split payments exist"
                    .to_string(),
            ));
        }
        return Ok(()); // Amount and AA both OK
    }

    // Item split active → does not block any mode
    // (item split only marks specific items as paid, remaining amount is still available)

    Ok(())
}

enum SplitMode {
    Item,
    Amount,
    Aa,
}

/// Validate items exist in order and have sufficient quantity.
/// Returns the calculated amount from items.
fn validate_items_and_calculate(
    snapshot: &shared::order::OrderSnapshot,
    items: &[SplitItem],
) -> Result<Decimal, OrderError> {
    let mut calculated_amount = Decimal::ZERO;
    for split_item in items {
        let order_item = snapshot
            .items
            .iter()
            .find(|i| i.instance_id == split_item.instance_id)
            .ok_or_else(|| OrderError::ItemNotFound(split_item.instance_id.clone()))?;

        let paid_qty = snapshot
            .paid_item_quantities
            .get(&split_item.instance_id)
            .copied()
            .unwrap_or(0);
        let available_qty = order_item.quantity - paid_qty;

        if split_item.quantity > available_qty {
            return Err(OrderError::InsufficientQuantity);
        }

        let unit_price = calculate_unit_price(order_item);
        calculated_amount += unit_price * Decimal::from(split_item.quantity);
    }
    Ok(calculated_amount)
}

// ============================================================================
// SplitByItems (菜品分单)
// ============================================================================

/// SplitByItems action — pays for specific items
#[derive(Debug, Clone)]
pub struct SplitByItemsAction {
    pub order_id: String,
    pub payment_method: String,
    pub items: Vec<SplitItem>,
    pub tendered: Option<f64>,
}

#[async_trait]
impl CommandHandler for SplitByItemsAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        let snapshot = ctx.load_snapshot(&self.order_id)?;
        validate_active_order(&snapshot, &self.order_id)?;
        validate_split_mode_allowed(&snapshot, SplitMode::Item)?;

        if self.items.is_empty() {
            return Err(OrderError::InvalidOperation(
                "Item-based split requires at least one item".to_string(),
            ));
        }

        let calculated_amount = validate_items_and_calculate(&snapshot, &self.items)?;
        let amount_f64 = to_f64(calculated_amount);
        if amount_f64 <= 0.0 {
            return Err(OrderError::InvalidAmount);
        }

        // Cannot overpay
        let remaining = to_decimal(snapshot.total) - to_decimal(snapshot.paid_amount);
        if calculated_amount > remaining + MONEY_TOLERANCE {
            return Err(OrderError::InvalidOperation(format!(
                "Split amount ({:.2}) exceeds remaining unpaid ({:.2})",
                amount_f64,
                to_f64(remaining)
            )));
        }

        let payment_id = uuid::Uuid::new_v4().to_string();
        let seq = ctx.next_sequence();
        if let Some(t) = self.tendered {
            if to_decimal(t) < to_decimal(amount_f64) - MONEY_TOLERANCE {
                return Err(OrderError::InvalidOperation(format!(
                    "Tendered {:.2} is less than required {:.2}",
                    t, amount_f64
                )));
            }
        }
        let change = self.tendered.map(|t| {
            let diff = to_decimal(t) - to_decimal(amount_f64);
            to_f64(diff.max(Decimal::ZERO))
        });

        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::ItemSplit,
            EventPayload::ItemSplit {
                payment_id,
                split_amount: amount_f64,
                payment_method: self.payment_method.clone(),
                items: self.items.clone(),
                tendered: self.tendered,
                change,
            },
        );

        Ok(vec![event])
    }
}

// ============================================================================
// SplitByAmount (金额分单)
// ============================================================================

/// SplitByAmount action — pays a fixed amount without item tracking
#[derive(Debug, Clone)]
pub struct SplitByAmountAction {
    pub order_id: String,
    pub split_amount: f64,
    pub payment_method: String,
    pub tendered: Option<f64>,
}

#[async_trait]
impl CommandHandler for SplitByAmountAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        let snapshot = ctx.load_snapshot(&self.order_id)?;
        validate_active_order(&snapshot, &self.order_id)?;
        validate_split_mode_allowed(&snapshot, SplitMode::Amount)?;

        if self.split_amount <= 0.0 {
            return Err(OrderError::InvalidAmount);
        }

        let remaining = to_decimal(snapshot.total) - to_decimal(snapshot.paid_amount);
        if to_decimal(self.split_amount) > remaining + MONEY_TOLERANCE {
            return Err(OrderError::InvalidOperation(format!(
                "Split amount ({:.2}) exceeds remaining unpaid ({:.2})",
                self.split_amount,
                to_f64(remaining)
            )));
        }

        let payment_id = uuid::Uuid::new_v4().to_string();
        let seq = ctx.next_sequence();
        if let Some(t) = self.tendered {
            if to_decimal(t) < to_decimal(self.split_amount) - MONEY_TOLERANCE {
                return Err(OrderError::InvalidOperation(format!(
                    "Tendered {:.2} is less than required {:.2}",
                    t, self.split_amount
                )));
            }
        }
        let change = self.tendered.map(|t| {
            let diff = to_decimal(t) - to_decimal(self.split_amount);
            to_f64(diff.max(Decimal::ZERO))
        });

        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::AmountSplit,
            EventPayload::AmountSplit {
                payment_id,
                split_amount: self.split_amount,
                payment_method: self.payment_method.clone(),
                tendered: self.tendered,
                change,
            },
        );

        Ok(vec![event])
    }
}

// ============================================================================
// StartAASplit (AA 开始 + 第一份支付)
// ============================================================================

/// StartAASplit action — locks headcount and pays the first share(s)
#[derive(Debug, Clone)]
pub struct StartAaSplitAction {
    pub order_id: String,
    pub total_shares: i32,
    pub shares: i32,
    pub payment_method: String,
    pub tendered: Option<f64>,
}

#[async_trait]
impl CommandHandler for StartAaSplitAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        let snapshot = ctx.load_snapshot(&self.order_id)?;
        validate_active_order(&snapshot, &self.order_id)?;
        validate_split_mode_allowed(&snapshot, SplitMode::Aa)?;

        // Must not already be in AA mode
        if snapshot.aa_total_shares.is_some() {
            return Err(OrderError::InvalidOperation(
                "AA split already started. Use PayAaSplit for subsequent payments".to_string(),
            ));
        }

        if self.total_shares < 2 {
            return Err(OrderError::InvalidOperation(
                "AA total shares must be at least 2".to_string(),
            ));
        }
        if self.shares < 1 {
            return Err(OrderError::InvalidAmount);
        }
        if self.shares > self.total_shares {
            return Err(OrderError::InvalidOperation(format!(
                "Shares ({}) exceeds total shares ({})",
                self.shares, self.total_shares
            )));
        }

        let remaining_unpaid = to_decimal(snapshot.total) - to_decimal(snapshot.paid_amount);
        let per_share = remaining_unpaid / Decimal::from(self.total_shares);
        let order_total_f64 = to_f64(remaining_unpaid);
        let per_share_f64 = to_f64(per_share);

        // Calculate first payment amount
        let amount = if self.shares == self.total_shares {
            remaining_unpaid
        } else {
            per_share * Decimal::from(self.shares)
        };
        let amount_f64 = to_f64(amount);

        if amount_f64 <= 0.0 {
            return Err(OrderError::InvalidAmount);
        }

        // Event 1: AASplitStarted
        let seq1 = ctx.next_sequence();
        let started_event = OrderEvent::new(
            seq1,
            self.order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::AaSplitStarted,
            EventPayload::AaSplitStarted {
                total_shares: self.total_shares,
                per_share_amount: per_share_f64,
                order_total: order_total_f64,
            },
        );

        // Event 2: AASplitPaid (first payment)
        let payment_id = uuid::Uuid::new_v4().to_string();
        let seq2 = ctx.next_sequence();
        if let Some(t) = self.tendered {
            if to_decimal(t) < to_decimal(amount_f64) - MONEY_TOLERANCE {
                return Err(OrderError::InvalidOperation(format!(
                    "Tendered {:.2} is less than required {:.2}",
                    t, amount_f64
                )));
            }
        }
        let change = self.tendered.map(|t| {
            let diff = to_decimal(t) - to_decimal(amount_f64);
            to_f64(diff.max(Decimal::ZERO))
        });
        let paid_event = OrderEvent::new(
            seq2,
            self.order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::AaSplitPaid,
            EventPayload::AaSplitPaid {
                payment_id,
                shares: self.shares,
                amount: amount_f64,
                payment_method: self.payment_method.clone(),
                progress_paid: self.shares,
                progress_total: self.total_shares,
                tendered: self.tendered,
                change,
            },
        );

        Ok(vec![started_event, paid_event])
    }
}

// ============================================================================
// PayAASplit (AA 后续支付)
// ============================================================================

/// PayAASplit action — pays additional shares in an existing AA split
#[derive(Debug, Clone)]
pub struct PayAaSplitAction {
    pub order_id: String,
    pub shares: i32,
    pub payment_method: String,
    pub tendered: Option<f64>,
}

#[async_trait]
impl CommandHandler for PayAaSplitAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        let snapshot = ctx.load_snapshot(&self.order_id)?;
        validate_active_order(&snapshot, &self.order_id)?;

        // Must already be in AA mode
        let total_shares = snapshot.aa_total_shares.ok_or_else(|| {
            OrderError::InvalidOperation(
                "AA split not started. Use StartAaSplit first".to_string(),
            )
        })?;

        if self.shares < 1 {
            return Err(OrderError::InvalidAmount);
        }

        let remaining_shares = total_shares - snapshot.aa_paid_shares;
        if self.shares > remaining_shares {
            return Err(OrderError::InvalidOperation(format!(
                "AA shares ({}) exceeds remaining shares ({})",
                self.shares, remaining_shares
            )));
        }

        let remaining_unpaid = to_decimal(snapshot.total) - to_decimal(snapshot.paid_amount);

        // Calculate amount: last payer gets exact remaining to avoid rounding residual
        let amount = if self.shares == remaining_shares {
            remaining_unpaid
        } else {
            let per_share = remaining_unpaid / Decimal::from(remaining_shares);
            per_share * Decimal::from(self.shares)
        };
        let amount_f64 = to_f64(amount);

        if amount_f64 <= 0.0 {
            return Err(OrderError::InvalidAmount);
        }

        let payment_id = uuid::Uuid::new_v4().to_string();
        let seq = ctx.next_sequence();
        if let Some(t) = self.tendered {
            if to_decimal(t) < to_decimal(amount_f64) - MONEY_TOLERANCE {
                return Err(OrderError::InvalidOperation(format!(
                    "Tendered {:.2} is less than required {:.2}",
                    t, amount_f64
                )));
            }
        }
        let change = self.tendered.map(|t| {
            let diff = to_decimal(t) - to_decimal(amount_f64);
            to_f64(diff.max(Decimal::ZERO))
        });

        let progress_paid = snapshot.aa_paid_shares + self.shares;

        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::AaSplitPaid,
            EventPayload::AaSplitPaid {
                payment_id,
                shares: self.shares,
                amount: amount_f64,
                payment_method: self.payment_method.clone(),
                progress_paid,
                progress_total: total_shares,
                tendered: self.tendered,
                change,
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

    fn create_active_order_with_items(order_id: &str) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;
        snapshot.table_id = Some("dining_table:t1".to_string());
        snapshot.table_name = Some("Table 1".to_string());

        let item1 = CartItemSnapshot {
            id: "product:1".to_string(),
            instance_id: "item-1".to_string(),
            name: "Coffee".to_string(),
            price: 10.0,
            original_price: None,
            quantity: 3,
            unpaid_quantity: 3,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            unit_price: None,
            line_total: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            tax: None,
            tax_rate: None,
        };
        let item2 = CartItemSnapshot {
            id: "product:2".to_string(),
            instance_id: "item-2".to_string(),
            name: "Tea".to_string(),
            price: 8.0,
            original_price: None,
            quantity: 2,
            unpaid_quantity: 2,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            unit_price: None,
            line_total: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            tax: None,
            tax_rate: None,
        };
        snapshot.items.push(item1);
        snapshot.items.push(item2);
        snapshot.subtotal = 46.0; // 3*10 + 2*8
        snapshot.total = 46.0;

        snapshot
    }

    // ========== SplitByItems tests ==========

    #[tokio::test]
    async fn test_split_by_items_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order_with_items("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = SplitByItemsAction {
            order_id: "order-1".to_string(),
            payment_method: "CASH".to_string(),
            items: vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 2,
                unit_price: 10.0,
            }],
            tendered: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, OrderEventType::ItemSplit);

        if let EventPayload::ItemSplit {
            split_amount,
            payment_method,
            items,
            ..
        } = &events[0].payload
        {
            assert_eq!(*split_amount, 20.0);
            assert_eq!(payment_method, "CASH");
            assert_eq!(items.len(), 1);
        } else {
            panic!("Expected ItemSplit payload");
        }
    }

    #[tokio::test]
    async fn test_split_by_items_empty_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order_with_items("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = SplitByItemsAction {
            order_id: "order-1".to_string(),
            payment_method: "CASH".to_string(),
            items: vec![],
            tendered: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    // ========== SplitByAmount tests ==========

    #[tokio::test]
    async fn test_split_by_amount_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order_with_items("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = SplitByAmountAction {
            order_id: "order-1".to_string(),
            split_amount: 20.0,
            payment_method: "CARD".to_string(),
            tendered: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, OrderEventType::AmountSplit);

        if let EventPayload::AmountSplit {
            split_amount,
            payment_method,
            ..
        } = &events[0].payload
        {
            assert_eq!(*split_amount, 20.0);
            assert_eq!(payment_method, "CARD");
        } else {
            panic!("Expected AmountSplit payload");
        }
    }

    #[tokio::test]
    async fn test_split_by_amount_zero_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order_with_items("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = SplitByAmountAction {
            order_id: "order-1".to_string(),
            split_amount: 0.0,
            payment_method: "CASH".to_string(),
            tendered: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidAmount)));
    }

    // ========== StartAASplit tests ==========

    #[tokio::test]
    async fn test_start_aa_split_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order_with_items("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = StartAaSplitAction {
            order_id: "order-1".to_string(),
            total_shares: 3,
            shares: 1,
            payment_method: "CASH".to_string(),
            tendered: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        // Should produce 2 events: AaSplitStarted + AaSplitPaid
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, OrderEventType::AaSplitStarted);
        assert_eq!(events[1].event_type, OrderEventType::AaSplitPaid);

        if let EventPayload::AaSplitStarted {
            total_shares,
            order_total,
            ..
        } = &events[0].payload
        {
            assert_eq!(*total_shares, 3);
            assert_eq!(*order_total, 46.0);
        } else {
            panic!("Expected AaSplitStarted payload");
        }

        if let EventPayload::AaSplitPaid {
            shares,
            progress_paid,
            progress_total,
            ..
        } = &events[1].payload
        {
            assert_eq!(*shares, 1);
            assert_eq!(*progress_paid, 1);
            assert_eq!(*progress_total, 3);
        } else {
            panic!("Expected AaSplitPaid payload");
        }
    }

    #[tokio::test]
    async fn test_start_aa_split_invalid_total_shares() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order_with_items("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = StartAaSplitAction {
            order_id: "order-1".to_string(),
            total_shares: 1, // Must be >= 2
            shares: 1,
            payment_method: "CASH".to_string(),
            tendered: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    // ========== PayAASplit tests ==========

    #[tokio::test]
    async fn test_pay_aa_split_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_order_with_items("order-1");
        // Simulate AA already started
        snapshot.aa_total_shares = Some(3);
        snapshot.aa_paid_shares = 1;
        snapshot.paid_amount = 15.33; // ~46/3
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = PayAaSplitAction {
            order_id: "order-1".to_string(),
            shares: 1,
            payment_method: "CARD".to_string(),
            tendered: None,
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, OrderEventType::AaSplitPaid);

        if let EventPayload::AaSplitPaid {
            shares,
            progress_paid,
            progress_total,
            ..
        } = &events[0].payload
        {
            assert_eq!(*shares, 1);
            assert_eq!(*progress_paid, 2);
            assert_eq!(*progress_total, 3);
        } else {
            panic!("Expected AaSplitPaid payload");
        }
    }

    #[tokio::test]
    async fn test_pay_aa_split_not_started_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order_with_items("order-1");
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = PayAaSplitAction {
            order_id: "order-1".to_string(),
            shares: 1,
            payment_method: "CASH".to_string(),
            tendered: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }

    // ========== Mutual exclusion tests ==========

    #[tokio::test]
    async fn test_item_split_then_amount_split_allowed() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_order_with_items("order-1");
        // Simulate item split already happened: item-1 has 2 units paid
        snapshot
            .paid_item_quantities
            .insert("item-1".to_string(), 2);
        snapshot.paid_amount = 20.0;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Amount split should be allowed after item split
        let action = SplitByAmountAction {
            order_id: "order-1".to_string(),
            split_amount: 10.0,
            payment_method: "CASH".to_string(),
            tendered: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(result.is_ok(), "Amount split should be allowed after item split");
    }

    #[tokio::test]
    async fn test_item_split_then_aa_split_allowed() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_order_with_items("order-1");
        // Simulate item split already happened
        snapshot
            .paid_item_quantities
            .insert("item-1".to_string(), 1);
        snapshot.paid_amount = 10.0;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // AA split should be allowed after item split
        let action = StartAaSplitAction {
            order_id: "order-1".to_string(),
            total_shares: 3,
            shares: 1,
            payment_method: "CASH".to_string(),
            tendered: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(result.is_ok(), "AA split should be allowed after item split");
    }

    #[tokio::test]
    async fn test_amount_split_then_item_split_blocked() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_order_with_items("order-1");
        snapshot.has_amount_split = true;
        snapshot.paid_amount = 10.0;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Item split should be blocked after amount split
        let action = SplitByItemsAction {
            order_id: "order-1".to_string(),
            payment_method: "CASH".to_string(),
            items: vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 1,
                unit_price: 10.0,
            }],
            tendered: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(
            matches!(result, Err(OrderError::InvalidOperation(_))),
            "Item split should be blocked after amount split"
        );
    }

    #[tokio::test]
    async fn test_amount_split_then_aa_split_allowed() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_order_with_items("order-1");
        snapshot.has_amount_split = true;
        snapshot.paid_amount = 10.0;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // AA split should be allowed after amount split
        let action = StartAaSplitAction {
            order_id: "order-1".to_string(),
            total_shares: 3,
            shares: 1,
            payment_method: "CASH".to_string(),
            tendered: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(result.is_ok(), "AA split should be allowed after amount split");
    }

    #[tokio::test]
    async fn test_aa_active_blocks_item_split() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_order_with_items("order-1");
        snapshot.aa_total_shares = Some(3);
        snapshot.aa_paid_shares = 1;
        snapshot.paid_amount = 15.33;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = SplitByItemsAction {
            order_id: "order-1".to_string(),
            payment_method: "CASH".to_string(),
            items: vec![SplitItem {
                instance_id: "item-1".to_string(),
                name: "Coffee".to_string(),
                quantity: 1,
                unit_price: 10.0,
            }],
            tendered: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(
            matches!(result, Err(OrderError::InvalidOperation(_))),
            "Item split should be blocked while AA is active"
        );
    }

    #[tokio::test]
    async fn test_aa_active_blocks_amount_split() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_order_with_items("order-1");
        snapshot.aa_total_shares = Some(3);
        snapshot.aa_paid_shares = 1;
        snapshot.paid_amount = 15.33;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = SplitByAmountAction {
            order_id: "order-1".to_string(),
            split_amount: 10.0,
            payment_method: "CASH".to_string(),
            tendered: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(
            matches!(result, Err(OrderError::InvalidOperation(_))),
            "Amount split should be blocked while AA is active"
        );
    }

    // ========== Remaining AA tests ==========

    #[tokio::test]
    async fn test_pay_aa_split_exceeds_remaining_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_order_with_items("order-1");
        snapshot.aa_total_shares = Some(3);
        snapshot.aa_paid_shares = 2; // Only 1 remaining
        snapshot.paid_amount = 30.67;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = PayAaSplitAction {
            order_id: "order-1".to_string(),
            shares: 2, // Only 1 available
            payment_method: "CASH".to_string(),
            tendered: None,
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;
        assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
    }
}
