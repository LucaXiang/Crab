//! Item Price Calculator
//!
//! Calculate item-level prices with support for:
//! - Manual discounts (percentage-based)
//! - Rule-based discounts (exclusive, non-stackable, stackable)
//! - Rule-based surcharges (same stacking logic)
//!
//! Uses rust_decimal for precision calculations.

use rust_decimal::prelude::*;
use shared::models::{AdjustmentType, PriceRule, ProductScope, RuleType};
use shared::order::AppliedRule;
use tracing::{debug, trace};

/// Rounding strategy for monetary values (2 decimal places, half-up)
const DECIMAL_PLACES: u32 = 2;

/// Result of item price calculation
#[derive(Debug, Clone)]
pub struct ItemCalculationResult {
    /// Base price (original_price + options_modifier)
    pub base: f64,
    /// Manual discount amount
    pub manual_discount_amount: f64,
    /// Price after manual discount
    pub after_manual: f64,
    /// Rule discount amount
    pub rule_discount_amount: f64,
    /// Price after rule discounts
    pub after_discount: f64,
    /// Rule surcharge amount
    pub rule_surcharge_amount: f64,
    /// Final item price
    pub item_final: f64,
    /// Applied rules (for tracking/display)
    pub applied_rules: Vec<AppliedRule>,
}

impl Default for ItemCalculationResult {
    fn default() -> Self {
        Self {
            base: 0.0,
            manual_discount_amount: 0.0,
            after_manual: 0.0,
            rule_discount_amount: 0.0,
            after_discount: 0.0,
            rule_surcharge_amount: 0.0,
            item_final: 0.0,
            applied_rules: vec![],
        }
    }
}

// ==================== Conversion Helpers ====================

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

// ==================== Priority Calculation ====================

/// Calculate effective priority for a rule
///
/// Priority formula: zone_weight * 10 + product_weight
///
/// This ensures more specific rules (specific zone, specific product) have higher priority
/// than general rules (global zone, global product scope).
///
/// Zone weights:
/// - zone:all: 0
/// - all others (including zone:retail): 1
///
/// Product weights:
/// - Global: 0
/// - Category: 1
/// - Tag: 2
/// - Product: 3
pub fn calculate_effective_priority(rule: &PriceRule) -> i32 {
    let zone_weight = match rule.zone_scope.as_str() {
        shared::models::ZONE_SCOPE_ALL => 0,
        _ => 1,
    };

    let product_weight = match rule.product_scope {
        ProductScope::Global => 0,
        ProductScope::Category => 1,
        ProductScope::Tag => 2,
        ProductScope::Product => 3,
    };

    zone_weight * 10 + product_weight
}

// ==================== Rule Selection ====================

/// Select the winning rule based on effective priority (highest wins).
/// If tied, prefer the rule created more recently (created_at DESC).
fn select_winner<'a>(rules: &[&'a PriceRule]) -> Option<&'a PriceRule> {
    if rules.is_empty() {
        return None;
    }

    rules
        .iter()
        .max_by(|a, b| {
            let priority_a = calculate_effective_priority(a);
            let priority_b = calculate_effective_priority(b);

            match priority_a.cmp(&priority_b) {
                std::cmp::Ordering::Equal => a.created_at.cmp(&b.created_at), // Higher created_at wins
                other => other,
            }
        })
        .copied()
}

// ==================== Discount Application ====================

/// Result of applying discount rules
pub struct DiscountResult {
    /// Total discount amount
    pub amount: Decimal,
    /// Rules that were applied
    pub applied: Vec<AppliedRule>,
}

