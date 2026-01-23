//! Price Calculator
//!
//! Logic for calculating price adjustments from matched rules.
//! Uses rust_decimal for precise calculations, stores as f64.

use crate::db::models::{AdjustmentType, PriceRule, RuleType};
use rust_decimal::prelude::*;

/// Rounding strategy for monetary values (2 decimal places, half-up)
const DECIMAL_PLACES: u32 = 2;

/// Convert f64 to Decimal for calculation
#[inline]
fn to_decimal(value: f64) -> Decimal {
    Decimal::from_f64(value).unwrap_or_default()
}

/// Convert Decimal back to f64 for storage, rounded to 2 decimal places
#[inline]
fn to_f64(value: Decimal) -> f64 {
    value
        .round_dp_with_strategy(DECIMAL_PLACES, RoundingStrategy::MidpointAwayFromZero)
        .to_f64()
        .unwrap_or_default()
}

/// Calculated adjustment result
#[derive(Debug, Clone, Default)]
pub struct PriceAdjustment {
    /// Surcharge amount (from SURCHARGE rules)
    pub surcharge: f64,
    /// Discount percentage (from DISCOUNT rules, percentage type)
    pub manual_discount_percent: f64,
    /// Discount fixed amount (from DISCOUNT rules, fixed type)
    pub discount_fixed: f64,
    /// Applied rule names for display
    pub applied_rules: Vec<String>,
}

impl PriceAdjustment {
    /// Calculate the final price after applying adjustments
    /// Order: base_price + surcharge - (surcharge * discount%) - fixed_discount
    ///
    /// Uses Decimal internally for precise calculations
    pub fn calculate_final_price(&self, base_price: f64) -> f64 {
        let base = to_decimal(base_price);
        let surcharge = to_decimal(self.surcharge);
        let discount_pct = to_decimal(self.manual_discount_percent);
        let discount_fixed = to_decimal(self.discount_fixed);

        // Step 1: Add surcharge
        let price_with_surcharge = base + surcharge;

        // Step 2: Apply percentage discount
        let discount_multiplier = Decimal::ONE - discount_pct / Decimal::ONE_HUNDRED;
        let after_percent_discount = price_with_surcharge * discount_multiplier;

        // Step 3: Apply fixed discount (never go below zero)
        let final_price = (after_percent_discount - discount_fixed).max(Decimal::ZERO);

        to_f64(final_price)
    }
}

/// Sort rules by priority (higher priority first)
pub fn sort_rules_by_priority(rules: &mut [&PriceRule]) {
    rules.sort_by(|a, b| b.priority.cmp(&a.priority));
}

/// Calculate adjustment from a single rule using Decimal precision
fn calculate_single_adjustment(rule: &PriceRule, base_price: Decimal) -> (Decimal, Decimal, Decimal) {
    let value = to_decimal(rule.adjustment_value);
    let hundred = Decimal::ONE_HUNDRED;

    match (&rule.rule_type, &rule.adjustment_type) {
        (RuleType::Surcharge, AdjustmentType::Percentage) => {
            // Surcharge as percentage of base price (e.g., value=10 means 10%)
            let amount = base_price * value / hundred;
            (amount, Decimal::ZERO, Decimal::ZERO)
        }
        (RuleType::Surcharge, AdjustmentType::FixedAmount) => {
            // Surcharge as fixed amount (e.g., value=5.00 means ¥5.00)
            (value, Decimal::ZERO, Decimal::ZERO)
        }
        (RuleType::Discount, AdjustmentType::Percentage) => {
            // Discount as percentage (store the percentage value, not the amount)
            (Decimal::ZERO, value, Decimal::ZERO)
        }
        (RuleType::Discount, AdjustmentType::FixedAmount) => {
            // Discount as fixed amount (e.g., value=5.00 means ¥5.00)
            (Decimal::ZERO, Decimal::ZERO, value)
        }
    }
}

