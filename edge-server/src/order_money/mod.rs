//! Money calculation utilities using rust_decimal for precision
//!
//! This module provides precise decimal arithmetic for monetary calculations.
//! All calculations are done using `Decimal` internally, then converted to `f64`
//! for storage/serialization.

use crate::orders::traits::OrderError;
use crate::utils::validation::{MAX_NAME_LEN, MAX_NOTE_LEN, validate_order_optional_text};
use rust_decimal::prelude::*;
use shared::models::price_rule::{AdjustmentType, RuleType};
use shared::order::types::CommandErrorCode;
use shared::order::{
    CartItemInput, CartItemSnapshot, ItemChanges, MAX_OPTION_QUANTITY, OrderSnapshot, PaymentInput,
};

/// Rounding strategy for monetary values (2 decimal places, half-up)
const DECIMAL_PLACES: u32 = 2;

/// Tolerance for monetary comparisons (0.01)
pub const MONEY_TOLERANCE: Decimal = Decimal::from_parts(1, 0, 0, false, 2);

/// Maximum allowed price per item (€1,000,000)
const MAX_PRICE: f64 = 1_000_000.0;
/// Maximum allowed quantity per item
const MAX_QUANTITY: i32 = 9999;
/// Maximum allowed payment amount (€1,000,000)
const MAX_PAYMENT_AMOUNT: f64 = 1_000_000.0;

/// Validate that a f64 value is finite (not NaN, not Infinity)
#[inline]
fn require_finite(value: f64, field_name: &str) -> Result<(), OrderError> {
    if !value.is_finite() {
        return Err(OrderError::InvalidOperation(
            CommandErrorCode::InvalidAmount,
            format!("{} must be a finite number, got {}", field_name, value),
        ));
    }
    Ok(())
}

/// Validate a CartItemInput before processing
pub fn validate_cart_item(item: &CartItemInput) -> Result<(), OrderError> {
    // Price must be finite and non-negative
    require_finite(item.price, "price")?;
    if item.price < 0.0 {
        return Err(OrderError::InvalidOperation(
            CommandErrorCode::InvalidAmount,
            format!("price must be non-negative, got {}", item.price),
        ));
    }
    if item.price > MAX_PRICE {
        return Err(OrderError::InvalidOperation(
            CommandErrorCode::InvalidAmount,
            format!(
                "price exceeds maximum allowed ({}), got {}",
                MAX_PRICE, item.price
            ),
        ));
    }

    // original_price must be finite and non-negative if present
    if let Some(op) = item.original_price {
        require_finite(op, "original_price")?;
        if op < 0.0 {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::InvalidAmount,
                format!("original_price must be non-negative, got {}", op),
            ));
        }
        if op > MAX_PRICE {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::InvalidAmount,
                format!(
                    "original_price exceeds maximum allowed ({}), got {}",
                    MAX_PRICE, op
                ),
            ));
        }
    }

    // Quantity must be positive and within bounds
    if item.quantity <= 0 {
        return Err(OrderError::InvalidOperation(
            CommandErrorCode::InvalidQuantity,
            format!("quantity must be positive, got {}", item.quantity),
        ));
    }
    if item.quantity > MAX_QUANTITY {
        return Err(OrderError::InvalidOperation(
            CommandErrorCode::InvalidQuantity,
            format!(
                "quantity exceeds maximum allowed ({}), got {}",
                MAX_QUANTITY, item.quantity
            ),
        ));
    }

    // manual_discount_percent must be in [0, 100]
    if let Some(d) = item.manual_discount_percent {
        require_finite(d, "manual_discount_percent")?;
        if !(0.0..=100.0).contains(&d) {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::InvalidAdjustmentValue,
                format!(
                    "manual_discount_percent must be between 0 and 100, got {}",
                    d
                ),
            ));
        }
    }

    // Option price modifiers and quantities must be valid
    if let Some(opts) = &item.selected_options {
        for opt in opts {
            if let Some(pm) = opt.price_modifier {
                require_finite(pm, "option price_modifier")?;
                if pm.abs() > MAX_PRICE {
                    return Err(OrderError::InvalidOperation(
                        CommandErrorCode::InvalidAmount,
                        format!("option price_modifier exceeds maximum allowed, got {}", pm),
                    ));
                }
            }
            // Validate option quantity
            if opt.quantity <= 0 {
                return Err(OrderError::InvalidOperation(
                    CommandErrorCode::InvalidQuantity,
                    format!(
                        "option quantity must be positive, got {} for option '{}'",
                        opt.quantity, opt.option_name
                    ),
                ));
            }
            // Use shared constant for max option quantity
            if opt.quantity > MAX_OPTION_QUANTITY {
                return Err(OrderError::InvalidOperation(
                    CommandErrorCode::InvalidQuantity,
                    format!(
                        "option quantity exceeds maximum allowed ({}), got {} for option '{}'",
                        MAX_OPTION_QUANTITY, opt.quantity, opt.option_name
                    ),
                ));
            }
        }
    }

    // Note and authorizer_name must be within length limits
    validate_order_optional_text(&item.note, "note", MAX_NOTE_LEN)?;
    validate_order_optional_text(&item.authorizer_name, "authorizer_name", MAX_NAME_LEN)?;

    Ok(())
}

