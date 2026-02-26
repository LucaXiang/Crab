//! MG Discount Calculator
//!
//! Calculates Marketing Group discounts for order items.
//! All matching rules are applied multiplicatively ("capitalist mode").
//! Completely independent from the pricing/ module.

use rust_decimal::prelude::*;
use shared::models::{AdjustmentType, MgDiscountRule, ProductScope};
use shared::order::AppliedMgRule;

use crate::order_money::{to_decimal, to_f64};

/// Result of MG discount calculation for a single item
pub struct MgCalculationResult {
    /// Total MG discount amount (f64, rounded to 2 decimal places)
    pub mg_discount: f64,
    /// List of applied MG rules with individual contributions
    pub applied_rules: Vec<AppliedMgRule>,
}

/// Calculate MG discount for a single item.
///
/// All matching active rules are applied multiplicatively:
/// - Percentage: running_price *= (1 - rate/100)
/// - FixedAmount: running_price -= amount (floor at 0)
///
/// # Arguments
/// * `base_price` - Price after PriceRule discounts (the unit_price from pricing engine)
/// * `product_id` - Product ID for scope matching
/// * `category_id` - Category ID for scope matching (from backend metadata cache)
/// * `rules` - MG discount rules for the member's marketing group
pub fn calculate_mg_discount(
    base_price: f64,
    product_id: i64,
    category_id: Option<i64>,
    rules: &[MgDiscountRule],
) -> MgCalculationResult {
    let hundred = Decimal::ONE_HUNDRED;
    let mut running_price = to_decimal(base_price);
    let original_price = running_price;
    let mut applied_rules = Vec::new();

    for rule in rules {
        if !rule.is_active {
            continue;
        }

        if !matches_product(rule, product_id, category_id) {
            continue;
        }

        let price_before = running_price;

        match rule.adjustment_type {
            AdjustmentType::Percentage => {
                let rate = to_decimal(rule.adjustment_value) / hundred;
                running_price *= Decimal::ONE - rate;
            }
            AdjustmentType::FixedAmount => {
                let amount = to_decimal(rule.adjustment_value);
                running_price = (running_price - amount).max(Decimal::ZERO);
            }
        }

        let contributed = price_before - running_price;

        applied_rules.push(AppliedMgRule {
            rule_id: rule.id,
            name: rule.name.clone(),
            receipt_name: rule.receipt_name.clone(),
            product_scope: rule.product_scope.clone(),
            adjustment_type: rule.adjustment_type.clone(),
            adjustment_value: rule.adjustment_value,
            calculated_amount: to_f64(contributed),
            skipped: false,
        });
    }

    let total_discount = original_price - running_price;

    MgCalculationResult {
        mg_discount: to_f64(total_discount),
        applied_rules,
    }
}