/// Apply discount rules with stacking logic:
/// 1. Exclusive rules: If any exist, only the highest priority exclusive applies
/// 2. Non-stackable: Winner takes all, but can stack with stackable rules
/// 3. Stackable percentage: Multiply (1-rate1) * (1-rate2) for "capitalist mode"
/// 4. Stackable fixed: Simple addition
///
/// All discounts are calculated based on `price_basis`.
pub fn apply_discount_rules(rules: &[&PriceRule], price_basis: Decimal) -> DiscountResult {
    let discount_rules: Vec<&PriceRule> = rules
        .iter()
        .filter(|r| matches!(r.rule_type, RuleType::Discount))
        .copied()
        .collect();

    debug!(
        target: "pricing",
        total_rules = rules.len(),
        discount_rules_count = discount_rules.len(),
        price_basis = %price_basis,
        "Starting discount rule application"
    );

    if discount_rules.is_empty() {
        debug!(target: "pricing", "No discount rules to apply");
        return DiscountResult {
            amount: Decimal::ZERO,
            applied: vec![],
        };
    }

    let mut applied_rules = Vec::new();
    let hundred = Decimal::ONE_HUNDRED;

    // Separate rules by stacking behavior
    let exclusive: Vec<&PriceRule> = discount_rules
        .iter()
        .filter(|r| r.is_exclusive)
        .copied()
        .collect();

    let non_stackable: Vec<&PriceRule> = discount_rules
        .iter()
        .filter(|r| !r.is_exclusive && !r.is_stackable)
        .copied()
        .collect();

    let stackable: Vec<&PriceRule> = discount_rules
        .iter()
        .filter(|r| !r.is_exclusive && r.is_stackable)
        .copied()
        .collect();

    debug!(
        exclusive_count = exclusive.len(),
        non_stackable_count = non_stackable.len(),
        stackable_count = stackable.len(),
        "[DiscountRules] Rules categorized"
    );

    // Step 1: Check for exclusive rules
    if let Some(winner) = select_winner(&exclusive) {
        let amount = calculate_single_discount(winner, price_basis);
        debug!(
            winner_name = %winner.name,
            winner_value = winner.adjustment_value,
            winner_type = ?winner.adjustment_type,
            discount_amount = %amount,
            "[DiscountRules] Exclusive winner selected"
        );
        let applied = AppliedRule::from_rule(winner, to_f64(amount));
        return DiscountResult {
            amount,
            applied: vec![applied],
        };
    }

    let mut total_discount = Decimal::ZERO;

    // Step 2: Apply non-stackable winner (if any)
    if let Some(winner) = select_winner(&non_stackable) {
        let amount = calculate_single_discount(winner, price_basis);
        debug!(
            winner_name = %winner.name,
            winner_value = winner.adjustment_value,
            winner_type = ?winner.adjustment_type,
            discount_amount = %amount,
            "[DiscountRules] Non-stackable winner selected"
        );
        total_discount += amount;
        applied_rules.push(AppliedRule::from_rule(winner, to_f64(amount)));
    }

    // Step 3: Apply stackable rules
    // Percentage stackable: use "capitalist mode" (1 - rate1) * (1 - rate2)
    // Fixed stackable: simple addition
    let stackable_pct: Vec<&PriceRule> = stackable
        .iter()
        .filter(|r| matches!(r.adjustment_type, AdjustmentType::Percentage))
        .copied()
        .collect();

    let stackable_fixed: Vec<&PriceRule> = stackable
        .iter()
        .filter(|r| matches!(r.adjustment_type, AdjustmentType::FixedAmount))
        .copied()
        .collect();

    debug!(
        stackable_pct_count = stackable_pct.len(),
        stackable_fixed_count = stackable_fixed.len(),
        "[DiscountRules] Stackable rules categorized"
    );

    // Capitalist mode for percentage discounts
    if !stackable_pct.is_empty() {
        let mut remaining_multiplier = Decimal::ONE;
        for rule in &stackable_pct {
            let rate = to_decimal(rule.adjustment_value) / hundred;
            remaining_multiplier *= Decimal::ONE - rate;
            trace!(
                rule_name = %rule.name,
                rate = %rate,
                remaining_multiplier = %remaining_multiplier,
                "[DiscountRules] Stackable pct rule applied"
            );
        }
        // Total percentage discount amount
        let pct_discount = price_basis * (Decimal::ONE - remaining_multiplier);
        debug!(
            pct_discount = %pct_discount,
            final_multiplier = %remaining_multiplier,
            "[DiscountRules] Capitalist mode pct discount"
        );
        total_discount += pct_discount;

        // Record each rule's individual contribution
        for rule in &stackable_pct {
            let individual_amount = price_basis * to_decimal(rule.adjustment_value) / hundred;
            applied_rules.push(AppliedRule::from_rule(rule, to_f64(individual_amount)));
        }
    }

    // Simple addition for fixed discounts (value is already in currency units)
    for rule in &stackable_fixed {
        let amount = to_decimal(rule.adjustment_value);
        debug!(
            rule_name = %rule.name,
            fixed_amount = %amount,
            "[DiscountRules] Stackable fixed rule applied"
        );
        total_discount += amount;
        applied_rules.push(AppliedRule::from_rule(rule, to_f64(amount)));
    }

    debug!(
        total_discount = %total_discount,
        applied_rules_count = applied_rules.len(),
        "[DiscountRules] Final discount result"
    );

    DiscountResult {
        amount: total_discount,
        applied: applied_rules,
    }
}