/// Validate a PaymentInput before processing
pub fn validate_payment(payment: &PaymentInput) -> Result<(), OrderError> {
    // Amount must be finite and positive
    require_finite(payment.amount, "payment amount")?;
    if payment.amount <= 0.0 {
        return Err(OrderError::InvalidAmount);
    }
    if payment.amount > MAX_PAYMENT_AMOUNT {
        return Err(OrderError::InvalidOperation(
            CommandErrorCode::InvalidAmount,
            format!(
                "payment amount exceeds maximum allowed ({}), got {}",
                MAX_PAYMENT_AMOUNT, payment.amount
            ),
        ));
    }

    // Tendered must be finite if present
    if let Some(t) = payment.tendered {
        require_finite(t, "tendered")?;
        if t < 0.0 {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::InvalidAmount,
                "tendered amount must be non-negative".to_string(),
            ));
        }
    }

    Ok(())
}

/// Validate item changes (from ModifyItem command)
pub fn validate_item_changes(changes: &ItemChanges) -> Result<(), OrderError> {
    if let Some(p) = changes.price {
        require_finite(p, "price")?;
        if p < 0.0 {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::InvalidAmount,
                format!("price must be non-negative, got {}", p),
            ));
        }
        if p > MAX_PRICE {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::InvalidAmount,
                format!("price exceeds maximum allowed ({}), got {}", MAX_PRICE, p),
            ));
        }
    }

    if let Some(q) = changes.quantity {
        if q <= 0 {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::InvalidQuantity,
                format!("quantity must be positive, got {}", q),
            ));
        }
        if q > MAX_QUANTITY {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::InvalidQuantity,
                format!(
                    "quantity exceeds maximum allowed ({}), got {}",
                    MAX_QUANTITY, q
                ),
            ));
        }
    }

    if let Some(d) = changes.manual_discount_percent {
        require_finite(d, "manual_discount_percent")?;
        if !(0.0..=100.0).contains(&d) {
            return Err(OrderError::InvalidOperation(
                CommandErrorCode::InvalidAdjustmentValue,
                format!(
                    "manual_discount_percent must be between 0 and 100, got {}",
                    d
                ),
            ));
        }
    }

    // Validate option price modifiers if present
    if let Some(opts) = &changes.selected_options {
        for opt in opts {
            if let Some(pm) = opt.price_modifier {
                require_finite(pm, "option price_modifier")?;
                if pm.abs() > MAX_PRICE {
                    return Err(OrderError::InvalidOperation(
                        CommandErrorCode::InvalidAmount,
                        format!("option price_modifier exceeds maximum allowed, got {}", pm),
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Convert f64 to Decimal for calculation
///
/// Input values should be pre-validated via `require_finite()` at the boundary.
/// If NaN/Infinity somehow reaches here, logs an error and returns ZERO
/// to avoid silent data corruption in financial calculations.
#[inline]
pub fn to_decimal(value: f64) -> Decimal {
    Decimal::from_f64(value).unwrap_or_else(|| {
        tracing::error!(value = ?value, "Non-finite f64 in monetary calculation, defaulting to zero");
        Decimal::ZERO
    })
}

/// Convert Decimal back to f64 for storage, rounded to 2 decimal places
#[inline]
pub fn to_f64(value: Decimal) -> f64 {
    value
        .round_dp_with_strategy(DECIMAL_PLACES, RoundingStrategy::MidpointAwayFromZero)
        .to_f64()
        // SAFETY: Decimal rounded to 2dp with max input ≤ 1_000_000 (validated at boundary)
        // is always within f64 representable range (~1.8e308)
        .expect("Decimal rounded to 2dp is always representable as f64")
}

/// Compute effective per-unit rule discount, dynamically recalculating from `adjustment_value`.
/// `after_manual` is the per-unit price after manual discount (basis for percentage discounts).
/// Falls back to pre-computed `rule_discount_amount` when `applied_rules` is absent.
fn effective_rule_discount(item: &CartItemSnapshot, after_manual: Decimal) -> Decimal {
    if item.applied_rules.is_empty() {
        // Legacy fallback: use pre-computed amount
        to_decimal(item.rule_discount_amount)
    } else {
        item.applied_rules
            .iter()
            .filter(|r| !r.skipped && r.rule_type == RuleType::Discount)
            .map(|r| match r.adjustment_type {
                AdjustmentType::Percentage => (after_manual * to_decimal(r.adjustment_value)
                    / Decimal::ONE_HUNDRED)
                    .round_dp(DECIMAL_PLACES),
                AdjustmentType::FixedAmount => to_decimal(r.adjustment_value),
            })
            .sum()
    }
}

/// Compute effective per-unit rule surcharge, dynamically recalculating from `adjustment_value`.
/// `base_with_options` is the per-unit price before discounts (basis for percentage surcharges).
/// Falls back to pre-computed `rule_surcharge_amount` when `applied_rules` is absent.
fn effective_rule_surcharge(item: &CartItemSnapshot, base_with_options: Decimal) -> Decimal {
    if item.applied_rules.is_empty() {
        // Legacy fallback: use pre-computed amount
        to_decimal(item.rule_surcharge_amount)
    } else {
        item.applied_rules
            .iter()
            .filter(|r| !r.skipped && r.rule_type == RuleType::Surcharge)
            .map(|r| match r.adjustment_type {
                AdjustmentType::Percentage => (base_with_options * to_decimal(r.adjustment_value)
                    / Decimal::ONE_HUNDRED)
                    .round_dp(DECIMAL_PLACES),
                AdjustmentType::FixedAmount => to_decimal(r.adjustment_value),
            })
            .sum()
    }
}

/// Compute effective order-level rule discount, dynamically recalculating from `adjustment_value`.
/// `subtotal` is the order subtotal (basis for percentage order-level discounts).
/// Falls back to pre-computed `order_rule_discount_amount` when `order_applied_rules` is absent.
fn effective_order_rule_discount(snapshot: &OrderSnapshot, subtotal: Decimal) -> Decimal {
    if snapshot.order_applied_rules.is_empty() {
        // Legacy fallback: use pre-computed amount
        to_decimal(snapshot.order_rule_discount_amount)
    } else {
        snapshot
            .order_applied_rules
            .iter()
            .filter(|r| !r.skipped && r.rule_type == RuleType::Discount)
            .map(|r| match r.adjustment_type {
                AdjustmentType::Percentage => (subtotal * to_decimal(r.adjustment_value)
                    / Decimal::ONE_HUNDRED)
                    .round_dp(DECIMAL_PLACES),
                AdjustmentType::FixedAmount => to_decimal(r.adjustment_value),
            })
            .sum()
    }
}

/// Compute effective order-level rule surcharge, dynamically recalculating from `adjustment_value`.
/// `subtotal` is the order subtotal (basis for percentage order-level surcharges).
/// Falls back to pre-computed `order_rule_surcharge_amount` when `order_applied_rules` is absent.
fn effective_order_rule_surcharge(snapshot: &OrderSnapshot, subtotal: Decimal) -> Decimal {
    if snapshot.order_applied_rules.is_empty() {
        // Legacy fallback: use pre-computed amount
        to_decimal(snapshot.order_rule_surcharge_amount)
    } else {
        snapshot
            .order_applied_rules
            .iter()
            .filter(|r| !r.skipped && r.rule_type == RuleType::Surcharge)
            .map(|r| match r.adjustment_type {
                AdjustmentType::Percentage => (subtotal * to_decimal(r.adjustment_value)
                    / Decimal::ONE_HUNDRED)
                    .round_dp(DECIMAL_PLACES),
                AdjustmentType::FixedAmount => to_decimal(r.adjustment_value),
            })
            .sum()
    }
}

/// Compute effective MG discount by re-applying multiplicative stacking from `adjustment_value`.
/// `after_rules` is the per-unit price after manual discount and price rule adjustments.
fn effective_mg_discount(item: &CartItemSnapshot, after_rules: Decimal) -> Decimal {
    if item.applied_mg_rules.is_empty() {
        return Decimal::ZERO;
    }

    let mut running = after_rules;
    for rule in &item.applied_mg_rules {
        if rule.skipped {
            continue;
        }
        match rule.adjustment_type {
            AdjustmentType::Percentage => {
                let rate = to_decimal(rule.adjustment_value) / Decimal::ONE_HUNDRED;
                running *= Decimal::ONE - rate;
            }
            AdjustmentType::FixedAmount => {
                running = (running - to_decimal(rule.adjustment_value)).max(Decimal::ZERO);
            }
        }
    }
    (after_rules - running).max(Decimal::ZERO)
}

/// Calculate item unit price with precise decimal arithmetic
///
/// Formula: base_price * (1 - manual_discount_percent/100) - rule_discount + rule_surcharge - mg_discount
/// where base_price = original_price (base price for calculations, updated on manual repricing)
///
/// This is the final per-unit price shown to customers
pub fn calculate_unit_price(item: &CartItemSnapshot) -> Decimal {
    // Comped items are always free — skip all calculations
    if item.is_comped {
        return Decimal::ZERO;
    }

    // Use original_price as the base for calculations (updated on manual repricing/spec change)
    let base_price = to_decimal(if item.original_price > 0.0 {
        item.original_price
    } else {
        item.price
    });

    // Options modifier: sum of (price_modifier × quantity) for each selected option
    let options_modifier: Decimal = item
        .selected_options
        .as_ref()
        .map(|opts| {
            opts.iter()
                .filter_map(|o| {
                    o.price_modifier
                        .map(|p| to_decimal(p) * Decimal::from(o.quantity))
                })
                .sum()
        })
        .unwrap_or(Decimal::ZERO);

    // Base with options = spec price + options (clamped to >= 0)
    let base_with_options = (base_price + options_modifier).max(Decimal::ZERO);

    // Manual discount is percentage-based on the full base (including options)
    let manual_discount = item
        .manual_discount_percent
        .map(|d| base_with_options * to_decimal(d) / Decimal::ONE_HUNDRED)
        .unwrap_or(Decimal::ZERO);

    // Rule discount/surcharge: dynamically recalculate from adjustment_value
    let after_manual = base_with_options - manual_discount;
    let rule_discount = effective_rule_discount(item, after_manual);
    let rule_surcharge = effective_rule_surcharge(item, base_with_options);

    // Price after manual + rule adjustments
    let after_rules = base_with_options - manual_discount - rule_discount + rule_surcharge;

    // MG discount (applied multiplicatively after price rules)
    let mg_discount = effective_mg_discount(item, after_rules);

    // Final unit price
    let unit_price = after_rules - mg_discount;

    unit_price
        .max(Decimal::ZERO) // Ensure non-negative
        .round_dp_with_strategy(DECIMAL_PLACES, RoundingStrategy::MidpointAwayFromZero)
}

/// Calculate item line total with precise decimal arithmetic
///
/// Formula: unit_price * quantity
pub fn calculate_item_total(item: &CartItemSnapshot) -> Decimal {
    let unit_price = calculate_unit_price(item);
    let quantity = Decimal::from(item.quantity);

    (unit_price * quantity)
        .round_dp_with_strategy(DECIMAL_PLACES, RoundingStrategy::MidpointAwayFromZero)
}

/// Recalculate order totals from items using precise decimal arithmetic
///
/// This function calculates all financial totals:
/// - original_total: sum of base prices before any adjustments
/// - subtotal: sum of line totals (after item-level adjustments)
/// - total_discount: item-level + order-level discounts
/// - total_surcharge: item-level + order-level surcharges
/// - total: final amount to pay
/// - remaining_amount: total - paid_amount
///
/// Also resets `is_pre_payment` to false if total changes (prepaid receipt invalidated)
pub fn recalculate_totals(snapshot: &mut OrderSnapshot) {
    // Save old total for pre-payment check
    let old_total = to_decimal(snapshot.total);

    let mut original_total = Decimal::ZERO;
    let mut subtotal = Decimal::ZERO;
    let mut item_discount_total = Decimal::ZERO;
    let mut item_surcharge_total = Decimal::ZERO;
    let mut item_mg_discount_total = Decimal::ZERO;
    let mut comp_total = Decimal::ZERO;
    let mut total_tax = Decimal::ZERO;

    for item in &mut snapshot.items {
        let quantity = Decimal::from(item.quantity);

        // Update unpaid_quantity
        let paid_qty = snapshot
            .paid_item_quantities
            .get(&item.instance_id)
            .copied()
            .unwrap_or(0);
        item.unpaid_quantity = (item.quantity - paid_qty).max(0);

        // Calculate base price + options modifier
        let base_price = to_decimal(if item.original_price > 0.0 {
            item.original_price
        } else {
            item.price
        });
        // Options modifier: sum of (price_modifier × quantity) for each selected option
        let options_modifier: Decimal = item
            .selected_options
            .as_ref()
            .map(|opts| {
                opts.iter()
                    .filter_map(|o| {
                        o.price_modifier
                            .map(|p| to_decimal(p) * Decimal::from(o.quantity))
                    })
                    .sum()
            })
            .unwrap_or(Decimal::ZERO);
        let base_with_options = (base_price + options_modifier).max(Decimal::ZERO);
        original_total += base_with_options * quantity;

        // Calculate item-level discount (based on full base including options)
        // Comped items keep their applied_rules/manual_discount for uncomp restoration,
        // but should not contribute to discount/surcharge totals (they're free).
        let manual_discount = item
            .manual_discount_percent
            .map(|d| base_with_options * to_decimal(d) / Decimal::ONE_HUNDRED)
            .unwrap_or(Decimal::ZERO);
        let after_manual = base_with_options - manual_discount;
        let rule_discount = effective_rule_discount(item, after_manual);
        if !item.is_comped {
            item_discount_total += (manual_discount + rule_discount) * quantity;
        }

        // Calculate item-level surcharge (from rules only)
        let rule_surcharge = effective_rule_surcharge(item, base_with_options);
        if !item.is_comped {
            item_surcharge_total += rule_surcharge * quantity;
        }

        // Calculate MG discount (applied after price rules)
        let after_rules = base_with_options - manual_discount - rule_discount + rule_surcharge;
        let mg_discount = effective_mg_discount(item, after_rules);
        item.mg_discount_amount = to_f64(mg_discount);
        if !item.is_comped {
            item_mg_discount_total += mg_discount * quantity;
        }

        // Sync calculated_amount in applied_mg_rules
        {
            let mut running = after_rules;
            for rule in item.applied_mg_rules.iter_mut() {
                if rule.skipped {
                    continue;
                }
                let price_before = running;
                match rule.adjustment_type {
                    AdjustmentType::Percentage => {
                        let rate = to_decimal(rule.adjustment_value) / Decimal::ONE_HUNDRED;
                        running *= Decimal::ONE - rate;
                    }
                    AdjustmentType::FixedAmount => {
                        running = (running - to_decimal(rule.adjustment_value)).max(Decimal::ZERO);
                    }
                }
                rule.calculated_amount = to_f64(price_before - running);
            }
        }

        // Sync calculated_amount in applied_rules so snapshot stays consistent
        for rule in item.applied_rules.iter_mut() {
            if rule.skipped {
                continue;
            }
            let basis = match rule.rule_type {
                RuleType::Discount => after_manual,
                RuleType::Surcharge => base_with_options,
            };
            rule.calculated_amount = to_f64(match rule.adjustment_type {
                AdjustmentType::Percentage => (basis * to_decimal(rule.adjustment_value)
                    / Decimal::ONE_HUNDRED)
                    .round_dp(DECIMAL_PLACES),
                AdjustmentType::FixedAmount => to_decimal(rule.adjustment_value),
            });
        }

        // Calculate and set unit_price (final per-unit price for display)
        let unit_price = calculate_unit_price(item);
        item.unit_price = to_f64(unit_price);
        // Sync item.price to match computed unit_price (keeps price = "final price after rules")
        item.price = to_f64(unit_price);

        // Calculate and set line_total (unit_price * quantity)
        let item_total = unit_price * quantity;
        item.line_total = to_f64(item_total);

        // Calculate item tax (Spain IVA: prices are tax-inclusive)
        // Formula: tax = gross_amount * tax_rate / (100 + tax_rate)
        let tax_rate = Decimal::from(item.tax_rate);
        let item_tax = if tax_rate > Decimal::ZERO {
            item_total * tax_rate / (Decimal::ONE_HUNDRED + tax_rate)
        } else {
            Decimal::ZERO
        };
        item.tax = to_f64(item_tax);
        total_tax += item_tax;

        // Accumulate comp total (original value of comped items)
        // Use original_price for comp value since item.price is zeroed on comp
        if item.is_comped {
            let comp_base = to_decimal(if item.original_price > 0.0 {
                item.original_price
            } else {
                item.price
            });
            let comp_with_options = (comp_base + options_modifier).max(Decimal::ZERO);
            comp_total += comp_with_options * quantity;
        }

        // Accumulate subtotal
        subtotal += item_total;
    }

    // Order-level manual discount (computed amount)
    let order_manual_discount = snapshot
        .order_manual_discount_fixed
        .map(to_decimal)
        .unwrap_or(Decimal::ZERO)
        + snapshot
            .order_manual_discount_percent
            .map(|p| subtotal * to_decimal(p) / Decimal::ONE_HUNDRED)
            .unwrap_or(Decimal::ZERO);

    // Order-level manual surcharge (computed amount)
    let order_manual_surcharge = snapshot
        .order_manual_surcharge_fixed
        .map(to_decimal)
        .unwrap_or(Decimal::ZERO)
        + snapshot
            .order_manual_surcharge_percent
            .map(|p| subtotal * to_decimal(p) / Decimal::ONE_HUNDRED)
            .unwrap_or(Decimal::ZERO);

    // Order-level adjustments (rule amounts respect skipped flag, dynamically recalculated)
    let eff_order_rule_discount = effective_order_rule_discount(snapshot, subtotal);
    let eff_order_rule_surcharge = effective_order_rule_surcharge(snapshot, subtotal);
    let order_discount = eff_order_rule_discount + order_manual_discount;
    let order_surcharge = eff_order_rule_surcharge + order_manual_surcharge;

    // Sync calculated_amount in order_applied_rules so snapshot stays consistent
    for rule in snapshot.order_applied_rules.iter_mut() {
        if rule.skipped {
            continue;
        }
        rule.calculated_amount = to_f64(match rule.adjustment_type {
            AdjustmentType::Percentage => (subtotal * to_decimal(rule.adjustment_value)
                / Decimal::ONE_HUNDRED)
                .round_dp(DECIMAL_PLACES),
            AdjustmentType::FixedAmount => to_decimal(rule.adjustment_value),
        });
    }

    // Total discount and surcharge (item-level + order-level, MG tracked separately in mg_discount_amount)
    let total_discount = item_discount_total + order_discount;
    let total_surcharge = item_surcharge_total + order_surcharge;

    // Final total (Spanish IVA: tax is already included in subtotal)
    // Clamp to zero — extreme discounts must not produce negative totals
    let total = (subtotal - order_discount + order_surcharge).max(Decimal::ZERO);
    let paid = to_decimal(snapshot.paid_amount);
    let remaining = (total - paid).max(Decimal::ZERO);

    // Update snapshot
    snapshot.original_total = to_f64(original_total.max(Decimal::ZERO));
    snapshot.subtotal = to_f64(subtotal.max(Decimal::ZERO));
    snapshot.total_discount = to_f64(total_discount);
    snapshot.total_surcharge = to_f64(total_surcharge);
    snapshot.tax = to_f64(total_tax);
    snapshot.discount = to_f64(order_discount);
    snapshot.comp_total_amount = to_f64(comp_total);
    snapshot.order_manual_discount_amount = to_f64(order_manual_discount);
    snapshot.order_manual_surcharge_amount = to_f64(order_manual_surcharge);
    snapshot.order_rule_discount_amount = to_f64(eff_order_rule_discount);
    snapshot.order_rule_surcharge_amount = to_f64(eff_order_rule_surcharge);
    snapshot.mg_discount_amount = to_f64(item_mg_discount_total);
    snapshot.total = to_f64(total);
    snapshot.remaining_amount = to_f64(remaining);

    // Reset pre-payment status if total changed (prepaid receipt invalidated)
    if snapshot.is_pre_payment && total != old_total {
        snapshot.is_pre_payment = false;
    }
}

/// Sum payment amounts with precise arithmetic
pub fn sum_payments(payments: &[shared::order::PaymentRecord]) -> f64 {
    let total: Decimal = payments
        .iter()
        .filter(|p| !p.cancelled)
        .map(|p| to_decimal(p.amount))
        .sum();

    to_f64(total)
}

/// Check if payment is sufficient (with small tolerance for edge cases)
///
/// Returns true if paid >= required - 0.01
pub fn is_payment_sufficient(paid: f64, required: f64) -> bool {
    let paid_dec = to_decimal(paid);
    let required_dec = to_decimal(required);
    paid_dec >= required_dec - MONEY_TOLERANCE
}

/// Compare two monetary values for equality (within 0.01 tolerance)
pub fn money_eq(a: f64, b: f64) -> bool {
    let diff = (to_decimal(a) - to_decimal(b)).abs();
    diff < MONEY_TOLERANCE
}

#[cfg(test)]
mod tests;
