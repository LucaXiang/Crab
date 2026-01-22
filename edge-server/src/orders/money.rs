//! Money calculation utilities using rust_decimal for precision
//!
//! This module provides precise decimal arithmetic for monetary calculations.
//! All calculations are done using `Decimal` internally, then converted to `f64`
//! for storage/serialization.

use rust_decimal::prelude::*;
use shared::order::{CartItemSnapshot, OrderSnapshot};

/// Rounding strategy for monetary values (2 decimal places, half-up)
const DECIMAL_PLACES: u32 = 2;

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

/// Calculate item line total with precise decimal arithmetic
///
/// Formula: (price * quantity) * (1 - discount_percent/100)
pub fn calculate_item_total(item: &CartItemSnapshot) -> Decimal {
    let price = to_decimal(item.price);
    let quantity = Decimal::from(item.quantity);
    let discount_rate = item
        .discount_percent
        .map(|d| to_decimal(d) / Decimal::ONE_HUNDRED)
        .unwrap_or(Decimal::ZERO);

    let base_total = price * quantity;
    let discounted = base_total * (Decimal::ONE - discount_rate);

    discounted.round_dp_with_strategy(DECIMAL_PLACES, RoundingStrategy::MidpointAwayFromZero)
}

/// Recalculate order totals from items using precise decimal arithmetic
///
/// This function:
/// 1. Calculates subtotal from all items
/// 2. Updates unpaid_quantity for each item based on paid_item_quantities
/// 3. Calculates total = subtotal + tax - discount
pub fn recalculate_totals(snapshot: &mut OrderSnapshot) {
    let mut subtotal = Decimal::ZERO;

    for item in &mut snapshot.items {
        // Update unpaid_quantity
        let paid_qty = snapshot
            .paid_item_quantities
            .get(&item.instance_id)
            .copied()
            .unwrap_or(0);
        item.unpaid_quantity = (item.quantity - paid_qty).max(0);

        // Accumulate subtotal
        subtotal += calculate_item_total(item);
    }

    let tax = to_decimal(snapshot.tax);
    let discount = to_decimal(snapshot.discount);
    let total = subtotal + tax - discount;

    snapshot.subtotal = to_f64(subtotal);
    snapshot.total = to_f64(total);
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
    let tolerance = Decimal::new(1, 2); // 0.01

    paid_dec >= required_dec - tolerance
}

/// Compare two monetary values for equality (within 0.01 tolerance)
pub fn money_eq(a: f64, b: f64) -> bool {
    let diff = (to_decimal(a) - to_decimal(b)).abs();
    diff < Decimal::new(1, 2) // 0.01
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
            discount_percent: None,
            surcharge: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
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
            discount_percent: Some(33.33), // Tricky percentage
            surcharge: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
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
            discount_percent: Some(33.0),
            surcharge: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
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
                discount_percent: None,
                surcharge: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            })
            .collect();

        let total: Decimal = items.iter().map(|i| calculate_item_total(i)).sum();
        assert_eq!(to_f64(total), 1.0);
    }
}