/// Calculate discount amount for a single rule
fn calculate_single_discount(rule: &PriceRule, price_basis: Decimal) -> Decimal {
    let value = to_decimal(rule.adjustment_value);
    let hundred = Decimal::ONE_HUNDRED;

    match rule.adjustment_type {
        AdjustmentType::Percentage => price_basis * value / hundred,
        AdjustmentType::FixedAmount => value, // direct currency amount
    }
}

// ==================== Surcharge Application ====================

/// Result of applying surcharge rules
pub struct SurchargeResult {
    /// Total surcharge amount
    pub amount: Decimal,
    /// Rules that were applied
    pub applied: Vec<AppliedRule>,
}

/// Apply surcharge rules with same stacking logic as discounts.
/// Surcharges are calculated based on `price_basis` (typically the base price).
pub fn apply_surcharge_rules(rules: &[&PriceRule], price_basis: Decimal) -> SurchargeResult {
    let surcharge_rules: Vec<&PriceRule> = rules
        .iter()
        .filter(|r| matches!(r.rule_type, RuleType::Surcharge))
        .copied()
        .collect();

    debug!(
        target: "pricing",
        total_rules = rules.len(),
        surcharge_rules_count = surcharge_rules.len(),
        price_basis = %price_basis,
        "Starting surcharge rule application"
    );

    if surcharge_rules.is_empty() {
        debug!(target: "pricing", "No surcharge rules to apply");
        return SurchargeResult {
            amount: Decimal::ZERO,
            applied: vec![],
        };
    }

    let mut applied_rules = Vec::new();

    // Separate rules by stacking behavior
    let exclusive: Vec<&PriceRule> = surcharge_rules
        .iter()
        .filter(|r| r.is_exclusive)
        .copied()
        .collect();

    let non_stackable: Vec<&PriceRule> = surcharge_rules
        .iter()
        .filter(|r| !r.is_exclusive && !r.is_stackable)
        .copied()
        .collect();

    let stackable: Vec<&PriceRule> = surcharge_rules
        .iter()
        .filter(|r| !r.is_exclusive && r.is_stackable)
        .copied()
        .collect();

    debug!(
        exclusive_count = exclusive.len(),
        non_stackable_count = non_stackable.len(),
        stackable_count = stackable.len(),
        "[SurchargeRules] Rules categorized"
    );

    // Step 1: Check for exclusive rules
    if let Some(winner) = select_winner(&exclusive) {
        let amount = calculate_single_surcharge(winner, price_basis);
        debug!(
            winner_name = %winner.name,
            winner_value = winner.adjustment_value,
            winner_type = ?winner.adjustment_type,
            surcharge_amount = %amount,
            "[SurchargeRules] Exclusive winner selected"
        );
        let applied = AppliedRule::from_rule(winner, to_f64(amount));
        return SurchargeResult {
            amount,
            applied: vec![applied],
        };
    }

    let mut total_surcharge = Decimal::ZERO;

    // Step 2: Apply non-stackable winner (if any)
    if let Some(winner) = select_winner(&non_stackable) {
        let amount = calculate_single_surcharge(winner, price_basis);
        debug!(
            winner_name = %winner.name,
            winner_value = winner.adjustment_value,
            winner_type = ?winner.adjustment_type,
            surcharge_amount = %amount,
            "[SurchargeRules] Non-stackable winner selected"
        );
        total_surcharge += amount;
        applied_rules.push(AppliedRule::from_rule(winner, to_f64(amount)));
    }

    // Step 3: Apply stackable rules
    // For surcharges, we use simple addition for both percentage and fixed
    // (unlike discounts, surcharges don't compound in "capitalist mode")
    for rule in &stackable {
        let amount = calculate_single_surcharge(rule, price_basis);
        debug!(
            rule_name = %rule.name,
            rule_value = rule.adjustment_value,
            rule_type = ?rule.adjustment_type,
            surcharge_amount = %amount,
            "[SurchargeRules] Stackable rule applied"
        );
        total_surcharge += amount;
        applied_rules.push(AppliedRule::from_rule(rule, to_f64(amount)));
    }

    debug!(
        total_surcharge = %total_surcharge,
        applied_rules_count = applied_rules.len(),
        "[SurchargeRules] Final surcharge result"
    );

    SurchargeResult {
        amount: total_surcharge,
        applied: applied_rules,
    }
}

