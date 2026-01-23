//! Order-Level Price Calculator
//!
//! Calculate order-level prices with support for:
//! - Order-level rule discounts (exclusive, non-stackable, stackable)
//! - Order-level rule surcharges (same stacking logic)
//! - Manual order discounts (percentage or fixed amount)
//!
//! Uses functions from item_calculator for rule application logic.

use crate::db::models::PriceRule;
use rust_decimal::prelude::*;
use shared::order::AppliedRule;

use super::item_calculator::{apply_discount_rules, apply_surcharge_rules, to_decimal, to_f64};

/// Result of order price calculation
#[derive(Debug, Clone)]
pub struct OrderCalculationResult {
    /// Sum of all item totals (subtotal)
    pub subtotal: f64,
    /// Total discount amount from order-level rules
    pub order_rule_discount_amount: f64,
    /// Price after order rule discounts
    pub after_order_rule_discount: f64,
    /// Total surcharge amount from order-level rules
    pub order_rule_surcharge_amount: f64,
    /// Price after order rules (discounts and surcharges)
    pub after_order_rule: f64,
    /// Manual discount amount (either from percent or fixed)
    pub order_manual_discount_amount: f64,
    /// Final order total
    pub total: f64,
    /// Applied order-level rules (for tracking/display)
    pub order_applied_rules: Vec<AppliedRule>,
}

impl Default for OrderCalculationResult {
    fn default() -> Self {
        Self {
            subtotal: 0.0,
            order_rule_discount_amount: 0.0,
            after_order_rule_discount: 0.0,
            order_rule_surcharge_amount: 0.0,
            after_order_rule: 0.0,
            order_manual_discount_amount: 0.0,
            total: 0.0,
            order_applied_rules: vec![],
        }
    }
}

