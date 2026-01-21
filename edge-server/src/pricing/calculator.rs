//! Price Calculator
//!
//! Logic for calculating price adjustments from matched rules.

use crate::db::models::{AdjustmentType, PriceRule, RuleType};

/// Calculated adjustment result
#[derive(Debug, Clone, Default)]
pub struct PriceAdjustment {
    /// Surcharge amount (from SURCHARGE rules)
    pub surcharge: f64,
    /// Discount percentage (from DISCOUNT rules, percentage type)
    pub discount_percent: f64,
    /// Discount fixed amount (from DISCOUNT rules, fixed type)
    pub discount_fixed: f64,
    /// Applied rule names for display
    pub applied_rules: Vec<String>,
}

impl PriceAdjustment {
    /// Calculate the final price after applying adjustments
    /// Order: base_price + surcharge - (surcharge * discount%) - fixed_discount
    pub fn calculate_final_price(&self, base_price: f64) -> f64 {
        // Step 1: Add surcharge
        let price_with_surcharge = base_price + self.surcharge;

        // Step 2: Apply percentage discount
        let after_percent_discount = price_with_surcharge * (1.0 - self.discount_percent / 100.0);

        // Step 3: Apply fixed discount
        let final_price = (after_percent_discount - self.discount_fixed).max(0.0);

        // Round to 2 decimal places
        (final_price * 100.0).round() / 100.0
    }
}

/// Sort rules by priority (higher priority first)
pub fn sort_rules_by_priority(rules: &mut [&PriceRule]) {
    rules.sort_by(|a, b| b.priority.cmp(&a.priority));
}

/// Calculate adjustment from a single rule
fn calculate_single_adjustment(rule: &PriceRule, base_price: f64) -> (f64, f64, f64) {
    let value = rule.adjustment_value as f64;

    match (&rule.rule_type, &rule.adjustment_type) {
        (RuleType::Surcharge, AdjustmentType::Percentage) => {
            // Surcharge as percentage of base price
            let amount = base_price * value / 100.0;
            (amount, 0.0, 0.0)
        }
        (RuleType::Surcharge, AdjustmentType::FixedAmount) => {
            // Surcharge as fixed amount (value is in cents, convert to dollars)
            let amount = value / 100.0;
            (amount, 0.0, 0.0)
        }
        (RuleType::Discount, AdjustmentType::Percentage) => {
            // Discount as percentage
            (0.0, value, 0.0)
        }
        (RuleType::Discount, AdjustmentType::FixedAmount) => {
            // Discount as fixed amount (value is in cents, convert to dollars)
            let amount = value / 100.0;
            (0.0, 0.0, amount)
        }
    }
}

/// Calculate total adjustment from matched rules
///
/// Rules are processed in priority order:
/// - Stackable rules accumulate their effects
/// - Non-stackable rules compete - highest priority wins per type
///
/// Returns: (surcharge, discount_percent, discount_fixed, applied_rule_names)
pub fn calculate_adjustments(rules: &[&PriceRule], base_price: f64) -> PriceAdjustment {
    let mut result = PriceAdjustment::default();

    if rules.is_empty() {
        return result;
    }

    // Sort by priority (already done by caller, but ensure)
    let mut sorted_rules: Vec<&PriceRule> = rules.to_vec();
    sort_rules_by_priority(&mut sorted_rules);

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

        // Calculate adjustment
        let (surcharge, discount_pct, discount_fixed) =
            calculate_single_adjustment(rule, base_price);

        // Apply adjustment
        if rule.is_stackable {
            result.surcharge += surcharge;
            result.discount_percent += discount_pct;
            result.discount_fixed += discount_fixed;
        } else {
            // Non-stackable replaces
            if is_surcharge {
                result.surcharge = surcharge;
                has_surcharge_non_stackable = true;
            }
            if is_discount {
                result.discount_percent = discount_pct;
                result.discount_fixed = discount_fixed;
                has_discount_non_stackable = true;
            }
        }

        result.applied_rules.push(rule.receipt_name.clone());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{ProductScope, TimeMode};

    fn make_rule(
        rule_type: RuleType,
        adjustment_type: AdjustmentType,
        value: i32,
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
            zone_scope: -1,
            adjustment_type,
            adjustment_value: value,
            priority,
            is_stackable: stackable,
            time_mode: TimeMode::Always,
            start_time: None,
            end_time: None,
            schedule_config: None,
            is_active: true,
            created_by: None,
        }
    }

    #[test]
    fn test_percentage_discount() {
        let rule = make_rule(RuleType::Discount, AdjustmentType::Percentage, 10, 0, true);
        let rules: Vec<&PriceRule> = vec![&rule];
        let adj = calculate_adjustments(&rules, 100.0);

        assert_eq!(adj.discount_percent, 10.0);
        assert_eq!(adj.calculate_final_price(100.0), 90.0);
    }

    #[test]
    fn test_fixed_discount() {
        // 500 cents = $5
        let rule = make_rule(
            RuleType::Discount,
            AdjustmentType::FixedAmount,
            500,
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
        let rule = make_rule(RuleType::Surcharge, AdjustmentType::Percentage, 10, 0, true);
        let rules: Vec<&PriceRule> = vec![&rule];
        let adj = calculate_adjustments(&rules, 100.0);

        assert_eq!(adj.surcharge, 10.0);
        assert_eq!(adj.calculate_final_price(100.0), 110.0);
    }

    #[test]
    fn test_stackable_rules() {
        let rule1 = make_rule(RuleType::Discount, AdjustmentType::Percentage, 10, 1, true);
        let rule2 = make_rule(RuleType::Discount, AdjustmentType::Percentage, 5, 0, true);
        let rules: Vec<&PriceRule> = vec![&rule1, &rule2];
        let adj = calculate_adjustments(&rules, 100.0);

        // 10% + 5% = 15% total discount
        assert_eq!(adj.discount_percent, 15.0);
        assert_eq!(adj.calculate_final_price(100.0), 85.0);
    }

    #[test]
    fn test_non_stackable_wins() {
        let rule1 = make_rule(RuleType::Discount, AdjustmentType::Percentage, 10, 2, false); // Higher priority
        let rule2 = make_rule(RuleType::Discount, AdjustmentType::Percentage, 20, 1, false);
        let rules: Vec<&PriceRule> = vec![&rule1, &rule2];
        let adj = calculate_adjustments(&rules, 100.0);

        // Higher priority (10%) wins, lower priority (20%) ignored
        assert_eq!(adj.discount_percent, 10.0);
    }

    #[test]
    fn test_combined_surcharge_and_discount() {
        // Surcharge 10% + Discount 20%
        // $100 + $10 (surcharge) = $110
        // $110 - 20% = $88
        let surcharge = make_rule(RuleType::Surcharge, AdjustmentType::Percentage, 10, 0, true);
        let discount = make_rule(RuleType::Discount, AdjustmentType::Percentage, 20, 0, true);
        let rules: Vec<&PriceRule> = vec![&surcharge, &discount];
        let adj = calculate_adjustments(&rules, 100.0);

        assert_eq!(adj.surcharge, 10.0);
        assert_eq!(adj.discount_percent, 20.0);
        assert_eq!(adj.calculate_final_price(100.0), 88.0);
    }
}