/// Check if a rule matches a product based on its scope
fn matches_product(rule: &MgDiscountRule, product_id: i64, category_id: Option<i64>) -> bool {
    match rule.product_scope {
        ProductScope::Global => true,
        ProductScope::Product => rule.target_id == Some(product_id),
        ProductScope::Category => rule.target_id == category_id,
        // Tag/Zone scope not used for MG rules
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::models::ProductScope;

    /// Helper to create a test MG discount rule
    fn make_mg_rule(
        id: i64,
        product_scope: ProductScope,
        target_id: Option<i64>,
        adjustment_type: AdjustmentType,
        adjustment_value: f64,
        is_active: bool,
    ) -> MgDiscountRule {
        MgDiscountRule {
            id,
            marketing_group_id: 1,
            name: format!("rule_{}", id),
            receipt_name: Some(format!("R{}", id)),
            product_scope,
            target_id,
            adjustment_type,
            adjustment_value,
            is_active,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn test_no_matching_rules() {
        // Product-scoped rule targeting a different product
        let rules = vec![make_mg_rule(
            1,
            ProductScope::Product,
            Some(999),
            AdjustmentType::Percentage,
            10.0,
            true,
        )];

        let result = calculate_mg_discount(100.0, 1, Some(10), &rules);

        assert_eq!(result.mg_discount, 0.0);
        assert!(result.applied_rules.is_empty());
    }

    #[test]
    fn test_single_percentage_discount() {
        // 10% discount on 100.0 = 10.0 discount
        let rules = vec![make_mg_rule(
            1,
            ProductScope::Global,
            None,
            AdjustmentType::Percentage,
            10.0,
            true,
        )];

        let result = calculate_mg_discount(100.0, 1, Some(10), &rules);

        assert_eq!(result.mg_discount, 10.0);
        assert_eq!(result.applied_rules.len(), 1);
        assert_eq!(result.applied_rules[0].calculated_amount, 10.0);
    }

    #[test]
    fn test_single_fixed_discount() {
        // Fixed 5.50 discount on 100.0 = 5.50 discount
        let rules = vec![make_mg_rule(
            1,
            ProductScope::Global,
            None,
            AdjustmentType::FixedAmount,
            5.5,
            true,
        )];

        let result = calculate_mg_discount(100.0, 1, Some(10), &rules);

        assert_eq!(result.mg_discount, 5.5);
        assert_eq!(result.applied_rules.len(), 1);
        assert_eq!(result.applied_rules[0].calculated_amount, 5.5);
    }

    #[test]
    fn test_stacking_multiple_percentage() {
        // Two 10% discounts stacked multiplicatively:
        // 100.0 * 0.9 * 0.9 = 81.0 -> discount = 19.0
        let rules = vec![
            make_mg_rule(
                1,
                ProductScope::Global,
                None,
                AdjustmentType::Percentage,
                10.0,
                true,
            ),
            make_mg_rule(
                2,
                ProductScope::Global,
                None,
                AdjustmentType::Percentage,
                10.0,
                true,
            ),
        ];

        let result = calculate_mg_discount(100.0, 1, Some(10), &rules);

        assert_eq!(result.mg_discount, 19.0);
        assert_eq!(result.applied_rules.len(), 2);
        // First rule: 100 * 10% = 10.0
        assert_eq!(result.applied_rules[0].calculated_amount, 10.0);
        // Second rule: 90 * 10% = 9.0
        assert_eq!(result.applied_rules[1].calculated_amount, 9.0);
    }

    #[test]
    fn test_mixed_percentage_and_fixed() {
        // 10% percentage then 5.0 fixed on 100.0:
        // 100.0 * 0.9 = 90.0, then 90.0 - 5.0 = 85.0
        // Total discount = 15.0
        let rules = vec![
            make_mg_rule(
                1,
                ProductScope::Global,
                None,
                AdjustmentType::Percentage,
                10.0,
                true,
            ),
            make_mg_rule(
                2,
                ProductScope::Global,
                None,
                AdjustmentType::FixedAmount,
                5.0,
                true,
            ),
        ];

        let result = calculate_mg_discount(100.0, 1, Some(10), &rules);

        assert_eq!(result.mg_discount, 15.0);
        assert_eq!(result.applied_rules.len(), 2);
        assert_eq!(result.applied_rules[0].calculated_amount, 10.0);
        assert_eq!(result.applied_rules[1].calculated_amount, 5.0);
    }

    #[test]
    fn test_scope_filtering_product() {
        // Only the rule targeting product_id=42 should match
        let rules = vec![
            make_mg_rule(
                1,
                ProductScope::Product,
                Some(42),
                AdjustmentType::Percentage,
                10.0,
                true,
            ),
            make_mg_rule(
                2,
                ProductScope::Product,
                Some(99),
                AdjustmentType::Percentage,
                20.0,
                true,
            ),
        ];

        let result = calculate_mg_discount(100.0, 42, Some(10), &rules);

        assert_eq!(result.mg_discount, 10.0);
        assert_eq!(result.applied_rules.len(), 1);
        assert_eq!(result.applied_rules[0].rule_id, 1);
    }

    #[test]
    fn test_scope_filtering_category() {
        // Only the rule targeting category_id=10 should match
        let rules = vec![
            make_mg_rule(
                1,
                ProductScope::Category,
                Some(10),
                AdjustmentType::Percentage,
                15.0,
                true,
            ),
            make_mg_rule(
                2,
                ProductScope::Category,
                Some(99),
                AdjustmentType::Percentage,
                20.0,
                true,
            ),
        ];

        let result = calculate_mg_discount(100.0, 42, Some(10), &rules);

        assert_eq!(result.mg_discount, 15.0);
        assert_eq!(result.applied_rules.len(), 1);
        assert_eq!(result.applied_rules[0].rule_id, 1);
    }

    #[test]
    fn test_discount_floor_at_zero() {
        // Fixed discount larger than price should floor at 0
        let rules = vec![make_mg_rule(
            1,
            ProductScope::Global,
            None,
            AdjustmentType::FixedAmount,
            150.0,
            true,
        )];

        let result = calculate_mg_discount(100.0, 1, Some(10), &rules);

        assert_eq!(result.mg_discount, 100.0);
        assert_eq!(result.applied_rules.len(), 1);
        assert_eq!(result.applied_rules[0].calculated_amount, 100.0);
    }

    #[test]
    fn test_inactive_rules_skipped() {
        let rules = vec![
            make_mg_rule(
                1,
                ProductScope::Global,
                None,
                AdjustmentType::Percentage,
                10.0,
                false, // inactive
            ),
            make_mg_rule(
                2,
                ProductScope::Global,
                None,
                AdjustmentType::Percentage,
                5.0,
                true, // active
            ),
        ];

        let result = calculate_mg_discount(100.0, 1, Some(10), &rules);

        // Only the 5% active rule should apply
        assert_eq!(result.mg_discount, 5.0);
        assert_eq!(result.applied_rules.len(), 1);
        assert_eq!(result.applied_rules[0].rule_id, 2);
    }
}