/// Calculate order-level price with rules and manual discount
///
/// # Arguments
/// * `subtotal` - Sum of all item final prices
/// * `order_rules` - Rules that apply at order level
/// * `manual_discount_percent` - Optional manual discount percentage (0-100)
/// * `manual_discount_fixed` - Optional manual fixed discount amount
///
/// # Calculation Steps
/// 1. Apply order rule discounts (based on subtotal)
/// 2. Apply order rule surcharges (based on subtotal)
/// 3. Apply manual discount (either percent or fixed, based on after_order_rule)
///
/// # Notes
/// - If both manual_discount_percent and manual_discount_fixed are provided,
///   only percentage is applied (percent takes precedence)
/// - Manual discount is applied AFTER rule discounts/surcharges
///
/// # Returns
/// `OrderCalculationResult` with all intermediate values and applied rules
pub fn calculate_order_price(
    subtotal: f64,
    order_rules: &[&PriceRule],
    manual_discount_percent: Option<f64>,
    manual_discount_fixed: Option<f64>,
) -> OrderCalculationResult {
    let subtotal_decimal = to_decimal(subtotal);

    // Step 1: Apply order rule discounts (based on subtotal)
    let discount_result = apply_discount_rules(order_rules, subtotal_decimal);
    let after_discount = (subtotal_decimal - discount_result.amount).max(Decimal::ZERO);

    // Step 2: Apply order rule surcharges (based on subtotal)
    let surcharge_result = apply_surcharge_rules(order_rules, subtotal_decimal);
    let after_order_rule = (after_discount + surcharge_result.amount).max(Decimal::ZERO);

    // Step 3: Apply manual discount (based on after_order_rule)
    let hundred = Decimal::ONE_HUNDRED;
    let manual_discount_amount = if let Some(percent) = manual_discount_percent {
        // Percentage takes precedence
        let pct = to_decimal(percent);
        after_order_rule * pct / hundred
    } else if let Some(fixed) = manual_discount_fixed {
        // Fixed amount (already in dollars)
        to_decimal(fixed)
    } else {
        Decimal::ZERO
    };

    let total = (after_order_rule - manual_discount_amount).max(Decimal::ZERO);

    // Combine applied rules
    let mut order_applied_rules = discount_result.applied;
    order_applied_rules.extend(surcharge_result.applied);

    OrderCalculationResult {
        subtotal: to_f64(subtotal_decimal),
        order_rule_discount_amount: to_f64(discount_result.amount),
        after_order_rule_discount: to_f64(after_discount),
        order_rule_surcharge_amount: to_f64(surcharge_result.amount),
        after_order_rule: to_f64(after_order_rule),
        order_manual_discount_amount: to_f64(manual_discount_amount),
        total: to_f64(total),
        order_applied_rules,
    }
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{AdjustmentType, ProductScope, RuleType, TimeMode};

    /// Helper to create a test rule
    fn make_rule(
        rule_type: RuleType,
        adjustment_type: AdjustmentType,
        value: f64,
        priority: i32,
        stackable: bool,
        exclusive: bool,
    ) -> PriceRule {
        PriceRule {
            id: None,
            name: format!("rule_{}", value),
            display_name: format!("Rule {}", value),
            receipt_name: format!("R{}", value),
            description: None,
            rule_type,
            product_scope: ProductScope::Global,
            target: None,
            zone_scope: crate::db::models::ZONE_SCOPE_ALL.to_string(),
            adjustment_type,
            adjustment_value: value,
            priority,
            is_stackable: stackable,
            is_exclusive: exclusive,
            time_mode: TimeMode::Always,
            start_time: None,
            end_time: None,
            schedule_config: None,
            valid_from: None,
            valid_until: None,
            active_days: None,
            active_start_time: None,
            active_end_time: None,
            is_active: true,
            created_by: None,
            created_at: 0,
        }
    }

    // ==================== Order Discount Tests ====================

    #[test]
    fn test_order_discount() {
        // $100 subtotal with 10% order discount = $10 discount, $90 final
        let rule = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            10.0,
            0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&rule];

        let result = calculate_order_price(100.0, &rules, None, None);

        assert_eq!(result.subtotal, 100.0);
        assert_eq!(result.order_rule_discount_amount, 10.0);
        assert_eq!(result.after_order_rule_discount, 90.0);
        assert_eq!(result.order_rule_surcharge_amount, 0.0);
        assert_eq!(result.after_order_rule, 90.0);
        assert_eq!(result.order_manual_discount_amount, 0.0);
        assert_eq!(result.total, 90.0);
        assert_eq!(result.order_applied_rules.len(), 1);
    }

    // ==================== Manual Discount Tests ====================

    #[test]
    fn test_order_manual_discount_percent() {
        // $100 subtotal with 15% manual discount = $15 discount, $85 final
        let rules: Vec<&PriceRule> = vec![];

        let result = calculate_order_price(100.0, &rules, Some(15.0), None);

        assert_eq!(result.subtotal, 100.0);
        assert_eq!(result.order_rule_discount_amount, 0.0);
        assert_eq!(result.after_order_rule_discount, 100.0);
        assert_eq!(result.after_order_rule, 100.0);
        assert_eq!(result.order_manual_discount_amount, 15.0);
        assert_eq!(result.total, 85.0);
    }

    #[test]
    fn test_order_manual_discount_fixed() {
        // $100 subtotal with $20 fixed manual discount = $80 final
        let rules: Vec<&PriceRule> = vec![];

        let result = calculate_order_price(100.0, &rules, None, Some(20.0));

        assert_eq!(result.subtotal, 100.0);
        assert_eq!(result.order_rule_discount_amount, 0.0);
        assert_eq!(result.after_order_rule, 100.0);
        assert_eq!(result.order_manual_discount_amount, 20.0);
        assert_eq!(result.total, 80.0);
    }

    #[test]
    fn test_manual_discount_percent_takes_precedence() {
        // If both percent and fixed are provided, percent takes precedence
        // $100 subtotal, 10% percent vs $50 fixed -> 10% wins = $10 discount
        let rules: Vec<&PriceRule> = vec![];

        let result = calculate_order_price(100.0, &rules, Some(10.0), Some(50.0));

        assert_eq!(result.order_manual_discount_amount, 10.0);
        assert_eq!(result.total, 90.0);
    }

    // ==================== Combined Rule and Manual Tests ====================

    #[test]
    fn test_order_combined_rule_and_manual() {
        // $100 subtotal
        // 10% rule discount -> $10 discount -> $90
        // 5% rule surcharge on subtotal -> $5 -> $95
        // 10% manual discount on $95 -> $9.50 -> $85.50
        let discount = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            10.0,
            0,
            true,
            false,
        );
        let surcharge = make_rule(
            RuleType::Surcharge,
            AdjustmentType::Percentage,
            5.0,
            0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&discount, &surcharge];

        let result = calculate_order_price(100.0, &rules, Some(10.0), None);

        assert_eq!(result.subtotal, 100.0);
        assert_eq!(result.order_rule_discount_amount, 10.0);
        assert_eq!(result.after_order_rule_discount, 90.0);
        assert_eq!(result.order_rule_surcharge_amount, 5.0);
        assert_eq!(result.after_order_rule, 95.0);
        assert_eq!(result.order_manual_discount_amount, 9.5);
        assert_eq!(result.total, 85.5);
        assert_eq!(result.order_applied_rules.len(), 2);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_no_rules_no_manual() {
        let rules: Vec<&PriceRule> = vec![];
        let result = calculate_order_price(100.0, &rules, None, None);

        assert_eq!(result.subtotal, 100.0);
        assert_eq!(result.total, 100.0);
        assert!(result.order_applied_rules.is_empty());
    }

    #[test]
    fn test_discount_cannot_go_negative() {
        // 150% discount should result in $0, not negative
        let rule = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            150.0,
            0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&rule];

        let result = calculate_order_price(100.0, &rules, None, None);

        assert_eq!(result.after_order_rule_discount, 0.0);
        assert_eq!(result.total, 0.0);
    }

    #[test]
    fn test_manual_discount_cannot_exceed_total() {
        // $100 subtotal with $150 fixed manual discount should result in $0
        let rules: Vec<&PriceRule> = vec![];

        let result = calculate_order_price(100.0, &rules, None, Some(150.0));

        assert_eq!(result.total, 0.0);
    }

    #[test]
    fn test_order_with_surcharge_only() {
        // $100 subtotal with 8% surcharge = $8 surcharge, $108 final
        let rule = make_rule(
            RuleType::Surcharge,
            AdjustmentType::Percentage,
            8.0,
            0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&rule];

        let result = calculate_order_price(100.0, &rules, None, None);

        assert_eq!(result.subtotal, 100.0);
        assert_eq!(result.order_rule_discount_amount, 0.0);
        assert_eq!(result.after_order_rule_discount, 100.0);
        assert_eq!(result.order_rule_surcharge_amount, 8.0);
        assert_eq!(result.after_order_rule, 108.0);
        assert_eq!(result.total, 108.0);
    }

    #[test]
    fn test_fixed_discount_rule() {
        // $100 subtotal with $15 fixed discount = $85 final
        let rule = make_rule(
            RuleType::Discount,
            AdjustmentType::FixedAmount,
            15.0,
            0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&rule];

        let result = calculate_order_price(100.0, &rules, None, None);

        assert_eq!(result.order_rule_discount_amount, 15.0);
        assert_eq!(result.total, 85.0);
    }
}
