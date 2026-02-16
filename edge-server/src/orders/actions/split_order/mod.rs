//! Split order command handlers
//!
//! Four independent action handlers for split payments:
//! - **SplitByItems** (菜品分单): items provided, backend calculates amount
//! - **SplitByAmount** (金额分单): amount provided, no item tracking
//! - **StartAASplit** (AA 开始): lock headcount + pay first share
//! - **PayAASplit** (AA 后续支付): pay additional shares

mod aa_split;
mod split_by_amount;
mod split_by_items;

pub use aa_split::{PayAaSplitAction, StartAaSplitAction};
pub use split_by_amount::SplitByAmountAction;
pub use split_by_items::SplitByItemsAction;

use crate::order_money::{MONEY_TOLERANCE, calculate_unit_price, to_decimal, to_f64};
use crate::orders::traits::OrderError;
use rust_decimal::Decimal;
use shared::order::types::CommandErrorCode;
use shared::order::{OrderSnapshot, OrderStatus, SplitItem};

// ============================================================================
// Shared validation
// ============================================================================

pub(super) fn validate_active_order(
    snapshot: &OrderSnapshot,
    order_id: &str,
) -> Result<(), OrderError> {
    match snapshot.status {
        OrderStatus::Active => Ok(()),
        OrderStatus::Completed => Err(OrderError::OrderAlreadyCompleted(order_id.to_string())),
        OrderStatus::Void => Err(OrderError::OrderAlreadyVoided(order_id.to_string())),
        _ => Err(OrderError::OrderNotFound(order_id.to_string())),
    }
}

pub(super) enum SplitMode {
    Item,
    Amount,
    Aa,
}

/// Validate that a specific split mode is allowed given the current snapshot state.
pub(super) fn validate_split_mode_allowed(
    snapshot: &OrderSnapshot,
    mode: SplitMode,
) -> Result<(), OrderError> {
    // AA mode active → only AA payments allowed
    if snapshot.aa_total_shares.is_some() {
        if !matches!(mode, SplitMode::Aa) {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::AaSplitActive,
                "AA split is active. Only AA share payments are allowed".to_string(),
            ));
        }
        return Ok(());
    }

    // Amount split active → block item split only, allow AA
    if snapshot.has_amount_split {
        if matches!(mode, SplitMode::Item) {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::ItemSplitBlocked,
                "Item-based split is disabled while amount-based split payments exist".to_string(),
            ));
        }
        return Ok(()); // Amount and AA both OK
    }

    // Item split active → does not block any mode
    // (item split only marks specific items as paid, remaining amount is still available)

    Ok(())
}

/// Validate items exist in order and have sufficient quantity.
/// Returns the calculated amount from items.
pub(super) fn validate_items_and_calculate(
    snapshot: &OrderSnapshot,
    items: &[SplitItem],
) -> Result<Decimal, OrderError> {
    // Reject duplicate instance_ids (would double-count amounts)
    {
        let mut seen = std::collections::HashSet::new();
        for item in items {
            if !seen.insert(&item.instance_id) {
                return Err(OrderError::InvalidOperation(
                    CommandErrorCode::DuplicateSplitItem,
                    format!(
                        "Duplicate instance_id '{}' in split items",
                        item.instance_id
                    ),
                ));
            }
        }
    }

    let mut calculated_amount = Decimal::ZERO;
    for split_item in items {
        let order_item = snapshot
            .items
            .iter()
            .find(|i| i.instance_id == split_item.instance_id)
            .ok_or_else(|| OrderError::ItemNotFound(split_item.instance_id.clone()))?;

        // Reject comped items — they are free and cannot be split
        if order_item.is_comped {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::CannotSplitComped,
                format!("Cannot split comped item '{}'", order_item.name),
            ));
        }

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

/// Validate tendered amount is sufficient, return change amount.
pub(super) fn validate_tendered_and_change(
    tendered: Option<f64>,
    amount_f64: f64,
) -> Result<Option<f64>, OrderError> {
    if let Some(t) = tendered
        && to_decimal(t) < to_decimal(amount_f64) - MONEY_TOLERANCE
    {
        return Err(OrderError::InvalidOperation(
            CommandErrorCode::InsufficientTender,
            format!("Tendered {:.2} is less than required {:.2}", t, amount_f64),
        ));
    }
    let change = tendered.map(|t| {
        let diff = to_decimal(t) - to_decimal(amount_f64);
        to_f64(diff.max(Decimal::ZERO))
    });
    Ok(change)
}

#[cfg(test)]
mod tests;