/// Calculate surcharge amount for a single rule
fn calculate_single_surcharge(rule: &PriceRule, price_basis: Decimal) -> Decimal {
    let value = to_decimal(rule.adjustment_value);
    let hundred = Decimal::ONE_HUNDRED;

    match rule.adjustment_type {
        AdjustmentType::Percentage => price_basis * value / hundred,
        AdjustmentType::FixedAmount => value, // direct currency amount
    }
}

// ==================== Main Calculator ====================

/// Calculate item price with manual discount and matched rules
///
/// # Arguments
/// * `original_price` - The item's original price
/// * `options_modifier` - Price modifier from selected options
/// * `manual_discount_percent` - Manual discount percentage (0-100)
/// * `matched_rules` - Rules that matched this item
///
/// # Calculation Steps
/// 1. Calculate base = original_price + options_modifier
/// 2. Apply manual discount (percentage of base)
/// 3. Apply rule discounts (exclusive > non-stackable > stackable)
/// 4. Apply rule surcharges (based on base, same stacking logic)
///
/// # Returns
/// `ItemCalculationResult` with all intermediate values and applied rules
pub fn calculate_item_price(
    original_price: f64,
    options_modifier: f64,
    manual_discount_percent: f64,
    matched_rules: &[&PriceRule],
) -> ItemCalculationResult {
    debug!(
        original_price,
        options_modifier,
        manual_discount_percent,
        matched_rules_count = matched_rules.len(),
        "[ItemCalc] Starting calculation"
    );

    // Log each matched rule
    for (idx, rule) in matched_rules.iter().enumerate() {
        trace!(
            rule_idx = idx,
            rule_name = %rule.name,
            rule_type = ?rule.rule_type,
            adjustment_type = ?rule.adjustment_type,
            adjustment_value = rule.adjustment_value,
            is_stackable = rule.is_stackable,
            is_exclusive = rule.is_exclusive,
            effective_priority = calculate_effective_priority(rule),
            "[ItemCalc] Matched rule"
        );
    }

    let original = to_decimal(original_price);
    let modifier = to_decimal(options_modifier);
    let manual_pct = to_decimal(manual_discount_percent);
    let hundred = Decimal::ONE_HUNDRED;

    // Step 1: Calculate base price (clamped to >= 0)
    let base = (original + modifier).max(Decimal::ZERO);
    debug!(
        original = %original,
        modifier = %modifier,
        base = %base,
        "[ItemCalc] Step 1: Base price"
    );

    // Step 2: Apply manual discount (percentage of base)
    let manual_discount_amount = base * manual_pct / hundred;
    let after_manual = base - manual_discount_amount;
    debug!(
        manual_pct = %manual_pct,
        manual_discount_amount = %manual_discount_amount,
        after_manual = %after_manual,
        "[ItemCalc] Step 2: Manual discount"
    );

    // Step 3: Apply rule discounts (based on after_manual price)
    let discount_result = apply_discount_rules(matched_rules, after_manual);
    let after_discount = (after_manual - discount_result.amount).max(Decimal::ZERO);
    debug!(
        discount_amount = %discount_result.amount,
        after_discount = %after_discount,
        discount_rules_applied = discount_result.applied.len(),
        "[ItemCalc] Step 3: Rule discounts"
    );

    // Step 4: Apply rule surcharges (based on base price)
    let surcharge_result = apply_surcharge_rules(matched_rules, base);
    let item_final = (after_discount + surcharge_result.amount).max(Decimal::ZERO);
    debug!(
        surcharge_amount = %surcharge_result.amount,
        surcharge_rules_applied = surcharge_result.applied.len(),
        item_final = %item_final,
        "[ItemCalc] Step 4: Rule surcharges & final"
    );

    // Combine applied rules
    let mut applied_rules = discount_result.applied;
    applied_rules.extend(surcharge_result.applied);

    // Log final result
    debug!(
        base = to_f64(base),
        manual_discount_amount = to_f64(manual_discount_amount),
        after_manual = to_f64(after_manual),
        rule_discount_amount = to_f64(discount_result.amount),
        after_discount = to_f64(after_discount),
        rule_surcharge_amount = to_f64(surcharge_result.amount),
        item_final = to_f64(item_final),
        applied_rules_count = applied_rules.len(),
        "[ItemCalc] Final result"
    );

    ItemCalculationResult {
        base: to_f64(base),
        manual_discount_amount: to_f64(manual_discount_amount),
        after_manual: to_f64(after_manual),
        rule_discount_amount: to_f64(discount_result.amount),
        after_discount: to_f64(after_discount),
        rule_surcharge_amount: to_f64(surcharge_result.amount),
        item_final: to_f64(item_final),
        applied_rules,
    }
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;
    /// Helper to create a test rule
    fn make_rule(
        rule_type: RuleType,
        adjustment_type: AdjustmentType,
        value: f64,
        stackable: bool,
        exclusive: bool,
    ) -> PriceRule {
        PriceRule {
            id: 0,
            name: format!("rule_{}", value),
            display_name: format!("Rule {}", value),
            receipt_name: format!("R{}", value),
            description: None,
            rule_type,
            product_scope: ProductScope::Global,
            target_id: None,
            zone_scope: shared::models::ZONE_SCOPE_ALL.to_string(),
            adjustment_type,
            adjustment_value: value,
            is_stackable: stackable,
            is_exclusive: exclusive,
            valid_from: None,
            valid_until: None,
            active_days: None,
            active_start_time: None,
            active_end_time: None,
            is_active: true,
            created_by: None,
            created_at: shared::util::now_millis(),
        }
    }

    fn make_rule_with_scope(
        rule_type: RuleType,
        adjustment_type: AdjustmentType,
        value: f64,
        stackable: bool,
        exclusive: bool,
        zone_scope: &str,
        product_scope: ProductScope,
    ) -> PriceRule {
        let mut rule = make_rule(rule_type, adjustment_type, value, stackable, exclusive);
        rule.zone_scope = zone_scope.to_string();
        rule.product_scope = product_scope;
        rule
    }

    // ==================== Basic Tests ====================

    #[test]
    fn test_simple_discount() {
        // 10% discount on ¥100 = ¥10 discount, ¥90 final
        let rule = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            10.0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&rule];

        let result = calculate_item_price(100.0, 0.0, 0.0, &rules);

        assert_eq!(result.base, 100.0);
        assert_eq!(result.manual_discount_amount, 0.0);
        assert_eq!(result.after_manual, 100.0);
        assert_eq!(result.rule_discount_amount, 10.0);
        assert_eq!(result.after_discount, 90.0);
        assert_eq!(result.rule_surcharge_amount, 0.0);
        assert_eq!(result.item_final, 90.0);
        assert_eq!(result.applied_rules.len(), 1);
    }

    #[test]
    fn test_manual_then_rule_discount() {
        // ¥100 base
        // 10% manual discount -> ¥90
        // 10% rule discount on ¥90 -> ¥9 discount -> ¥81 final
        let rule = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            10.0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&rule];

        let result = calculate_item_price(100.0, 0.0, 10.0, &rules);

        assert_eq!(result.base, 100.0);
        assert_eq!(result.manual_discount_amount, 10.0);
        assert_eq!(result.after_manual, 90.0);
        assert_eq!(result.rule_discount_amount, 9.0);
        assert_eq!(result.after_discount, 81.0);
        assert_eq!(result.item_final, 81.0);
    }

    #[test]
    fn test_exclusive_wins() {
        // Exclusive 20% should win over non-exclusive 30%
        let exclusive = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            20.0,
            false,
            true,
        );
        let non_exclusive = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            30.0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&exclusive, &non_exclusive];

        let result = calculate_item_price(100.0, 0.0, 0.0, &rules);

        assert_eq!(result.rule_discount_amount, 20.0);
        assert_eq!(result.item_final, 80.0);
        assert_eq!(result.applied_rules.len(), 1);
        assert!(result.applied_rules[0].is_exclusive);
    }

    #[test]
    fn test_surcharge_based_on_base() {
        // Surcharge should be calculated on base price, not after discounts
        // ¥100 base, 10% manual discount -> ¥90
        // 10% surcharge on ¥100 base = ¥10
        // Final = ¥90 + ¥10 = ¥100
        let rule = make_rule(
            RuleType::Surcharge,
            AdjustmentType::Percentage,
            10.0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&rule];

        let result = calculate_item_price(100.0, 0.0, 10.0, &rules);

        assert_eq!(result.base, 100.0);
        assert_eq!(result.manual_discount_amount, 10.0);
        assert_eq!(result.after_manual, 90.0);
        assert_eq!(result.rule_surcharge_amount, 10.0); // Based on $100 base
        assert_eq!(result.item_final, 100.0); // $90 + $10
    }

    // ==================== Options Modifier Tests ====================

    #[test]
    fn test_options_modifier() {
        // ¥10 base + ¥2 options = ¥12 base
        // 10% discount = ¥1.20
        let rule = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            10.0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&rule];

        let result = calculate_item_price(10.0, 2.0, 0.0, &rules);

        assert_eq!(result.base, 12.0);
        assert_eq!(result.rule_discount_amount, 1.2);
        assert_eq!(result.item_final, 10.8);
    }

    // ==================== Stacking Tests ====================

    #[test]
    fn test_stackable_percentage_capitalist_mode() {
        // Two 10% stackable discounts in "capitalist mode"
        // ¥100 * (1 - 0.10) * (1 - 0.10) = ¥100 * 0.9 * 0.9 = ¥81
        // Total discount = ¥100 - ¥81 = ¥19
        let rule1 = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            10.0,
            true,
            false,
        );
        let rule2 = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            10.0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&rule1, &rule2];

        let result = calculate_item_price(100.0, 0.0, 0.0, &rules);

        assert_eq!(result.rule_discount_amount, 19.0);
        assert_eq!(result.item_final, 81.0);
        assert_eq!(result.applied_rules.len(), 2);
    }

    #[test]
    fn test_stackable_fixed_addition() {
        // Two fixed discounts: ¥5 + ¥3 = ¥8 total
        let rule1 = make_rule(
            RuleType::Discount,
            AdjustmentType::FixedAmount,
            5.0,
            true,
            false,
        );
        let rule2 = make_rule(
            RuleType::Discount,
            AdjustmentType::FixedAmount,
            3.0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&rule1, &rule2];

        let result = calculate_item_price(100.0, 0.0, 0.0, &rules);

        assert_eq!(result.rule_discount_amount, 8.0);
        assert_eq!(result.item_final, 92.0);
    }

    #[test]
    fn test_non_stackable_winner() {
        // Two non-stackable rules with same effective_priority, newer created_at wins
        let mut winner = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            15.0,
            false,
            false,
        );
        winner.created_at = 2000; // Newer
        let mut loser = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            25.0,
            false,
            false,
        );
        loser.created_at = 1000; // Older
        let rules: Vec<&PriceRule> = vec![&winner, &loser];

        let result = calculate_item_price(100.0, 0.0, 0.0, &rules);

        assert_eq!(result.rule_discount_amount, 15.0); // 15% wins (newer)
        assert_eq!(result.item_final, 85.0);
        assert_eq!(result.applied_rules.len(), 1);
    }

    #[test]
    fn test_non_stackable_with_stackable() {
        // Non-stackable 10% + stackable 5%
        // Non-stackable applies, then stackable on top
        let non_stackable = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            10.0,
            false,
            false,
        );
        let stackable = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            5.0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&non_stackable, &stackable];

        let result = calculate_item_price(100.0, 0.0, 0.0, &rules);

        // Non-stackable: $10, Stackable capitalist: $100 * (1-0.05) = $95, so $5
        // Total = $10 + $5 = $15 (but actually combined with capitalist mode)
        // Actually: non-stackable $10 + stackable $5 (simple) = $15
        // Wait, let me recalculate based on the implementation:
        // apply_discount_rules combines non-stackable ($10) + stackable_pct capitalist ($5) = $15
        assert_eq!(result.rule_discount_amount, 15.0);
        assert_eq!(result.item_final, 85.0);
    }

    // ==================== Priority Tests ====================

    #[test]
    fn test_effective_priority_full_matrix() {
        // Full 2x4 matrix: zone_weight(0|1) * 10 + product_weight(0|1|2|3)
        let cases: Vec<(&str, ProductScope, i32)> = vec![
            // zone:all (weight=0)
            (shared::models::ZONE_SCOPE_ALL, ProductScope::Global, 0),
            (shared::models::ZONE_SCOPE_ALL, ProductScope::Category, 1),
            (shared::models::ZONE_SCOPE_ALL, ProductScope::Tag, 2),
            (shared::models::ZONE_SCOPE_ALL, ProductScope::Product, 3),
            // zone:retail (weight=1)
            (shared::models::ZONE_SCOPE_RETAIL, ProductScope::Global, 10),
            (
                shared::models::ZONE_SCOPE_RETAIL,
                ProductScope::Category,
                11,
            ),
            (shared::models::ZONE_SCOPE_RETAIL, ProductScope::Tag, 12),
            (shared::models::ZONE_SCOPE_RETAIL, ProductScope::Product, 13),
            // specific zone (weight=1, same as retail)
            ("zone:dining-room", ProductScope::Global, 10),
            ("zone:dining-room", ProductScope::Category, 11),
            ("zone:dining-room", ProductScope::Tag, 12),
            ("zone:dining-room", ProductScope::Product, 13),
        ];

        for (zone, scope, expected) in cases {
            let rule = make_rule_with_scope(
                RuleType::Discount,
                AdjustmentType::Percentage,
                10.0,
                true,
                false,
                zone,
                scope.clone(),
            );
            assert_eq!(
                calculate_effective_priority(&rule),
                expected,
                "zone={}, scope={:?} should be {}",
                zone,
                scope,
                expected
            );
        }
    }

    #[test]
    fn test_specific_scope_wins_over_global() {
        // Specific product rule should win over global rule
        let global = make_rule_with_scope(
            RuleType::Discount,
            AdjustmentType::Percentage,
            30.0, // Higher discount
            false,
            false,
            shared::models::ZONE_SCOPE_ALL,
            ProductScope::Global,
        );
        let specific = make_rule_with_scope(
            RuleType::Discount,
            AdjustmentType::Percentage,
            10.0, // Lower discount
            false,
            false,
            "zone:1", // Specific zone
            ProductScope::Product,
        );
        let rules: Vec<&PriceRule> = vec![&global, &specific];

        let result = calculate_item_price(100.0, 0.0, 0.0, &rules);

        // Specific should win due to higher effective priority (13 > 0)
        assert_eq!(result.rule_discount_amount, 10.0);
        assert_eq!(result.item_final, 90.0);
    }

    #[test]
    fn test_non_stackable_higher_priority_wins_regardless_of_created_at() {
        // A rule with higher effective_priority wins even if it was created earlier
        let mut global = make_rule_with_scope(
            RuleType::Discount,
            AdjustmentType::Percentage,
            30.0, // Higher discount amount
            false,
            false,
            shared::models::ZONE_SCOPE_ALL,
            ProductScope::Global, // priority = 0
        );
        global.created_at = 9999; // Very new — but low priority

        let mut category_specific = make_rule_with_scope(
            RuleType::Discount,
            AdjustmentType::Percentage,
            5.0, // Lower discount amount
            false,
            false,
            "zone:retail",
            ProductScope::Category, // priority = 11
        );
        category_specific.created_at = 1000; // Old — but high priority

        let rules: Vec<&PriceRule> = vec![&global, &category_specific];
        let result = calculate_item_price(100.0, 0.0, 0.0, &rules);

        // Category-specific (priority 11) wins over global (priority 0)
        // even though global has newer created_at
        assert_eq!(result.rule_discount_amount, 5.0);
        assert_eq!(result.item_final, 95.0);
        assert_eq!(result.applied_rules.len(), 1);
    }

    #[test]
    fn test_non_stackable_same_priority_created_at_tiebreak() {
        // Two non-stackable rules with same effective_priority → newer created_at wins
        let mut newer = make_rule_with_scope(
            RuleType::Discount,
            AdjustmentType::Percentage,
            10.0,
            false,
            false,
            "zone:retail",
            ProductScope::Category, // priority = 11
        );
        newer.created_at = 5000;

        let mut older = make_rule_with_scope(
            RuleType::Discount,
            AdjustmentType::Percentage,
            20.0,
            false,
            false,
            "zone:bar",
            ProductScope::Category, // priority = 11 (same)
        );
        older.created_at = 1000;

        let rules: Vec<&PriceRule> = vec![&older, &newer];
        let result = calculate_item_price(100.0, 0.0, 0.0, &rules);

        // Newer (10%) wins over older (20%) at same priority level
        assert_eq!(result.rule_discount_amount, 10.0);
        assert_eq!(result.item_final, 90.0);
        assert_eq!(result.applied_rules.len(), 1);
    }

    // ==================== Surcharge Tests ====================

    #[test]
    fn test_exclusive_surcharge() {
        // Exclusive surcharge should be the only one applied
        let exclusive = make_rule(
            RuleType::Surcharge,
            AdjustmentType::Percentage,
            15.0,
            false,
            true,
        );
        let stackable = make_rule(
            RuleType::Surcharge,
            AdjustmentType::Percentage,
            5.0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&exclusive, &stackable];

        let result = calculate_item_price(100.0, 0.0, 0.0, &rules);

        assert_eq!(result.rule_surcharge_amount, 15.0);
        assert_eq!(result.item_final, 115.0);
    }

    #[test]
    fn test_stackable_surcharges() {
        // Two 5% surcharges = 10% total (simple addition, not capitalist mode)
        let rule1 = make_rule(
            RuleType::Surcharge,
            AdjustmentType::Percentage,
            5.0,
            true,
            false,
        );
        let rule2 = make_rule(
            RuleType::Surcharge,
            AdjustmentType::Percentage,
            5.0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&rule1, &rule2];

        let result = calculate_item_price(100.0, 0.0, 0.0, &rules);

        assert_eq!(result.rule_surcharge_amount, 10.0);
        assert_eq!(result.item_final, 110.0);
    }

    // ==================== Combined Tests ====================

    #[test]
    fn test_discount_and_surcharge_combined() {
        // ¥100 base
        // 10% discount on base -> ¥10 discount -> ¥90
        // 5% surcharge on base -> ¥5 surcharge
        // Final = ¥90 + ¥5 = ¥95
        let discount = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            10.0,
            true,
            false,
        );
        let surcharge = make_rule(
            RuleType::Surcharge,
            AdjustmentType::Percentage,
            5.0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&discount, &surcharge];

        let result = calculate_item_price(100.0, 0.0, 0.0, &rules);

        assert_eq!(result.rule_discount_amount, 10.0);
        assert_eq!(result.rule_surcharge_amount, 5.0);
        assert_eq!(result.item_final, 95.0);
        assert_eq!(result.applied_rules.len(), 2);
    }

    #[test]
    fn test_full_calculation_flow() {
        // ¥50 original + ¥5 options = ¥55 base
        // 20% manual discount -> ¥11 -> ¥44
        // 10% rule discount on ¥44 -> ¥4.40 -> ¥39.60
        // 5% surcharge on ¥55 base -> ¥2.75
        // Final = ¥39.60 + ¥2.75 = ¥42.35
        let discount = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            10.0,
            true,
            false,
        );
        let surcharge = make_rule(
            RuleType::Surcharge,
            AdjustmentType::Percentage,
            5.0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&discount, &surcharge];

        let result = calculate_item_price(50.0, 5.0, 20.0, &rules);

        assert_eq!(result.base, 55.0);
        assert_eq!(result.manual_discount_amount, 11.0);
        assert_eq!(result.after_manual, 44.0);
        assert_eq!(result.rule_discount_amount, 4.4);
        assert_eq!(result.after_discount, 39.6);
        assert_eq!(result.rule_surcharge_amount, 2.75);
        assert_eq!(result.item_final, 42.35);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_no_rules() {
        let rules: Vec<&PriceRule> = vec![];
        let result = calculate_item_price(100.0, 0.0, 0.0, &rules);

        assert_eq!(result.base, 100.0);
        assert_eq!(result.item_final, 100.0);
        assert!(result.applied_rules.is_empty());
    }

    #[test]
    fn test_discount_cannot_go_negative() {
        // 150% discount should result in ¥0, not negative
        let rule = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            150.0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&rule];

        let result = calculate_item_price(100.0, 0.0, 0.0, &rules);

        assert_eq!(result.after_discount, 0.0);
        assert_eq!(result.item_final, 0.0);
    }

    #[test]
    fn test_fixed_discount() {
        // ¥5.50 fixed discount
        let rule = make_rule(
            RuleType::Discount,
            AdjustmentType::FixedAmount,
            5.5,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&rule];

        let result = calculate_item_price(100.0, 0.0, 0.0, &rules);

        assert_eq!(result.rule_discount_amount, 5.5);
        assert_eq!(result.item_final, 94.5);
    }

    #[test]
    fn test_precision_rounding() {
        // ¥99.99 base with 33% discount
        // ¥99.99 * 0.33 = ¥32.9967 -> ¥33.00
        let rule = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            33.0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&rule];

        let result = calculate_item_price(99.99, 0.0, 0.0, &rules);

        assert_eq!(result.rule_discount_amount, 33.0);
        assert_eq!(result.item_final, 66.99);
    }
}
