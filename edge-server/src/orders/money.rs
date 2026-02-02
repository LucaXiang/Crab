//! Money calculation utilities using rust_decimal for precision
//!
//! This module provides precise decimal arithmetic for monetary calculations.
//! All calculations are done using `Decimal` internally, then converted to `f64`
//! for storage/serialization.

use crate::orders::traits::OrderError;
use rust_decimal::prelude::*;
use shared::models::price_rule::{AdjustmentType, RuleType};
use shared::order::{CartItemInput, CartItemSnapshot, ItemChanges, OrderSnapshot, PaymentInput};

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
        return Err(OrderError::InvalidOperation(format!(
            "{} must be a finite number, got {}",
            field_name, value
        )));
    }
    Ok(())
}

/// Validate a CartItemInput before processing
pub fn validate_cart_item(item: &CartItemInput) -> Result<(), OrderError> {
    // Price must be finite and non-negative
    require_finite(item.price, "price")?;
    if item.price < 0.0 {
        return Err(OrderError::InvalidOperation(format!(
            "price must be non-negative, got {}",
            item.price
        )));
    }
    if item.price > MAX_PRICE {
        return Err(OrderError::InvalidOperation(format!(
            "price exceeds maximum allowed ({}), got {}",
            MAX_PRICE, item.price
        )));
    }

    // original_price must be finite and non-negative if present
    if let Some(op) = item.original_price {
        require_finite(op, "original_price")?;
        if op < 0.0 {
            return Err(OrderError::InvalidOperation(format!(
                "original_price must be non-negative, got {}",
                op
            )));
        }
        if op > MAX_PRICE {
            return Err(OrderError::InvalidOperation(format!(
                "original_price exceeds maximum allowed ({}), got {}",
                MAX_PRICE, op
            )));
        }
    }

    // Quantity must be positive and within bounds
    if item.quantity <= 0 {
        return Err(OrderError::InvalidOperation(format!(
            "quantity must be positive, got {}",
            item.quantity
        )));
    }
    if item.quantity > MAX_QUANTITY {
        return Err(OrderError::InvalidOperation(format!(
            "quantity exceeds maximum allowed ({}), got {}",
            MAX_QUANTITY, item.quantity
        )));
    }

    // manual_discount_percent must be in [0, 100]
    if let Some(d) = item.manual_discount_percent {
        require_finite(d, "manual_discount_percent")?;
        if !(0.0..=100.0).contains(&d) {
            return Err(OrderError::InvalidOperation(format!(
                "manual_discount_percent must be between 0 and 100, got {}",
                d
            )));
        }
    }

    // Option price modifiers must be finite
    if let Some(opts) = &item.selected_options {
        for opt in opts {
            if let Some(pm) = opt.price_modifier {
                require_finite(pm, "option price_modifier")?;
                if pm.abs() > MAX_PRICE {
                    return Err(OrderError::InvalidOperation(format!(
                        "option price_modifier exceeds maximum allowed, got {}",
                        pm
                    )));
                }
            }
        }
    }

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
        return Err(OrderError::InvalidOperation(format!(
            "payment amount exceeds maximum allowed ({}), got {}",
            MAX_PAYMENT_AMOUNT, payment.amount
        )));
    }

    // Tendered must be finite if present
    if let Some(t) = payment.tendered {
        require_finite(t, "tendered")?;
        if t < 0.0 {
            return Err(OrderError::InvalidOperation(
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
            return Err(OrderError::InvalidOperation(format!(
                "price must be non-negative, got {}", p
            )));
        }
        if p > MAX_PRICE {
            return Err(OrderError::InvalidOperation(format!(
                "price exceeds maximum allowed ({}), got {}", MAX_PRICE, p
            )));
        }
    }

    if let Some(q) = changes.quantity {
        if q <= 0 {
            return Err(OrderError::InvalidOperation(format!(
                "quantity must be positive, got {}", q
            )));
        }
        if q > MAX_QUANTITY {
            return Err(OrderError::InvalidOperation(format!(
                "quantity exceeds maximum allowed ({}), got {}", MAX_QUANTITY, q
            )));
        }
    }

    if let Some(d) = changes.manual_discount_percent {
        require_finite(d, "manual_discount_percent")?;
        if !(0.0..=100.0).contains(&d) {
            return Err(OrderError::InvalidOperation(format!(
                "manual_discount_percent must be between 0 and 100, got {}", d
            )));
        }
    }

    // Validate option price modifiers if present
    if let Some(opts) = &changes.selected_options {
        for opt in opts {
            if let Some(pm) = opt.price_modifier {
                require_finite(pm, "option price_modifier")?;
                if pm.abs() > MAX_PRICE {
                    return Err(OrderError::InvalidOperation(format!(
                        "option price_modifier exceeds maximum allowed, got {}", pm
                    )));
                }
            }
        }
    }

    Ok(())
}

/// Convert f64 to Decimal for calculation
#[inline]
pub fn to_decimal(value: f64) -> Decimal {
    Decimal::from_f64(value).unwrap_or_default()
}

/// Convert Decimal back to f64 for storage, rounded to 2 decimal places
#[inline]
pub fn to_f64(value: Decimal) -> f64 {
    value
        .round_dp_with_strategy(DECIMAL_PLACES, RoundingStrategy::MidpointAwayFromZero)
        .to_f64()
        .unwrap_or_default()
}

/// Compute effective per-unit rule discount, dynamically recalculating from `adjustment_value`.
/// `after_manual` is the per-unit price after manual discount (basis for percentage discounts).
/// Falls back to pre-computed `rule_discount_amount` when `applied_rules` is absent.
fn effective_rule_discount(item: &CartItemSnapshot, after_manual: Decimal) -> Decimal {
    match &item.applied_rules {
        Some(rules) => rules
            .iter()
            .filter(|r| !r.skipped && r.rule_type == RuleType::Discount)
            .map(|r| match r.adjustment_type {
                AdjustmentType::Percentage => {
                    (after_manual * to_decimal(r.adjustment_value) / Decimal::ONE_HUNDRED)
                        .round_dp(DECIMAL_PLACES)
                }
                AdjustmentType::FixedAmount => to_decimal(r.adjustment_value),
            })
            .sum(),
        None => item
            .rule_discount_amount
            .map(to_decimal)
            .unwrap_or(Decimal::ZERO),
    }
}

/// Compute effective per-unit rule surcharge, dynamically recalculating from `adjustment_value`.
/// `base_with_options` is the per-unit price before discounts (basis for percentage surcharges).
/// Falls back to pre-computed `rule_surcharge_amount` when `applied_rules` is absent.
fn effective_rule_surcharge(item: &CartItemSnapshot, base_with_options: Decimal) -> Decimal {
    match &item.applied_rules {
        Some(rules) => rules
            .iter()
            .filter(|r| !r.skipped && r.rule_type == RuleType::Surcharge)
            .map(|r| match r.adjustment_type {
                AdjustmentType::Percentage => {
                    (base_with_options * to_decimal(r.adjustment_value) / Decimal::ONE_HUNDRED)
                        .round_dp(DECIMAL_PLACES)
                }
                AdjustmentType::FixedAmount => to_decimal(r.adjustment_value),
            })
            .sum(),
        None => item
            .rule_surcharge_amount
            .map(to_decimal)
            .unwrap_or(Decimal::ZERO),
    }
}

/// Compute effective order-level rule discount, dynamically recalculating from `adjustment_value`.
/// `subtotal` is the order subtotal (basis for percentage order-level discounts).
/// Falls back to pre-computed `order_rule_discount_amount` when `order_applied_rules` is absent.
fn effective_order_rule_discount(snapshot: &OrderSnapshot, subtotal: Decimal) -> Decimal {
    match &snapshot.order_applied_rules {
        Some(rules) => rules
            .iter()
            .filter(|r| !r.skipped && r.rule_type == RuleType::Discount)
            .map(|r| match r.adjustment_type {
                AdjustmentType::Percentage => {
                    (subtotal * to_decimal(r.adjustment_value) / Decimal::ONE_HUNDRED)
                        .round_dp(DECIMAL_PLACES)
                }
                AdjustmentType::FixedAmount => to_decimal(r.adjustment_value),
            })
            .sum(),
        None => snapshot
            .order_rule_discount_amount
            .map(to_decimal)
            .unwrap_or(Decimal::ZERO),
    }
}

/// Compute effective order-level rule surcharge, dynamically recalculating from `adjustment_value`.
/// `subtotal` is the order subtotal (basis for percentage order-level surcharges).
/// Falls back to pre-computed `order_rule_surcharge_amount` when `order_applied_rules` is absent.
fn effective_order_rule_surcharge(snapshot: &OrderSnapshot, subtotal: Decimal) -> Decimal {
    match &snapshot.order_applied_rules {
        Some(rules) => rules
            .iter()
            .filter(|r| !r.skipped && r.rule_type == RuleType::Surcharge)
            .map(|r| match r.adjustment_type {
                AdjustmentType::Percentage => {
                    (subtotal * to_decimal(r.adjustment_value) / Decimal::ONE_HUNDRED)
                        .round_dp(DECIMAL_PLACES)
                }
                AdjustmentType::FixedAmount => to_decimal(r.adjustment_value),
            })
            .sum(),
        None => snapshot
            .order_rule_surcharge_amount
            .map(to_decimal)
            .unwrap_or(Decimal::ZERO),
    }
}

/// Calculate item unit price with precise decimal arithmetic
///
/// Formula: base_price * (1 - manual_discount_percent/100) - rule_discount + rule_surcharge
/// where base_price = original_price if available, otherwise price
///
/// This is the final per-unit price shown to customers
pub fn calculate_unit_price(item: &CartItemSnapshot) -> Decimal {
    // Comped items are always free — skip all calculations
    if item.is_comped {
        return Decimal::ZERO;
    }

    // Use original_price as the base for discount calculation (before any discounts)
    let base_price = to_decimal(item.original_price.unwrap_or(item.price));

    // Options modifier: sum of all selected option price modifiers
    let options_modifier: Decimal = item
        .selected_options
        .as_ref()
        .map(|opts| opts.iter().filter_map(|o| o.price_modifier.map(to_decimal)).sum())
        .unwrap_or(Decimal::ZERO);

    // Base with options = spec price + options
    let base_with_options = base_price + options_modifier;

    // Manual discount is percentage-based on the full base (including options)
    let manual_discount = item
        .manual_discount_percent
        .map(|d| base_with_options * to_decimal(d) / Decimal::ONE_HUNDRED)
        .unwrap_or(Decimal::ZERO);

    // Rule discount/surcharge: dynamically recalculate from adjustment_value
    let after_manual = base_with_options - manual_discount;
    let rule_discount = effective_rule_discount(item, after_manual);
    let rule_surcharge = effective_rule_surcharge(item, base_with_options);

    // Final unit price = base_with_options - discounts + rule surcharges
    let unit_price = base_with_options - manual_discount - rule_discount + rule_surcharge;

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

    (unit_price * quantity).round_dp_with_strategy(DECIMAL_PLACES, RoundingStrategy::MidpointAwayFromZero)
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

        // Calculate original price (base price before any adjustments) + options modifier
        let base_price = to_decimal(item.original_price.unwrap_or(item.price));
        let options_modifier: Decimal = item
            .selected_options
            .as_ref()
            .map(|opts| opts.iter().filter_map(|o| o.price_modifier.map(to_decimal)).sum())
            .unwrap_or(Decimal::ZERO);
        let base_with_options = base_price + options_modifier;
        original_total += base_with_options * quantity;

        // Calculate item-level discount (based on full base including options)
        let manual_discount = item
            .manual_discount_percent
            .map(|d| base_with_options * to_decimal(d) / Decimal::ONE_HUNDRED)
            .unwrap_or(Decimal::ZERO);
        let after_manual = base_with_options - manual_discount;
        let rule_discount = effective_rule_discount(item, after_manual);
        item_discount_total += (manual_discount + rule_discount) * quantity;

        // Calculate item-level surcharge (from rules only)
        let rule_surcharge = effective_rule_surcharge(item, base_with_options);
        item_surcharge_total += rule_surcharge * quantity;

        // Sync calculated_amount in applied_rules so snapshot stays consistent
        if let Some(ref mut rules) = item.applied_rules {
            for rule in rules.iter_mut() {
                if rule.skipped {
                    continue;
                }
                let basis = match rule.rule_type {
                    RuleType::Discount => after_manual,
                    RuleType::Surcharge => base_with_options,
                };
                rule.calculated_amount = to_f64(match rule.adjustment_type {
                    AdjustmentType::Percentage => {
                        (basis * to_decimal(rule.adjustment_value) / Decimal::ONE_HUNDRED)
                            .round_dp(DECIMAL_PLACES)
                    }
                    AdjustmentType::FixedAmount => to_decimal(rule.adjustment_value),
                });
            }
        }

        // Calculate and set unit_price (final per-unit price for display)
        let unit_price = calculate_unit_price(item);
        item.unit_price = Some(to_f64(unit_price));

        // Calculate and set line_total (unit_price * quantity)
        let item_total = unit_price * quantity;
        item.line_total = Some(to_f64(item_total));

        // Calculate item tax (Spain IVA: prices are tax-inclusive)
        // Formula: tax = gross_amount * tax_rate / (100 + tax_rate)
        let tax_rate = Decimal::from(item.tax_rate.unwrap_or(0));
        let item_tax = if tax_rate > Decimal::ZERO {
            item_total * tax_rate / (Decimal::ONE_HUNDRED + tax_rate)
        } else {
            Decimal::ZERO
        };
        item.tax = Some(to_f64(item_tax));
        total_tax += item_tax;

        // Accumulate comp total (original value of comped items)
        if item.is_comped {
            comp_total += base_with_options * quantity;
        }

        // Accumulate subtotal
        subtotal += item_total;
    }

    // Order-level manual discount (computed amount)
    let order_manual_discount =
        snapshot.order_manual_discount_fixed.map(to_decimal).unwrap_or(Decimal::ZERO)
        + snapshot.order_manual_discount_percent
            .map(|p| subtotal * to_decimal(p) / Decimal::ONE_HUNDRED)
            .unwrap_or(Decimal::ZERO);

    // Order-level manual surcharge (computed amount)
    let order_manual_surcharge =
        snapshot.order_manual_surcharge_fixed.map(to_decimal).unwrap_or(Decimal::ZERO)
        + snapshot.order_manual_surcharge_percent
            .map(|p| subtotal * to_decimal(p) / Decimal::ONE_HUNDRED)
            .unwrap_or(Decimal::ZERO);

    // Order-level adjustments (rule amounts respect skipped flag, dynamically recalculated)
    let eff_order_rule_discount = effective_order_rule_discount(snapshot, subtotal);
    let eff_order_rule_surcharge = effective_order_rule_surcharge(snapshot, subtotal);
    let order_discount = eff_order_rule_discount + order_manual_discount;
    let order_surcharge = eff_order_rule_surcharge + order_manual_surcharge;

    // Sync calculated_amount in order_applied_rules so snapshot stays consistent
    if let Some(ref mut rules) = snapshot.order_applied_rules {
        for rule in rules.iter_mut() {
            if rule.skipped {
                continue;
            }
            rule.calculated_amount = to_f64(match rule.adjustment_type {
                AdjustmentType::Percentage => {
                    (subtotal * to_decimal(rule.adjustment_value) / Decimal::ONE_HUNDRED)
                        .round_dp(DECIMAL_PLACES)
                }
                AdjustmentType::FixedAmount => to_decimal(rule.adjustment_value),
            });
        }
    }

    // Total discount and surcharge (item-level + order-level)
    let total_discount = item_discount_total + order_discount;
    let total_surcharge = item_surcharge_total + order_surcharge;

    // Final total (Spanish IVA: tax is already included in subtotal)
    let total = subtotal - order_discount + order_surcharge;
    let paid = to_decimal(snapshot.paid_amount);
    let remaining = (total - paid).max(Decimal::ZERO);

    // Update snapshot
    snapshot.original_total = to_f64(original_total);
    snapshot.subtotal = to_f64(subtotal);
    snapshot.total_discount = to_f64(total_discount);
    snapshot.total_surcharge = to_f64(total_surcharge);
    snapshot.tax = to_f64(total_tax);
    snapshot.discount = to_f64(order_discount);
    snapshot.comp_total_amount = to_f64(comp_total);
    snapshot.order_manual_discount_amount = to_f64(order_manual_discount);
    snapshot.order_manual_surcharge_amount = to_f64(order_manual_surcharge);
    snapshot.order_rule_discount_amount = Some(to_f64(eff_order_rule_discount));
    snapshot.order_rule_surcharge_amount = Some(to_f64(eff_order_rule_surcharge));
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
mod tests {
    use super::*;

    #[test]
    fn test_to_decimal_precision() {
        // Classic floating point problem: 0.1 + 0.2 != 0.3
        let a = 0.1_f64;
        let b = 0.2_f64;
        let sum_f64 = a + b;

        // f64 fails
        assert_ne!(sum_f64, 0.3);

        // Decimal succeeds
        let sum_dec = to_decimal(a) + to_decimal(b);
        assert_eq!(to_f64(sum_dec), 0.3);
    }

    #[test]
    fn test_accumulation_precision() {
        // Sum 0.01 one thousand times
        let mut total = Decimal::ZERO;
        for _ in 0..1000 {
            total += to_decimal(0.01);
        }
        assert_eq!(to_f64(total), 10.0);
    }

    #[test]
    fn test_calculate_item_total_no_discount() {
        let item = CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Item".to_string(),
            price: 10.99,
            original_price: None,
            quantity: 3,
            unpaid_quantity: 3,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
            unit_price: None,
            line_total: None,
            tax: None,
        tax_rate: None,
        };

        let total = calculate_item_total(&item);
        assert_eq!(to_f64(total), 32.97); // 10.99 * 3
    }

    #[test]
    fn test_calculate_item_total_with_discount() {
        let item = CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Item".to_string(),
            price: 100.0,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: Some(33.33), // Tricky percentage
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
            unit_price: None,
            line_total: None,
            tax: None,
        tax_rate: None,
        };

        let total = calculate_item_total(&item);
        assert_eq!(to_f64(total), 66.67); // 100 * (1 - 0.3333) = 66.67
    }

    #[test]
    fn test_calculate_item_total_33_percent_discount() {
        // Edge case: 33% discount on $100 should be $67.00
        let item = CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Item".to_string(),
            price: 100.0,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: Some(33.0),
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
            unit_price: None,
            line_total: None,
            tax: None,
        tax_rate: None,
        };

        let total = calculate_item_total(&item);
        assert_eq!(to_f64(total), 67.0);
    }

    #[test]
    fn test_is_payment_sufficient() {
        assert!(is_payment_sufficient(100.0, 100.0));
        assert!(is_payment_sufficient(100.01, 100.0));
        assert!(is_payment_sufficient(99.995, 100.0)); // Within tolerance
        assert!(!is_payment_sufficient(99.98, 100.0)); // Outside tolerance
    }

    #[test]
    fn test_money_eq() {
        assert!(money_eq(100.0, 100.0));
        assert!(money_eq(100.004, 100.006)); // Both round to 100.00/100.01
        assert!(!money_eq(100.0, 100.02));
    }

    #[test]
    fn test_rounding_half_up() {
        // 0.005 should round up to 0.01
        let value = Decimal::new(5, 3); // 0.005
        let rounded = value.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
        assert_eq!(rounded.to_f64().unwrap(), 0.01);

        // 0.004 should round down to 0.00
        let value2 = Decimal::new(4, 3); // 0.004
        let rounded2 = value2.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
        assert_eq!(rounded2.to_f64().unwrap(), 0.0);
    }

    #[test]
    fn test_many_small_items() {
        // 100 items at $0.01 each
        let items: Vec<CartItemSnapshot> = (0..100)
            .map(|i| CartItemSnapshot {
                id: format!("p{}", i),
                instance_id: format!("i{}", i),
                name: "Penny Item".to_string(),
                price: 0.01,
                original_price: None,
                quantity: 1,
                unpaid_quantity: 1,
                selected_options: None,
                selected_specification: None,
                manual_discount_percent: None,
                rule_discount_amount: None,
                rule_surcharge_amount: None,
                applied_rules: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
                category_name: None,
                is_comped: false,
                unit_price: None,
                line_total: None,
            tax: None,
            tax_rate: None,
            })
            .collect();

        let total: Decimal = items.iter().map(|i| calculate_item_total(i)).sum();
        assert_eq!(to_f64(total), 1.0);
    }

    #[test]
    fn test_is_pre_payment_reset_when_total_changes() {
        use shared::order::OrderSnapshot;

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Item".to_string(),
            price: 100.0,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
            unit_price: None,
            line_total: None,
            tax: None,
        tax_rate: None,
        });

        // Initial calculation
        recalculate_totals(&mut snapshot);
        assert_eq!(snapshot.total, 100.0);

        // Set pre-payment flag (simulating prepaid receipt printed)
        snapshot.is_pre_payment = true;

        // Recalculate without changing items - total unchanged, is_pre_payment stays true
        recalculate_totals(&mut snapshot);
        assert!(snapshot.is_pre_payment, "is_pre_payment should stay true when total unchanged");

        // Add another item - total changes, is_pre_payment should reset
        snapshot.items.push(CartItemSnapshot {
            id: "p2".to_string(),
            instance_id: "i2".to_string(),
            name: "Item 2".to_string(),
            price: 50.0,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
            unit_price: None,
            line_total: None,
            tax: None,
        tax_rate: None,
        });

        recalculate_totals(&mut snapshot);
        assert_eq!(snapshot.total, 150.0);
        assert!(!snapshot.is_pre_payment, "is_pre_payment should reset when total changes");
    }

    #[test]
    fn test_is_pre_payment_not_affected_when_false() {
        use shared::order::OrderSnapshot;

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Item".to_string(),
            price: 100.0,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
            unit_price: None,
            line_total: None,
            tax: None,
        tax_rate: None,
        });

        // is_pre_payment is false by default
        assert!(!snapshot.is_pre_payment);

        recalculate_totals(&mut snapshot);

        // Add item and recalculate - is_pre_payment should stay false
        snapshot.items[0].price = 200.0;
        recalculate_totals(&mut snapshot);

        assert!(!snapshot.is_pre_payment, "is_pre_payment should stay false");
    }

    // ========================================================================
    // Decimal 转换边界测试
    // ========================================================================

    #[test]
    fn test_to_decimal_nan_becomes_zero() {
        // NaN 被 Decimal::from_f64 拒绝，unwrap_or_default 返回 0
        let result = to_decimal(f64::NAN);
        assert_eq!(result, Decimal::ZERO, "NaN should silently convert to 0");
    }

    #[test]
    fn test_to_decimal_infinity_becomes_zero() {
        let result = to_decimal(f64::INFINITY);
        assert_eq!(result, Decimal::ZERO, "INFINITY should silently convert to 0");

        let result_neg = to_decimal(f64::NEG_INFINITY);
        assert_eq!(result_neg, Decimal::ZERO, "NEG_INFINITY should silently convert to 0");
    }

    #[test]
    fn test_to_decimal_f64_max_becomes_zero() {
        // f64::MAX 超出 Decimal 范围
        let result = to_decimal(f64::MAX);
        assert_eq!(result, Decimal::ZERO, "f64::MAX should silently convert to 0");
    }

    #[test]
    fn test_to_decimal_f64_min_becomes_zero() {
        let result = to_decimal(f64::MIN);
        assert_eq!(result, Decimal::ZERO, "f64::MIN should silently convert to 0");
    }

    #[test]
    fn test_to_decimal_negative_price() {
        // 负价格被正常转换 (不会被拒绝)
        let result = to_decimal(-10.0);
        assert_eq!(result, Decimal::new(-10, 0));
    }

    #[test]
    fn test_to_decimal_very_large_but_valid() {
        // 1_000_000_000.99 在 Decimal 范围内
        let result = to_decimal(1_000_000_000.99);
        assert!(result > Decimal::ZERO, "Large but valid f64 should convert normally");
    }

    // ========================================================================
    // calculate_unit_price 边界测试
    // ========================================================================

    #[test]
    fn test_unit_price_negative_base_clamped_to_zero() {
        let item = CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Item".to_string(),
            price: -50.0,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
        };

        let result = calculate_unit_price(&item);
        // 负价格 clamp 到 0
        assert_eq!(result, Decimal::ZERO, "Negative price should be clamped to 0");
    }

    #[test]
    fn test_unit_price_discount_exceeding_100_percent() {
        let item = CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Item".to_string(),
            price: 100.0,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: Some(150.0), // 150% 折扣
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
        };

        let result = calculate_unit_price(&item);
        // 150% 折扣使价格变负，clamp 到 0
        assert_eq!(result, Decimal::ZERO, "150% discount should clamp to 0");
    }

    #[test]
    fn test_unit_price_nan_price_becomes_zero() {
        let item = CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Item".to_string(),
            price: f64::NAN,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
        };

        let result = calculate_unit_price(&item);
        assert_eq!(result, Decimal::ZERO, "NaN price should result in 0 unit price");
    }

    #[test]
    fn test_unit_price_infinity_price_becomes_zero() {
        let item = CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Item".to_string(),
            price: f64::INFINITY,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
        };

        let result = calculate_unit_price(&item);
        assert_eq!(result, Decimal::ZERO, "Infinity price should result in 0 unit price");
    }

    #[test]
    fn test_unit_price_negative_discount_increases_price() {
        let item = CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Item".to_string(),
            price: 100.0,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: Some(-20.0), // 负折扣 = 加价
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
        };

        let result = calculate_unit_price(&item);
        // -20% discount = +20% => 100 + 20 = 120
        assert_eq!(to_f64(result), 120.0, "Negative discount should increase price");
    }

    // ========================================================================
    // calculate_item_total 边界测试
    // ========================================================================

    #[test]
    fn test_calculate_item_total_negative_quantity() {
        let item = CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Item".to_string(),
            price: 10.0,
            original_price: None,
            quantity: -5,
            unpaid_quantity: -5,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
        };

        let result = calculate_item_total(&item);
        // 10.0 * -5 = -50.0 — 负数行总计
        assert_eq!(to_f64(result), -50.0, "Negative quantity produces negative line total");
    }

    #[test]
    fn test_calculate_item_total_zero_quantity() {
        let item = CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Item".to_string(),
            price: 10.0,
            original_price: None,
            quantity: 0,
            unpaid_quantity: 0,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
        };

        let result = calculate_item_total(&item);
        assert_eq!(to_f64(result), 0.0, "Zero quantity produces zero line total");
    }

    #[test]
    fn test_calculate_item_total_large_quantity_times_price() {
        // 大数量 × 大价格，但在 Decimal 范围内
        let item = CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Item".to_string(),
            price: 999999.99,
            original_price: None,
            quantity: 10000,
            unpaid_quantity: 10000,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
        };

        let result = calculate_item_total(&item);
        // 999999.99 * 10000 = 9_999_999_900.0
        assert_eq!(to_f64(result), 9_999_999_900.0);
    }

    // ========================================================================
    // recalculate_totals 边界测试
    // ========================================================================

    #[test]
    fn test_recalculate_totals_with_mixed_edge_items() {
        use shared::order::OrderSnapshot;

        let mut snapshot = OrderSnapshot::new("order-1".to_string());

        // 正常商品
        snapshot.items.push(CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Normal".to_string(),
            price: 10.0,
            original_price: None,
            quantity: 2,
            unpaid_quantity: 2,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
        });

        // 零价格商品
        snapshot.items.push(CartItemSnapshot {
            id: "p2".to_string(),
            instance_id: "i2".to_string(),
            name: "Free".to_string(),
            price: 0.0,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
        });

        recalculate_totals(&mut snapshot);

        assert_eq!(snapshot.subtotal, 20.0);
        assert_eq!(snapshot.total, 20.0);
        assert_eq!(snapshot.remaining_amount, 20.0);
    }

    #[test]
    fn test_recalculate_totals_order_discount_exceeds_subtotal() {
        use shared::order::OrderSnapshot;

        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Item".to_string(),
            price: 50.0,
            original_price: None,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_name: None,
            is_comped: false,
            unit_price: None,
            line_total: None,
            tax: None,
            tax_rate: None,
        });
        // 订单级固定折扣大于小计
        snapshot.order_manual_discount_fixed = Some(100.0);

        recalculate_totals(&mut snapshot);

        // total = subtotal(50) - order_discount(100) = -50, 但 remaining_amount 被 clamp 到 0
        assert_eq!(snapshot.subtotal, 50.0);
        assert_eq!(snapshot.total, -50.0, "total 可以为负 (订单折扣大于小计)");
        assert_eq!(snapshot.remaining_amount, 0.0, "remaining_amount 被 clamp 到 0");
    }

    // ========================================================================
    // is_payment_sufficient 边界测试
    // ========================================================================

    #[test]
    fn test_is_payment_sufficient_nan_values() {
        // NaN 转为 0, 所以 is_payment_sufficient(NaN, 100) → 0 >= 99.99 → false
        assert!(!is_payment_sufficient(f64::NAN, 100.0));
        // is_payment_sufficient(100, NaN) → 100 >= -0.01 → true
        assert!(is_payment_sufficient(100.0, f64::NAN));
        // is_payment_sufficient(NaN, NaN) → 0 >= -0.01 → true
        assert!(is_payment_sufficient(f64::NAN, f64::NAN));
    }

    #[test]
    fn test_is_payment_sufficient_infinity_values() {
        // Infinity → 0
        assert!(!is_payment_sufficient(f64::INFINITY, 100.0));
        assert!(is_payment_sufficient(100.0, f64::INFINITY));
    }

    // ========================================================================
    // 规则 + options + original_price 不再双重计算
    // ========================================================================

    #[test]
    fn test_unit_price_with_original_price_and_options_no_double_counting() {
        // Scenario: reducer sets original_price=Some(spec_price), price=item_final
        // money.rs should use original_price as base, add options, not double-count
        let item = CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Pizza".to_string(),
            price: 16.50,                     // item_final from reducer (already includes options)
            original_price: Some(12.0),       // spec price set by reducer
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: Some(vec![
                shared::order::ItemOption {
                    attribute_id: "attr:size".to_string(),
                    attribute_name: "Size".to_string(),
                    option_idx: 2,
                    option_name: "Large".to_string(),
                    price_modifier: Some(3.0),
                },
                shared::order::ItemOption {
                    attribute_id: "attr:topping".to_string(),
                    attribute_name: "Topping".to_string(),
                    option_idx: 0,
                    option_name: "Extra Cheese".to_string(),
                    price_modifier: Some(1.50),
                },
            ]),
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
        };

        let unit_price = calculate_unit_price(&item);
        // base_price = original_price = 12.0
        // options = 3.0 + 1.50 = 4.50
        // base_with_options = 16.50
        // No discounts, no surcharges
        // unit_price = 16.50
        assert_eq!(to_f64(unit_price), 16.50);
    }

    #[test]
    fn test_rule_discount_plus_options_plus_manual_discount() {
        // Full combination: rule_discount + options + manual_discount
        let item = CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Item".to_string(),
            price: 85.0,                      // item_final from reducer
            original_price: Some(100.0),      // spec price
            quantity: 2,
            unpaid_quantity: 2,
            selected_options: Some(vec![shared::order::ItemOption {
                attribute_id: "attr:a1".to_string(),
                attribute_name: "Extra".to_string(),
                option_idx: 0,
                option_name: "Cheese".to_string(),
                price_modifier: Some(5.0),
            }]),
            selected_specification: None,
            manual_discount_percent: Some(10.0),   // 10% off
            rule_discount_amount: Some(3.0),       // -3 per unit
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
        };

        let unit_price = calculate_unit_price(&item);
        // base_price = 100.0
        // options = 5.0
        // base_with_options = 105.0
        // manual_discount = 105.0 * 10% = 10.5
        // rule_discount = 3.0
        // unit_price = 105.0 - 10.5 - 3.0 = 91.5
        assert_eq!(to_f64(unit_price), 91.5);

        let total = calculate_item_total(&item);
        // 91.5 * 2 = 183.0
        assert_eq!(to_f64(total), 183.0);
    }

    #[test]
    fn test_rule_discount_exceeding_price_clamps_to_zero() {
        let item = CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Item".to_string(),
            price: 5.0,
            original_price: Some(10.0),
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: Some(15.0),  // Discount exceeds base price
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
        };

        let unit_price = calculate_unit_price(&item);
        // base_with_options = 10.0
        // rule_discount = 15.0 > 10.0
        // unit_price = max(0, 10.0 - 15.0) = 0
        assert_eq!(unit_price, Decimal::ZERO);
    }

    // ========================================================================
    // validate_item_changes 隔离测试
    // ========================================================================

    #[test]
    fn test_validate_item_changes_valid() {
        let changes = ItemChanges {
            price: Some(25.0),
            quantity: Some(3),
            manual_discount_percent: Some(15.0),
            note: Some("test".to_string()),
            selected_options: None,
            selected_specification: None,
        };
        assert!(validate_item_changes(&changes).is_ok());
    }

    #[test]
    fn test_validate_item_changes_all_none() {
        let changes = ItemChanges {
            price: None,
            quantity: None,
            manual_discount_percent: None,
            note: None,
            selected_options: None,
            selected_specification: None,
        };
        // All-None is technically valid (no-op)
        assert!(validate_item_changes(&changes).is_ok());
    }

    #[test]
    fn test_validate_item_changes_nan_price() {
        let changes = ItemChanges {
            price: Some(f64::NAN),
            quantity: None,
            manual_discount_percent: None,
            note: None,
            selected_options: None,
            selected_specification: None,
        };
        assert!(validate_item_changes(&changes).is_err(), "NaN price should be rejected");
    }

    #[test]
    fn test_validate_item_changes_infinity_price() {
        let changes = ItemChanges {
            price: Some(f64::INFINITY),
            quantity: None,
            manual_discount_percent: None,
            note: None,
            selected_options: None,
            selected_specification: None,
        };
        assert!(validate_item_changes(&changes).is_err(), "Infinity price should be rejected");
    }

    #[test]
    fn test_validate_item_changes_negative_price() {
        let changes = ItemChanges {
            price: Some(-5.0),
            quantity: None,
            manual_discount_percent: None,
            note: None,
            selected_options: None,
            selected_specification: None,
        };
        assert!(validate_item_changes(&changes).is_err(), "Negative price should be rejected");
    }

    #[test]
    fn test_validate_item_changes_exceeds_max_price() {
        let changes = ItemChanges {
            price: Some(MAX_PRICE + 1.0),
            quantity: None,
            manual_discount_percent: None,
            note: None,
            selected_options: None,
            selected_specification: None,
        };
        assert!(validate_item_changes(&changes).is_err(), "Price > MAX_PRICE should be rejected");
    }

    #[test]
    fn test_validate_item_changes_zero_quantity() {
        let changes = ItemChanges {
            price: None,
            quantity: Some(0),
            manual_discount_percent: None,
            note: None,
            selected_options: None,
            selected_specification: None,
        };
        assert!(validate_item_changes(&changes).is_err(), "Zero quantity should be rejected");
    }

    #[test]
    fn test_validate_item_changes_negative_quantity() {
        let changes = ItemChanges {
            price: None,
            quantity: Some(-1),
            manual_discount_percent: None,
            note: None,
            selected_options: None,
            selected_specification: None,
        };
        assert!(validate_item_changes(&changes).is_err(), "Negative quantity should be rejected");
    }

    #[test]
    fn test_validate_item_changes_exceeds_max_quantity() {
        let changes = ItemChanges {
            price: None,
            quantity: Some(MAX_QUANTITY + 1),
            manual_discount_percent: None,
            note: None,
            selected_options: None,
            selected_specification: None,
        };
        assert!(validate_item_changes(&changes).is_err(), "Quantity > MAX should be rejected");
    }

    #[test]
    fn test_validate_item_changes_discount_nan() {
        let changes = ItemChanges {
            price: None,
            quantity: None,
            manual_discount_percent: Some(f64::NAN),
            note: None,
            selected_options: None,
            selected_specification: None,
        };
        assert!(validate_item_changes(&changes).is_err(), "NaN discount should be rejected");
    }

    #[test]
    fn test_validate_item_changes_discount_negative() {
        let changes = ItemChanges {
            price: None,
            quantity: None,
            manual_discount_percent: Some(-10.0),
            note: None,
            selected_options: None,
            selected_specification: None,
        };
        assert!(validate_item_changes(&changes).is_err(), "Negative discount should be rejected");
    }

    #[test]
    fn test_validate_item_changes_discount_over_100() {
        let changes = ItemChanges {
            price: None,
            quantity: None,
            manual_discount_percent: Some(101.0),
            note: None,
            selected_options: None,
            selected_specification: None,
        };
        assert!(validate_item_changes(&changes).is_err(), "Discount > 100% should be rejected");
    }

    #[test]
    fn test_validate_item_changes_option_nan_modifier() {
        let changes = ItemChanges {
            price: None,
            quantity: None,
            manual_discount_percent: None,
            note: None,
            selected_options: Some(vec![shared::order::ItemOption {
                attribute_id: "attr:1".to_string(),
                attribute_name: "Size".to_string(),
                option_idx: 0,
                option_name: "Large".to_string(),
                price_modifier: Some(f64::NAN),
            }]),
            selected_specification: None,
        };
        assert!(validate_item_changes(&changes).is_err(), "NaN option modifier should be rejected");
    }

    #[test]
    fn test_validate_item_changes_option_exceeds_max_modifier() {
        let changes = ItemChanges {
            price: None,
            quantity: None,
            manual_discount_percent: None,
            note: None,
            selected_options: Some(vec![shared::order::ItemOption {
                attribute_id: "attr:1".to_string(),
                attribute_name: "Size".to_string(),
                option_idx: 0,
                option_name: "Large".to_string(),
                price_modifier: Some(MAX_PRICE + 1.0),
            }]),
            selected_specification: None,
        };
        assert!(validate_item_changes(&changes).is_err(), "Option modifier > MAX_PRICE should be rejected");
    }

    // ========================================================================
    // sum_payments 隔离测试
    // ========================================================================

    #[test]
    fn test_sum_payments_empty() {
        assert_eq!(sum_payments(&[]), 0.0);
    }

    #[test]
    fn test_sum_payments_single() {
        let payments = vec![shared::order::PaymentRecord {
            payment_id: "p1".to_string(),
            method: "CASH".to_string(),
            amount: 25.50,
            tendered: None,
            change: None,
            note: None,
            cancelled: false,
            cancel_reason: None,
            split_items: None,
            aa_shares: None,
            split_type: None,
            timestamp: 1000,
        }];
        assert_eq!(sum_payments(&payments), 25.50);
    }

    #[test]
    fn test_sum_payments_with_cancelled() {
        let payments = vec![
            shared::order::PaymentRecord {
                payment_id: "p1".to_string(),
                method: "CASH".to_string(),
                amount: 30.0,
                tendered: None,
                change: None,
                note: None,
                cancelled: true, // cancelled — should be excluded
                cancel_reason: Some("wrong".to_string()),
                split_items: None,
                aa_shares: None,
                split_type: None,
                timestamp: 1000,
            },
            shared::order::PaymentRecord {
                payment_id: "p2".to_string(),
                method: "CARD".to_string(),
                amount: 15.0,
                tendered: None,
                change: None,
                note: None,
                cancelled: false,
                cancel_reason: None,
                split_items: None,
                aa_shares: None,
                split_type: None,
                timestamp: 2000,
            },
        ];
        assert_eq!(sum_payments(&payments), 15.0, "Cancelled payment should be excluded");
    }

    #[test]
    fn test_sum_payments_all_cancelled() {
        let payments = vec![
            shared::order::PaymentRecord {
                payment_id: "p1".to_string(),
                method: "CASH".to_string(),
                amount: 50.0,
                tendered: None,
                change: None,
                note: None,
                cancelled: true,
                cancel_reason: None,
                split_items: None,
                aa_shares: None,
                split_type: None,
                timestamp: 1000,
            },
        ];
        assert_eq!(sum_payments(&payments), 0.0, "All cancelled = 0");
    }

    #[test]
    fn test_sum_payments_precision() {
        // 10 payments of 0.1 each should sum to exactly 1.0
        let payments: Vec<shared::order::PaymentRecord> = (0..10)
            .map(|i| shared::order::PaymentRecord {
                payment_id: format!("p{}", i),
                method: "CASH".to_string(),
                amount: 0.1,
                tendered: None,
                change: None,
                note: None,
                cancelled: false,
                cancel_reason: None,
                split_items: None,
                aa_shares: None,
                split_type: None,
                timestamp: 1000 + i,
            })
            .collect();
        assert_eq!(sum_payments(&payments), 1.0, "0.1 * 10 = 1.0 with Decimal precision");
    }

    // ========================================================================
    // effective_rule_* helpers: skipped flag handling
    // ========================================================================

    use shared::models::price_rule::{AdjustmentType, ProductScope};
    use shared::order::AppliedRule;

    fn make_applied_rule(rule_id: &str, rule_type: RuleType, adjustment_value: f64, skipped: bool) -> AppliedRule {
        AppliedRule {
            rule_id: rule_id.to_string(),
            name: rule_id.to_string(),
            display_name: rule_id.to_string(),
            receipt_name: "R".to_string(),
            rule_type,
            adjustment_type: AdjustmentType::Percentage,
            product_scope: ProductScope::Global,
            zone_scope: "zone:all".to_string(),
            adjustment_value,
            calculated_amount: 0.0, // no longer authoritative; dynamically recalculated
            is_stackable: true,
            is_exclusive: false,
            skipped,
        }
    }

    fn make_item_with_rules(
        original_price: f64,
        rules: Vec<AppliedRule>,
        legacy_discount: Option<f64>,
        legacy_surcharge: Option<f64>,
    ) -> CartItemSnapshot {
        CartItemSnapshot {
            id: "p1".to_string(),
            instance_id: "i1".to_string(),
            name: "Item".to_string(),
            price: original_price,
            original_price: Some(original_price),
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: legacy_discount,
            rule_surcharge_amount: legacy_surcharge,
            applied_rules: Some(rules),
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

    #[test]
    fn test_effective_rule_discount_skipped_excluded() {
        // One active discount + one skipped discount → only active counts
        let item = make_item_with_rules(
            100.0,
            vec![
                make_applied_rule("r1", RuleType::Discount, 5.0, false),
                make_applied_rule("r2", RuleType::Discount, 3.0, true), // skipped
            ],
            Some(8.0), // legacy total (should be ignored when applied_rules present)
            None,
        );
        // basis = 100 (no manual discount), adjustment_value=5 → 100*5/100=5.0
        let eff = effective_rule_discount(&item, to_decimal(100.0));
        assert_eq!(to_f64(eff), 5.0, "Only non-skipped discount should count");
    }

    #[test]
    fn test_effective_rule_surcharge_skipped_excluded() {
        let item = make_item_with_rules(
            100.0,
            vec![
                make_applied_rule("s1", RuleType::Surcharge, 7.0, true), // skipped
                make_applied_rule("s2", RuleType::Surcharge, 4.0, false),
            ],
            None,
            Some(11.0), // legacy total (should be ignored)
        );
        // basis = 100 (base_with_options), adjustment_value=4 → 100*4/100=4.0
        let eff = effective_rule_surcharge(&item, to_decimal(100.0));
        assert_eq!(to_f64(eff), 4.0, "Only non-skipped surcharge should count");
    }

    #[test]
    fn test_effective_rule_discount_all_skipped() {
        let item = make_item_with_rules(
            100.0,
            vec![
                make_applied_rule("r1", RuleType::Discount, 5.0, true),
                make_applied_rule("r2", RuleType::Discount, 3.0, true),
            ],
            Some(8.0),
            None,
        );
        let eff = effective_rule_discount(&item, to_decimal(100.0));
        assert_eq!(eff, Decimal::ZERO, "All skipped → zero effective discount");
    }

    #[test]
    fn test_effective_rule_surcharge_all_skipped() {
        let item = make_item_with_rules(
            100.0,
            vec![
                make_applied_rule("s1", RuleType::Surcharge, 5.0, true),
            ],
            None,
            Some(5.0),
        );
        let eff = effective_rule_surcharge(&item, to_decimal(100.0));
        assert_eq!(eff, Decimal::ZERO, "All skipped → zero effective surcharge");
    }

    #[test]
    fn test_effective_rule_discount_empty_applied_rules() {
        // applied_rules is Some but empty vec → should return 0, NOT fall back to legacy
        let item = make_item_with_rules(100.0, vec![], Some(10.0), None);
        let eff = effective_rule_discount(&item, to_decimal(100.0));
        assert_eq!(eff, Decimal::ZERO, "Empty applied_rules → zero (not legacy fallback)");
    }

    #[test]
    fn test_effective_rule_discount_legacy_fallback() {
        // applied_rules is None → fall back to legacy rule_discount_amount
        let mut item = make_item_with_rules(100.0, vec![], None, None);
        item.applied_rules = None;
        item.rule_discount_amount = Some(7.5);
        let eff = effective_rule_discount(&item, to_decimal(100.0));
        assert_eq!(to_f64(eff), 7.5, "None applied_rules → use legacy field");
    }

    #[test]
    fn test_effective_rule_surcharge_legacy_fallback() {
        let mut item = make_item_with_rules(100.0, vec![], None, None);
        item.applied_rules = None;
        item.rule_surcharge_amount = Some(3.5);
        let eff = effective_rule_surcharge(&item, to_decimal(100.0));
        assert_eq!(to_f64(eff), 3.5, "None applied_rules → use legacy field");
    }

    #[test]
    fn test_effective_rule_discount_ignores_surcharge_rules() {
        // applied_rules has both discount and surcharge → only discount counted
        let item = make_item_with_rules(
            100.0,
            vec![
                make_applied_rule("r1", RuleType::Discount, 5.0, false),
                make_applied_rule("s1", RuleType::Surcharge, 10.0, false),
            ],
            None,
            None,
        );
        let eff = effective_rule_discount(&item, to_decimal(100.0));
        assert_eq!(to_f64(eff), 5.0, "Only Discount type rules counted");
    }

    #[test]
    fn test_effective_rule_surcharge_ignores_discount_rules() {
        let item = make_item_with_rules(
            100.0,
            vec![
                make_applied_rule("r1", RuleType::Discount, 5.0, false),
                make_applied_rule("s1", RuleType::Surcharge, 10.0, false),
            ],
            None,
            None,
        );
        let eff = effective_rule_surcharge(&item, to_decimal(100.0));
        assert_eq!(to_f64(eff), 10.0, "Only Surcharge type rules counted");
    }

    #[test]
    fn test_unit_price_with_skipped_discount_rule() {
        // Item: base 100, active discount 5, skipped discount 3 → unit_price = 95
        let item = make_item_with_rules(
            100.0,
            vec![
                make_applied_rule("r1", RuleType::Discount, 5.0, false),
                make_applied_rule("r2", RuleType::Discount, 3.0, true), // skipped
            ],
            Some(8.0),
            None,
        );
        let up = calculate_unit_price(&item);
        assert_eq!(to_f64(up), 95.0, "Skipped discount should not reduce price");
    }

    #[test]
    fn test_unit_price_with_skipped_surcharge_rule() {
        // Item: base 100, skipped surcharge 10 → unit_price = 100 (not 110)
        let item = make_item_with_rules(
            100.0,
            vec![
                make_applied_rule("s1", RuleType::Surcharge, 10.0, true),
            ],
            None,
            Some(10.0),
        );
        let up = calculate_unit_price(&item);
        assert_eq!(to_f64(up), 100.0, "Skipped surcharge should not increase price");
    }

    #[test]
    fn test_unit_price_comped_item_ignores_applied_rules() {
        // Comped item always returns 0 regardless of rules
        let mut item = make_item_with_rules(
            100.0,
            vec![make_applied_rule("r1", RuleType::Discount, 5.0, false)],
            Some(5.0),
            None,
        );
        item.is_comped = true;
        let up = calculate_unit_price(&item);
        assert_eq!(up, Decimal::ZERO, "Comped item always zero");
    }

    #[test]
    fn test_unit_price_manual_plus_rule_discount_combined() {
        // manual 60% + rule discount 50% → rule now based on after_manual
        let mut item = make_item_with_rules(
            100.0,
            vec![make_applied_rule("r1", RuleType::Discount, 50.0, false)],
            Some(50.0),
            None,
        );
        item.manual_discount_percent = Some(60.0);
        let up = calculate_unit_price(&item);
        // base = 100, manual = 60, after_manual = 40
        // rule_discount = 40 * 50% = 20
        // unit_price = 100 - 60 - 20 = 20
        assert_eq!(up, Decimal::from(20), "Rule discount based on after_manual price");
    }

    #[test]
    fn test_recalculate_totals_with_skipped_item_rule() {
        // Verify recalculate_totals produces correct results when item has skipped rules
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(make_item_with_rules(
            100.0,
            vec![
                make_applied_rule("r1", RuleType::Discount, 10.0, true), // skipped
                make_applied_rule("r2", RuleType::Discount, 5.0, false), // active
            ],
            Some(15.0),
            None,
        ));
        snapshot.items[0].quantity = 2;
        snapshot.items[0].unpaid_quantity = 2;

        recalculate_totals(&mut snapshot);

        // unit_price = 100 - 5 (only active discount) = 95
        assert_eq!(snapshot.items[0].unit_price, Some(95.0));
        // line_total = 95 * 2 = 190
        assert_eq!(snapshot.items[0].line_total, Some(190.0));
        // subtotal = 190
        assert_eq!(snapshot.subtotal, 190.0);
        assert_eq!(snapshot.total, 190.0);
        // original_total = 100 * 2 = 200
        assert_eq!(snapshot.original_total, 200.0);
        // total_discount should count only active rule: 5 * 2 = 10
        assert_eq!(snapshot.total_discount, 10.0);
    }

    #[test]
    fn test_recalculate_totals_skipped_order_level_discount() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        // Simple item, no item-level rules
        let mut item = make_item_with_rules(100.0, vec![], None, None);
        item.applied_rules = None;
        snapshot.items.push(item);

        // Order-level discount, skipped
        snapshot.order_applied_rules = Some(vec![
            make_applied_rule("or1", RuleType::Discount, 20.0, true), // skipped
        ]);
        snapshot.order_rule_discount_amount = Some(20.0); // legacy

        recalculate_totals(&mut snapshot);

        // Order discount skipped → total equals subtotal
        assert_eq!(snapshot.subtotal, 100.0);
        assert_eq!(snapshot.total, 100.0);
        assert_eq!(snapshot.discount, 0.0, "Skipped order discount → 0");
    }

    #[test]
    fn test_recalculate_totals_skipped_order_level_surcharge() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let mut item = make_item_with_rules(100.0, vec![], None, None);
        item.applied_rules = None;
        snapshot.items.push(item);

        snapshot.order_applied_rules = Some(vec![
            make_applied_rule("os1", RuleType::Surcharge, 15.0, true), // skipped
        ]);
        snapshot.order_rule_surcharge_amount = Some(15.0);

        recalculate_totals(&mut snapshot);

        assert_eq!(snapshot.subtotal, 100.0);
        assert_eq!(snapshot.total, 100.0, "Skipped order surcharge → no increase");
    }

    #[test]
    fn test_recalculate_totals_mixed_active_and_skipped_order_rules() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let mut item = make_item_with_rules(200.0, vec![], None, None);
        item.applied_rules = None;
        snapshot.items.push(item);

        snapshot.order_applied_rules = Some(vec![
            make_applied_rule("or1", RuleType::Discount, 10.0, false),  // active
            make_applied_rule("or2", RuleType::Discount, 20.0, true),   // skipped
            make_applied_rule("os1", RuleType::Surcharge, 5.0, false),  // active
            make_applied_rule("os2", RuleType::Surcharge, 8.0, true),   // skipped
        ]);

        recalculate_totals(&mut snapshot);

        // subtotal = 200
        assert_eq!(snapshot.subtotal, 200.0);
        // order_discount = 200*10/100=20 (only or1), order_surcharge = 200*5/100=10 (only os1)
        // total = 200 - 20 + 10 = 190
        assert_eq!(snapshot.total, 190.0);
        assert_eq!(snapshot.discount, 20.0);
    }

    #[test]
    fn test_recalculate_totals_pre_payment_reset_on_rule_skip() {
        // When a rule skip causes total to change, is_pre_payment must reset
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(make_item_with_rules(
            100.0,
            vec![make_applied_rule("r1", RuleType::Discount, 10.0, false)],
            Some(10.0),
            None,
        ));
        // First: calculate initial totals
        recalculate_totals(&mut snapshot);
        assert_eq!(snapshot.total, 90.0);

        // Set pre-payment flag
        snapshot.is_pre_payment = true;

        // Now "skip" the rule (simulating applier toggle)
        snapshot.items[0].applied_rules.as_mut().unwrap()[0].skipped = true;
        recalculate_totals(&mut snapshot);

        // Total changed from 90 to 100 → pre_payment should reset
        assert_eq!(snapshot.total, 100.0);
        assert!(!snapshot.is_pre_payment, "Pre-payment must reset when total changes from rule skip");
    }

    #[test]
    fn test_recalculate_totals_tax_on_item_with_skipped_surcharge() {
        // Tax should be calculated on the effective total (after skipping surcharge)
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let mut item = make_item_with_rules(
            100.0,
            vec![make_applied_rule("s1", RuleType::Surcharge, 10.0, true)], // skipped
            None,
            Some(10.0),
        );
        item.tax_rate = Some(21); // 21% IVA
        snapshot.items.push(item);

        recalculate_totals(&mut snapshot);

        // Surcharge skipped → unit_price = 100, line_total = 100
        assert_eq!(snapshot.items[0].unit_price, Some(100.0));
        // Tax: 100 * 21 / (100 + 21) = 100 * 21/121 ≈ 17.36
        assert_eq!(snapshot.items[0].tax, Some(17.36));
        assert_eq!(snapshot.tax, 17.36);
    }

    #[test]
    fn test_recalculate_totals_tax_on_item_with_active_surcharge() {
        // Compare: with active surcharge, tax is on higher amount
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let mut item = make_item_with_rules(
            100.0,
            vec![make_applied_rule("s1", RuleType::Surcharge, 10.0, false)], // active
            None,
            Some(10.0),
        );
        item.tax_rate = Some(21);
        snapshot.items.push(item);

        recalculate_totals(&mut snapshot);

        // Surcharge active → unit_price = 110, line_total = 110
        assert_eq!(snapshot.items[0].unit_price, Some(110.0));
        // Tax: 110 * 21 / 121 ≈ 19.09
        assert_eq!(snapshot.items[0].tax, Some(19.09));
    }

    #[test]
    fn test_recalculate_totals_skip_item_rule_changes_order_discount_basis() {
        // When item-level rule is skipped, subtotal changes, which affects
        // order-level percentage discount calculation
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(make_item_with_rules(
            100.0,
            vec![make_applied_rule("r1", RuleType::Discount, 20.0, false)], // active: -20
            Some(20.0),
            None,
        ));
        // Order-level manual percentage discount
        snapshot.order_manual_discount_percent = Some(10.0); // 10% of subtotal

        recalculate_totals(&mut snapshot);

        // subtotal = 80 (100-20), order_discount = 80 * 10% = 8, total = 72
        assert_eq!(snapshot.subtotal, 80.0);
        assert_eq!(snapshot.total, 72.0);

        // Now skip the item rule
        snapshot.items[0].applied_rules.as_mut().unwrap()[0].skipped = true;
        recalculate_totals(&mut snapshot);

        // subtotal = 100 (no item discount), order_discount = 100 * 10% = 10, total = 90
        assert_eq!(snapshot.subtotal, 100.0);
        assert_eq!(snapshot.total, 90.0);
    }

    #[test]
    fn test_unit_price_with_options_and_skipped_rule() {
        // Options modifier + skipped rule → options still apply, rule doesn't
        let mut item = make_item_with_rules(
            50.0,
            vec![make_applied_rule("r1", RuleType::Discount, 5.0, true)], // skipped
            Some(5.0),
            None,
        );
        item.selected_options = Some(vec![shared::order::ItemOption {
            attribute_id: "attr:size".to_string(),
            attribute_name: "Size".to_string(),
            option_idx: 1,
            option_name: "Large".to_string(),
            price_modifier: Some(3.0),
        }]);

        let up = calculate_unit_price(&item);
        // base = 50, options = 3 → base_with_options = 53
        // rule discount skipped → 0
        // unit_price = 53
        assert_eq!(to_f64(up), 53.0);
    }

    #[test]
    fn test_effective_rule_discount_multiple_active_stacked() {
        // 3 active discount rules → sum all calculated_amounts
        let item = make_item_with_rules(
            100.0,
            vec![
                make_applied_rule("r1", RuleType::Discount, 3.0, false),
                make_applied_rule("r2", RuleType::Discount, 4.0, false),
                make_applied_rule("r3", RuleType::Discount, 2.5, false),
            ],
            Some(9.5),
            None,
        );
        // basis=100, values=3+4+2.5 → 100*3/100 + 100*4/100 + 100*2.5/100 = 9.5
        let eff = effective_rule_discount(&item, to_decimal(100.0));
        assert_eq!(to_f64(eff), 9.5, "Sum of all active discounts");
    }

    #[test]
    fn test_skip_unskip_cycle_preserves_amounts() {
        // Skip a rule, then unskip it → amounts should return to original
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(make_item_with_rules(
            100.0,
            vec![make_applied_rule("r1", RuleType::Discount, 10.0, false)],
            Some(10.0),
            None,
        ));

        recalculate_totals(&mut snapshot);
        let original_total = snapshot.total;
        let original_subtotal = snapshot.subtotal;
        assert_eq!(original_total, 90.0);

        // Skip
        snapshot.items[0].applied_rules.as_mut().unwrap()[0].skipped = true;
        recalculate_totals(&mut snapshot);
        assert_eq!(snapshot.total, 100.0);

        // Unskip
        snapshot.items[0].applied_rules.as_mut().unwrap()[0].skipped = false;
        recalculate_totals(&mut snapshot);

        assert_eq!(snapshot.total, original_total, "Total restored after unskip");
        assert_eq!(snapshot.subtotal, original_subtotal, "Subtotal restored after unskip");
    }

    #[test]
    fn test_effective_order_rule_discount_skipped() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.order_applied_rules = Some(vec![
            make_applied_rule("or1", RuleType::Discount, 10.0, false),
            make_applied_rule("or2", RuleType::Discount, 5.0, true), // skipped
        ]);
        snapshot.order_rule_discount_amount = Some(15.0);

        // subtotal=100 as basis, adjustment_value=10 → 100*10/100=10.0
        let eff = effective_order_rule_discount(&snapshot, to_decimal(100.0));
        assert_eq!(to_f64(eff), 10.0, "Only active order discount counted");
    }

    #[test]
    fn test_effective_order_rule_surcharge_skipped() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.order_applied_rules = Some(vec![
            make_applied_rule("os1", RuleType::Surcharge, 8.0, true), // skipped
        ]);
        snapshot.order_rule_surcharge_amount = Some(8.0);

        let eff = effective_order_rule_surcharge(&snapshot, to_decimal(100.0));
        assert_eq!(eff, Decimal::ZERO, "Skipped order surcharge → zero");
    }

    #[test]
    fn test_effective_order_rule_discount_legacy_fallback() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.order_applied_rules = None;
        snapshot.order_rule_discount_amount = Some(12.0);

        let eff = effective_order_rule_discount(&snapshot, to_decimal(100.0));
        assert_eq!(to_f64(eff), 12.0, "None order_applied_rules → legacy fallback");
    }

    #[test]
    fn test_effective_order_rule_surcharge_legacy_fallback() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.order_applied_rules = None;
        snapshot.order_rule_surcharge_amount = Some(6.0);

        let eff = effective_order_rule_surcharge(&snapshot, to_decimal(100.0));
        assert_eq!(to_f64(eff), 6.0, "None order_applied_rules → legacy fallback");
    }

    #[test]
    fn test_recalculate_totals_remaining_amount_with_skipped_rule_and_payment() {
        // Verify remaining_amount is correct when a rule is skipped and partial payment exists
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(make_item_with_rules(
            100.0,
            vec![make_applied_rule("r1", RuleType::Discount, 20.0, false)],
            Some(20.0),
            None,
        ));
        snapshot.paid_amount = 50.0;

        recalculate_totals(&mut snapshot);
        // subtotal = 80, total = 80, remaining = 80 - 50 = 30
        assert_eq!(snapshot.total, 80.0);
        assert_eq!(snapshot.remaining_amount, 30.0);

        // Skip the discount → total goes up
        snapshot.items[0].applied_rules.as_mut().unwrap()[0].skipped = true;
        recalculate_totals(&mut snapshot);
        // subtotal = 100, total = 100, remaining = 100 - 50 = 50
        assert_eq!(snapshot.total, 100.0);
        assert_eq!(snapshot.remaining_amount, 50.0);
    }

    // ========================================================================
    // 动态重算新增测试
    // ========================================================================

    #[test]
    fn test_rule_discount_recalculates_after_manual_discount_change() {
        // Core bug scenario: 10% rule discount, manual changes 0→50%
        // Rule discount should recalculate based on after_manual price
        let mut item = make_item_with_rules(
            100.0,
            vec![make_applied_rule("r1", RuleType::Discount, 10.0, false)],
            None,
            None,
        );

        // No manual discount: rule_discount = 100 * 10% = 10, unit_price = 90
        let up1 = calculate_unit_price(&item);
        assert_eq!(to_f64(up1), 90.0);

        // Add 50% manual discount: after_manual = 50, rule_discount = 50 * 10% = 5
        // unit_price = 100 - 50 - 5 = 45
        item.manual_discount_percent = Some(50.0);
        let up2 = calculate_unit_price(&item);
        assert_eq!(to_f64(up2), 45.0, "Rule discount should recalculate based on after_manual");
    }

    #[test]
    fn test_rule_surcharge_uses_base_not_after_manual() {
        // Surcharge should be based on base_with_options, not after_manual
        let mut item = make_item_with_rules(
            100.0,
            vec![make_applied_rule("s1", RuleType::Surcharge, 10.0, false)],
            None,
            None,
        );
        item.manual_discount_percent = Some(50.0);

        let up = calculate_unit_price(&item);
        // base = 100, manual = 50, surcharge = 100 * 10% = 10 (based on base, not after_manual)
        // unit_price = 100 - 50 + 10 = 60
        assert_eq!(to_f64(up), 60.0, "Surcharge should use base_with_options, not after_manual");
    }

    #[test]
    fn test_fixed_amount_rule_unaffected_by_manual_discount() {
        // FixedAmount rule discount stays constant regardless of manual discount
        let mut item = make_item_with_rules(100.0, vec![], None, None);
        item.applied_rules = Some(vec![AppliedRule {
            rule_id: "r1".to_string(),
            name: "r1".to_string(),
            display_name: "r1".to_string(),
            receipt_name: "R".to_string(),
            rule_type: RuleType::Discount,
            adjustment_type: AdjustmentType::FixedAmount,
            product_scope: ProductScope::Global,
            zone_scope: "zone:all".to_string(),
            adjustment_value: 5.0,
            calculated_amount: 0.0,
            is_stackable: true,
            is_exclusive: false,
            skipped: false,
        }]);

        // No manual discount: unit_price = 100 - 5 = 95
        let up1 = calculate_unit_price(&item);
        assert_eq!(to_f64(up1), 95.0);

        // With 50% manual: unit_price = 100 - 50 - 5 = 45
        item.manual_discount_percent = Some(50.0);
        let up2 = calculate_unit_price(&item);
        assert_eq!(to_f64(up2), 45.0, "FixedAmount discount should be constant");
    }

    #[test]
    fn test_order_rule_recalculates_on_subtotal_change() {
        // Order-level 10% discount should recalculate when subtotal changes
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let mut item = make_item_with_rules(100.0, vec![], None, None);
        item.applied_rules = None;
        snapshot.items.push(item);

        snapshot.order_applied_rules = Some(vec![
            make_applied_rule("or1", RuleType::Discount, 10.0, false),
        ]);

        recalculate_totals(&mut snapshot);
        // subtotal = 100, order_discount = 100 * 10% = 10, total = 90
        assert_eq!(snapshot.total, 90.0);

        // Change item price → subtotal changes
        snapshot.items[0].price = 200.0;
        snapshot.items[0].original_price = Some(200.0);
        recalculate_totals(&mut snapshot);
        // subtotal = 200, order_discount = 200 * 10% = 20, total = 180
        assert_eq!(snapshot.total, 180.0, "Order rule discount should recalculate on subtotal change");
    }

    #[test]
    fn test_recalculate_updates_calculated_amount_in_snapshot() {
        // Verify that recalculate_totals syncs calculated_amount in applied_rules
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.items.push(make_item_with_rules(
            100.0,
            vec![make_applied_rule("r1", RuleType::Discount, 10.0, false)],
            None,
            None,
        ));

        recalculate_totals(&mut snapshot);

        // calculated_amount should be synced: 100 * 10% = 10.0
        let ca = snapshot.items[0].applied_rules.as_ref().unwrap()[0].calculated_amount;
        assert_eq!(ca, 10.0, "calculated_amount should be synced by recalculate_totals");

        // Change manual discount → after_manual changes → calculated_amount should update
        snapshot.items[0].manual_discount_percent = Some(50.0);
        recalculate_totals(&mut snapshot);

        // after_manual = 50, rule discount = 50 * 10% = 5.0
        let ca2 = snapshot.items[0].applied_rules.as_ref().unwrap()[0].calculated_amount;
        assert_eq!(ca2, 5.0, "calculated_amount should update after manual discount change");
    }

    #[test]
    fn test_recalculate_updates_order_calculated_amount_in_snapshot() {
        // Verify that recalculate_totals syncs calculated_amount in order_applied_rules
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let mut item = make_item_with_rules(100.0, vec![], None, None);
        item.applied_rules = None;
        snapshot.items.push(item);

        snapshot.order_applied_rules = Some(vec![
            make_applied_rule("or1", RuleType::Discount, 10.0, false),
        ]);

        recalculate_totals(&mut snapshot);
        let ca = snapshot.order_applied_rules.as_ref().unwrap()[0].calculated_amount;
        assert_eq!(ca, 10.0, "order calculated_amount should be 100*10%=10");

        // Change item price → subtotal changes
        snapshot.items[0].price = 200.0;
        snapshot.items[0].original_price = Some(200.0);
        recalculate_totals(&mut snapshot);
        let ca2 = snapshot.order_applied_rules.as_ref().unwrap()[0].calculated_amount;
        assert_eq!(ca2, 20.0, "order calculated_amount should update to 200*10%=20");
    }
}
