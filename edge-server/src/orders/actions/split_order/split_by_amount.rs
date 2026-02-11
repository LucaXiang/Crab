//! SplitByAmount (金额分单) — pays a fixed amount without item tracking

use async_trait::async_trait;

use crate::orders::money::{to_decimal, to_f64, MONEY_TOLERANCE};
use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::types::CommandErrorCode;
use shared::order::{EventPayload, OrderEvent, OrderEventType};

use super::{validate_active_order, validate_split_mode_allowed, validate_tendered_and_change, SplitMode};

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
            return Err(OrderError::InvalidOperation(CommandErrorCode::SplitExceedsRemaining, format!(
                "Split amount ({:.2}) exceeds remaining unpaid ({:.2})",
                self.split_amount,
                to_f64(remaining)
            )));
        }

        let change = validate_tendered_and_change(self.tendered, self.split_amount)?;
        let payment_id = uuid::Uuid::new_v4().to_string();
        let seq = ctx.next_sequence();

        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id,
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
