//! AA Split handlers (均摊模式)
//!
//! - **StartAaSplit**: lock headcount + pay first share(s)
//! - **PayAaSplit**: pay additional shares in an existing AA split

use async_trait::async_trait;

use crate::orders::money::{to_decimal, to_f64};
use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use rust_decimal::Decimal;
use shared::order::types::CommandErrorCode;
use shared::order::{EventPayload, OrderEvent, OrderEventType};

use super::{validate_active_order, validate_split_mode_allowed, validate_tendered_and_change, SplitMode};

// ============================================================================
// StartAASplit (AA 开始 + 第一份支付)
// ============================================================================

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
                CommandErrorCode::AaSplitAlreadyStarted,
                "AA split already started. Use PayAaSplit for subsequent payments".to_string(),
            ));
        }

        if self.total_shares < 2 {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::InvalidShares,
                "AA total shares must be at least 2".to_string(),
            ));
        }
        if self.shares < 1 {
            return Err(OrderError::InvalidAmount);
        }
        if self.shares > self.total_shares {
            return Err(OrderError::InvalidOperation(CommandErrorCode::InvalidShares, format!(
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
            metadata.operator_id,
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
        let change = validate_tendered_and_change(self.tendered, amount_f64)?;
        let payment_id = uuid::Uuid::new_v4().to_string();
        let seq2 = ctx.next_sequence();
        let paid_event = OrderEvent::new(
            seq2,
            self.order_id.clone(),
            metadata.operator_id,
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
                CommandErrorCode::AaSplitNotStarted,
                "AA split not started. Use StartAaSplit first".to_string(),
            )
        })?;

        if self.shares < 1 {
            return Err(OrderError::InvalidAmount);
        }

        let remaining_shares = total_shares - snapshot.aa_paid_shares;
        if self.shares > remaining_shares {
            return Err(OrderError::InvalidOperation(CommandErrorCode::InvalidShares, format!(
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

        let change = validate_tendered_and_change(self.tendered, amount_f64)?;
        let payment_id = uuid::Uuid::new_v4().to_string();
        let seq = ctx.next_sequence();

        let progress_paid = snapshot.aa_paid_shares + self.shares;

        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id,
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