/// Calculate total adjustment from matched rules
///
/// Rules are processed in priority order:
/// - Stackable rules accumulate their effects
/// - Non-stackable rules compete - highest priority wins per type
///
/// Returns: PriceAdjustment with surcharge, manual_discount_percent, discount_fixed
pub fn calculate_adjustments(rules: &[&PriceRule], base_price: f64) -> PriceAdjustment {
    if rules.is_empty() {
        return PriceAdjustment::default();
    }

    let base_decimal = to_decimal(base_price);

    // Sort by priority (already done by caller, but ensure)
    let mut sorted_rules: Vec<&PriceRule> = rules.to_vec();
    sort_rules_by_priority(&mut sorted_rules);

    // Accumulate using Decimal for precision
    let mut surcharge_acc = Decimal::ZERO;
    let mut discount_pct_acc = Decimal::ZERO;
    let mut discount_fixed_acc = Decimal::ZERO;
    let mut applied_rules = Vec::new();

    // Track if we've applied a non-stackable rule for each type
    let mut has_surcharge_non_stackable = false;
    let mut has_discount_non_stackable = false;

    for rule in sorted_rules {
        let is_surcharge = matches!(rule.rule_type, RuleType::Surcharge);
        let is_discount = matches!(rule.rule_type, RuleType::Discount);

        // Skip if non-stackable rule of this type already applied
        if is_surcharge && has_surcharge_non_stackable && !rule.is_stackable {
            continue;
        }
        if is_discount && has_discount_non_stackable && !rule.is_stackable {
            continue;
        }

        // Calculate adjustment using Decimal
        let (surcharge, discount_pct, discount_fixed) =
            calculate_single_adjustment(rule, base_decimal);

        // Apply adjustment
        if rule.is_stackable {
            surcharge_acc += surcharge;
            discount_pct_acc += discount_pct;
            discount_fixed_acc += discount_fixed;
        } else {
            // Non-stackable replaces
            if is_surcharge {
                surcharge_acc = surcharge;
                has_surcharge_non_stackable = true;
            }
            if is_discount {
                discount_pct_acc = discount_pct;
                discount_fixed_acc = discount_fixed;
                has_discount_non_stackable = true;
            }
        }

        applied_rules.push(rule.receipt_name.clone());
    }

    PriceAdjustment {
        surcharge: to_f64(surcharge_acc),
        manual_discount_percent: to_f64(discount_pct_acc),
        discount_fixed: to_f64(discount_fixed_acc),
        applied_rules,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{ProductScope, TimeMode};

    fn make_rule(
        rule_type: RuleType,
        adjustment_type: AdjustmentType,
        value: f64,
        priority: i32,
        stackable: bool,
    ) -> PriceRule {
        PriceRule {
            id: None,
            name: "test".to_string(),
            display_name: "Test".to_string(),
            receipt_name: "TEST".to_string(),
            description: None,
            rule_type,
            product_scope: ProductScope::Global,
            target: None,
            zone_scope: crate::db::models::ZONE_SCOPE_ALL.to_string(),
            adjustment_type,
            adjustment_value: value,
            priority,
            is_stackable: stackable,
            is_exclusive: false,
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

    #[test]
    fn test_percentage_discount() {
        let rule = make_rule(RuleType::Discount, AdjustmentType::Percentage, 10.0, 0, true);
        let rules: Vec<&PriceRule> = vec![&rule];
        let adj = calculate_adjustments(&rules, 100.0);

        assert_eq!(adj.manual_discount_percent, 10.0);
        assert_eq!(adj.calculate_final_price(100.0), 90.0);
    }

    #[test]
    fn test_fixed_discount() {
        // ¥5.00 fixed discount
        let rule = make_rule(
            RuleType::Discount,
            AdjustmentType::FixedAmount,
            5.0,
            0,
            true,
        );
        let rules: Vec<&PriceRule> = vec![&rule];
        let adj = calculate_adjustments(&rules, 100.0);

        assert_eq!(adj.discount_fixed, 5.0);
        assert_eq!(adj.calculate_final_price(100.0), 95.0);
    }

    #[test]
    fn test_percentage_surcharge() {
        let rule = make_rule(RuleType::Surcharge, AdjustmentType::Percentage, 10.0, 0, true);
        let rules: Vec<&PriceRule> = vec![&rule];
        let adj = calculate_adjustments(&rules, 100.0);

        assert_eq!(adj.surcharge, 10.0);
        assert_eq!(adj.calculate_final_price(100.0), 110.0);
    }

    #[test]
    fn test_stackable_rules() {
        let rule1 = make_rule(RuleType::Discount, AdjustmentType::Percentage, 10.0, 1, true);
        let rule2 = make_rule(RuleType::Discount, AdjustmentType::Percentage, 5.0, 0, true);
        let rules: Vec<&PriceRule> = vec![&rule1, &rule2];
        let adj = calculate_adjustments(&rules, 100.0);

        // 10% + 5% = 15% total discount
        assert_eq!(adj.manual_discount_percent, 15.0);
        assert_eq!(adj.calculate_final_price(100.0), 85.0);
    }

    #[test]
    fn test_non_stackable_wins() {
        let rule1 = make_rule(RuleType::Discount, AdjustmentType::Percentage, 10.0, 2, false); // Higher priority
        let rule2 = make_rule(RuleType::Discount, AdjustmentType::Percentage, 20.0, 1, false);
        let rules: Vec<&PriceRule> = vec![&rule1, &rule2];
        let adj = calculate_adjustments(&rules, 100.0);

        // Higher priority (10%) wins, lower priority (20%) ignored
        assert_eq!(adj.manual_discount_percent, 10.0);
    }

    #[test]
    fn test_combined_surcharge_and_discount() {
        // Surcharge 10% + Discount 20%
        // ¥100 + ¥10 (surcharge) = ¥110
        // ¥110 - 20% = ¥88
        let surcharge = make_rule(RuleType::Surcharge, AdjustmentType::Percentage, 10.0, 0, true);
        let discount = make_rule(RuleType::Discount, AdjustmentType::Percentage, 20.0, 0, true);
        let rules: Vec<&PriceRule> = vec![&surcharge, &discount];
        let adj = calculate_adjustments(&rules, 100.0);

        assert_eq!(adj.surcharge, 10.0);
        assert_eq!(adj.manual_discount_percent, 20.0);
        assert_eq!(adj.calculate_final_price(100.0), 88.0);
    }

    // ========== Precision tests ==========

    #[test]
    fn test_precision_third_discount() {
        // 33% discount on ¥100 should be ¥67.00
        let rule = make_rule(RuleType::Discount, AdjustmentType::Percentage, 33.0, 0, true);
        let rules: Vec<&PriceRule> = vec![&rule];
        let adj = calculate_adjustments(&rules, 100.0);

        assert_eq!(adj.manual_discount_percent, 33.0);
        // ¥100 * (1 - 0.33) = ¥67.00
        assert_eq!(adj.calculate_final_price(100.0), 67.0);
    }

    #[test]
    fn test_precision_small_amounts() {
        // ¥0.01 surcharge
        let rule = make_rule(RuleType::Surcharge, AdjustmentType::FixedAmount, 0.01, 0, true);
        let rules: Vec<&PriceRule> = vec![&rule];
        let adj = calculate_adjustments(&rules, 0.01);

        assert_eq!(adj.surcharge, 0.01);
        assert_eq!(adj.calculate_final_price(0.01), 0.02);
    }

    #[test]
    fn test_precision_many_stackable_rules() {
        // Stack 10 rules of 1% each = 10% total
        let rules_owned: Vec<PriceRule> = (0..10)
            .map(|i| make_rule(RuleType::Discount, AdjustmentType::Percentage, 1.0, i, true))
            .collect();
        let rules: Vec<&PriceRule> = rules_owned.iter().collect();
        let adj = calculate_adjustments(&rules, 100.0);

        assert_eq!(adj.manual_discount_percent, 10.0);
        assert_eq!(adj.calculate_final_price(100.0), 90.0);
    }

    #[test]
    fn test_precision_complex_calculation() {
        // ¥99.99 base
        // 10% surcharge = ¥9.999 → ¥109.989
        // 15% discount = ¥109.989 * 0.85 = ¥93.49065
        // ¥5.55 fixed discount = ¥87.94065 → ¥87.94
        let surcharge = make_rule(RuleType::Surcharge, AdjustmentType::Percentage, 10.0, 0, true);
        let discount_pct = make_rule(RuleType::Discount, AdjustmentType::Percentage, 15.0, 0, true);
        let discount_fixed = make_rule(RuleType::Discount, AdjustmentType::FixedAmount, 5.55, 0, true);
        let rules: Vec<&PriceRule> = vec![&surcharge, &discount_pct, &discount_fixed];
        let adj = calculate_adjustments(&rules, 99.99);

        let final_price = adj.calculate_final_price(99.99);
        assert_eq!(final_price, 87.94);
    }

    #[test]
    fn test_precision_rounding_edge_case() {
        // Test that 0.005 rounds up to 0.01 (half-up rounding)
        // ¥10.005 should become ¥10.01
        let rule = make_rule(RuleType::Surcharge, AdjustmentType::FixedAmount, 0.05, 0, true);
        let rules: Vec<&PriceRule> = vec![&rule];
        let adj = calculate_adjustments(&rules, 9.955);

        // 9.955 + 0.05 = 10.005 → rounds to 10.01
        let final_price = adj.calculate_final_price(9.955);
        assert_eq!(final_price, 10.01);
    }
}
