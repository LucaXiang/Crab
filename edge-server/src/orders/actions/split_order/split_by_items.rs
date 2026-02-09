//! SplitByItems (菜品分单) — pays for specific items

use async_trait::async_trait;

use crate::orders::money::{to_decimal, to_f64, MONEY_TOLERANCE};
use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{EventPayload, OrderEvent, OrderEventType, SplitItem};

use super::{validate_active_order, validate_items_and_calculate, validate_split_mode_allowed, validate_tendered_and_change, SplitMode};

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

        let change = validate_tendered_and_change(self.tendered, amount_f64)?;
        let payment_id = uuid::Uuid::new_v4().to_string();
        let seq = ctx.next_sequence();

        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id,
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
