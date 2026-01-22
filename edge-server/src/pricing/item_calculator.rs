//! Item Price Calculator
//!
//! Calculate item-level prices with support for:
//! - Manual discounts (percentage-based)
//! - Rule-based discounts (exclusive, non-stackable, stackable)
//! - Rule-based surcharges (same stacking logic)
//!
//! Uses rust_decimal for precision calculations.

use crate::db::models::{AdjustmentType, PriceRule, ProductScope, RuleType};
use rust_decimal::prelude::*;
use shared::order::AppliedRule;

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
/// Priority formula: (zone_weight * 10 + product_weight) * 1000 + user_priority
///
/// This ensures more specific rules (specific zone, specific product) have higher priority
/// than general rules (global zone, global product scope).
///
/// Zone weights:
/// - -1 (Global): 0
/// - 0 (Retail): 1
/// - >0 (Specific zone): 2
///
/// Product weights:
/// - Global: 0
/// - Category: 1
/// - Tag: 2
/// - Product: 3
pub fn calculate_effective_priority(rule: &PriceRule) -> i32 {
    let zone_weight = match rule.zone_scope {
        -1 => 0, // Global
        0 => 1,  // Retail
        _ => 2,  // Specific zone
    };

    let product_weight = match rule.product_scope {
        ProductScope::Global => 0,
        ProductScope::Category => 1,
        ProductScope::Tag => 2,
        ProductScope::Product => 3,
    };

    (zone_weight * 10 + product_weight) * 1000 + rule.priority
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
pub fn apply_discount_rules(
    rules: &[&PriceRule],
    price_basis: Decimal,
) -> DiscountResult {
    let discount_rules: Vec<&PriceRule> = rules
        .iter()
        .filter(|r| matches!(r.rule_type, RuleType::Discount))
        .copied()
        .collect();

    if discount_rules.is_empty() {
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

    // Step 1: Check for exclusive rules
    if let Some(winner) = select_winner(&exclusive) {
        let amount = calculate_single_discount(winner, price_basis);
        let applied = AppliedRule::from_rule(
            &to_shared_rule(winner),
            to_f64(amount),
        );
        return DiscountResult {
            amount,
            applied: vec![applied],
        };
    }

    let mut total_discount = Decimal::ZERO;

    // Step 2: Apply non-stackable winner (if any)
    if let Some(winner) = select_winner(&non_stackable) {
        let amount = calculate_single_discount(winner, price_basis);
        total_discount += amount;
        applied_rules.push(AppliedRule::from_rule(
            &to_shared_rule(winner),
            to_f64(amount),
        ));
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

    // Capitalist mode for percentage discounts
    if !stackable_pct.is_empty() {
        let mut remaining_multiplier = Decimal::ONE;
        for rule in &stackable_pct {
            let rate = to_decimal(rule.adjustment_value) / hundred;
            remaining_multiplier *= Decimal::ONE - rate;
        }
        // Total percentage discount amount
        let pct_discount = price_basis * (Decimal::ONE - remaining_multiplier);
        total_discount += pct_discount;

        // Record each rule's individual contribution
        for rule in &stackable_pct {
            let individual_amount = price_basis * to_decimal(rule.adjustment_value) / hundred;
            applied_rules.push(AppliedRule::from_rule(
                &to_shared_rule(rule),
                to_f64(individual_amount),
            ));
        }
    }

    // Simple addition for fixed discounts
    for rule in &stackable_fixed {
        let amount = to_decimal(rule.adjustment_value) / hundred;
        total_discount += amount;
        applied_rules.push(AppliedRule::from_rule(
            &to_shared_rule(rule),
            to_f64(amount),
        ));
    }

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
pub fn apply_surcharge_rules(
    rules: &[&PriceRule],
    price_basis: Decimal,
) -> SurchargeResult {
    let surcharge_rules: Vec<&PriceRule> = rules
        .iter()
        .filter(|r| matches!(r.rule_type, RuleType::Surcharge))
        .copied()
        .collect();

    if surcharge_rules.is_empty() {
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

    // Step 1: Check for exclusive rules
    if let Some(winner) = select_winner(&exclusive) {
        let amount = calculate_single_surcharge(winner, price_basis);
        let applied = AppliedRule::from_rule(
            &to_shared_rule(winner),
            to_f64(amount),
        );
        return SurchargeResult {
            amount,
            applied: vec![applied],
        };
    }

    let mut total_surcharge = Decimal::ZERO;

    // Step 2: Apply non-stackable winner (if any)
    if let Some(winner) = select_winner(&non_stackable) {
        let amount = calculate_single_surcharge(winner, price_basis);
        total_surcharge += amount;
        applied_rules.push(AppliedRule::from_rule(
            &to_shared_rule(winner),
            to_f64(amount),
        ));
    }

    // Step 3: Apply stackable rules
    // For surcharges, we use simple addition for both percentage and fixed
    // (unlike discounts, surcharges don't compound in "capitalist mode")
    for rule in &stackable {
        let amount = calculate_single_surcharge(rule, price_basis);
        total_surcharge += amount;
        applied_rules.push(AppliedRule::from_rule(
            &to_shared_rule(rule),
            to_f64(amount),
        ));
    }

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

// ==================== Type Conversion ====================

/// Convert edge-server PriceRule to shared PriceRule for AppliedRule creation
fn to_shared_rule(rule: &PriceRule) -> shared::models::price_rule::PriceRule {
    shared::models::price_rule::PriceRule {
        id: rule.id.as_ref().map(|t| t.to_string()),
        name: rule.name.clone(),
        display_name: rule.display_name.clone(),
        receipt_name: rule.receipt_name.clone(),
        description: rule.description.clone(),
        rule_type: match rule.rule_type {
            RuleType::Discount => shared::models::price_rule::RuleType::Discount,
            RuleType::Surcharge => shared::models::price_rule::RuleType::Surcharge,
        },
        product_scope: match rule.product_scope {
            ProductScope::Global => shared::models::price_rule::ProductScope::Global,
            ProductScope::Category => shared::models::price_rule::ProductScope::Category,
            ProductScope::Tag => shared::models::price_rule::ProductScope::Tag,
            ProductScope::Product => shared::models::price_rule::ProductScope::Product,
        },
        target: rule.target.as_ref().map(|t| t.to_string()),
        zone_scope: rule.zone_scope,
        adjustment_type: match rule.adjustment_type {
            AdjustmentType::Percentage => shared::models::price_rule::AdjustmentType::Percentage,
            AdjustmentType::FixedAmount => shared::models::price_rule::AdjustmentType::FixedAmount,
        },
        adjustment_value: rule.adjustment_value,
        priority: rule.priority,
        is_stackable: rule.is_stackable,
        is_exclusive: rule.is_exclusive,
        time_mode: match rule.time_mode {
            crate::db::models::TimeMode::Always => shared::models::price_rule::TimeMode::Always,
            crate::db::models::TimeMode::Schedule => shared::models::price_rule::TimeMode::Schedule,
            crate::db::models::TimeMode::Onetime => shared::models::price_rule::TimeMode::Onetime,
        },
        start_time: rule.start_time.clone(),
        end_time: rule.end_time.clone(),
        schedule_config: rule.schedule_config.as_ref().map(|sc| {
            shared::models::price_rule::ScheduleConfig {
                days_of_week: sc.days_of_week.clone(),
                start_time: sc.start_time.clone(),
                end_time: sc.end_time.clone(),
            }
        }),
        valid_from: rule.valid_from,
        valid_until: rule.valid_until,
        active_days: rule.active_days.clone(),
        active_start_time: rule.active_start_time.clone(),
        active_end_time: rule.active_end_time.clone(),
        is_active: rule.is_active,
        created_by: rule.created_by.as_ref().map(|t| t.to_string()),
        created_at: rule.created_at,
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
    let original = to_decimal(original_price);
    let modifier = to_decimal(options_modifier);
    let manual_pct = to_decimal(manual_discount_percent);
    let hundred = Decimal::ONE_HUNDRED;

    // Step 1: Calculate base price
    let base = original + modifier;

    // Step 2: Apply manual discount (percentage of base)
    let manual_discount_amount = base * manual_pct / hundred;
    let after_manual = base - manual_discount_amount;

    // Step 3: Apply rule discounts (based on after_manual price)
    let discount_result = apply_discount_rules(matched_rules, after_manual);
    let after_discount = (after_manual - discount_result.amount).max(Decimal::ZERO);

    // Step 4: Apply rule surcharges (based on base price)
    let surcharge_result = apply_surcharge_rules(matched_rules, base);
    let item_final = (after_discount + surcharge_result.amount).max(Decimal::ZERO);

    // Combine applied rules
    let mut applied_rules = discount_result.applied;
    applied_rules.extend(surcharge_result.applied);

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
    use crate::db::models::TimeMode;

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
            zone_scope: -1,
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

    fn make_rule_with_scope(
        rule_type: RuleType,
        adjustment_type: AdjustmentType,
        value: f64,
        priority: i32,
        stackable: bool,
        exclusive: bool,
        zone_scope: i32,
        product_scope: ProductScope,
    ) -> PriceRule {
        let mut rule = make_rule(rule_type, adjustment_type, value, priority, stackable, exclusive);
        rule.zone_scope = zone_scope;
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
            0,
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
            0,
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
            0,
            false,
            true,
        );
        let non_exclusive = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            30.0,
            0,
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
            0,
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
            0,
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
            0,
            true,
            false,
        );
        let rule2 = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            10.0,
            0,
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
            0,
            true,
            false,
        );
        let rule2 = make_rule(
            RuleType::Discount,
            AdjustmentType::FixedAmount,
            3.0,
            0,
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
        // Two non-stackable rules, higher priority wins
        let winner = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            15.0,
            10, // Higher priority
            false,
            false,
        );
        let loser = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            25.0,
            5, // Lower priority
            false,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&winner, &loser];

        let result = calculate_item_price(100.0, 0.0, 0.0, &rules);

        assert_eq!(result.rule_discount_amount, 15.0); // 15% wins
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
            0,
            false,
            false,
        );
        let stackable = make_rule(
            RuleType::Discount,
            AdjustmentType::Percentage,
            5.0,
            0,
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
    fn test_effective_priority_calculation() {
        // Global zone, global product -> (0*10 + 0) * 1000 + priority
        let global_rule = make_rule_with_scope(
            RuleType::Discount,
            AdjustmentType::Percentage,
            10.0,
            5,
            true,
            false,
            -1,
            ProductScope::Global,
        );
        assert_eq!(calculate_effective_priority(&global_rule), 5);

        // Specific zone, specific product -> (2*10 + 3) * 1000 + priority
        let specific_rule = make_rule_with_scope(
            RuleType::Discount,
            AdjustmentType::Percentage,
            10.0,
            5,
            true,
            false,
            1,
            ProductScope::Product,
        );
        assert_eq!(calculate_effective_priority(&specific_rule), 23005);

        // Retail zone, category -> (1*10 + 1) * 1000 + priority
        let retail_rule = make_rule_with_scope(
            RuleType::Discount,
            AdjustmentType::Percentage,
            10.0,
            5,
            true,
            false,
            0,
            ProductScope::Category,
        );
        assert_eq!(calculate_effective_priority(&retail_rule), 11005);
    }

    #[test]
    fn test_specific_scope_wins_over_global() {
        // Specific product rule should win over global rule
        let global = make_rule_with_scope(
            RuleType::Discount,
            AdjustmentType::Percentage,
            30.0, // Higher discount
            100, // Higher user priority
            false,
            false,
            -1,
            ProductScope::Global,
        );
        let specific = make_rule_with_scope(
            RuleType::Discount,
            AdjustmentType::Percentage,
            10.0, // Lower discount
            0,  // Lower user priority
            false,
            false,
            1,
            ProductScope::Product,
        );
        let rules: Vec<&PriceRule> = vec![&global, &specific];

        let result = calculate_item_price(100.0, 0.0, 0.0, &rules);

        // Specific should win due to higher effective priority
        assert_eq!(result.rule_discount_amount, 10.0);
        assert_eq!(result.item_final, 90.0);
    }

    // ==================== Surcharge Tests ====================

    #[test]
    fn test_exclusive_surcharge() {
        // Exclusive surcharge should be the only one applied
        let exclusive = make_rule(
            RuleType::Surcharge,
            AdjustmentType::Percentage,
            15.0,
            0,
            false,
            true,
        );
        let stackable = make_rule(
            RuleType::Surcharge,
            AdjustmentType::Percentage,
            5.0,
            0,
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
            0,
            true,
            false,
        );
        let rule2 = make_rule(
            RuleType::Surcharge,
            AdjustmentType::Percentage,
            5.0,
            0,
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
            0,
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
            0,
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
            0,
            true,
            false,
        );
        let rules: Vec<&PriceRule> = vec![&rule];

        let result = calculate_item_price(99.99, 0.0, 0.0, &rules);

        assert_eq!(result.rule_discount_amount, 33.0);
        assert_eq!(result.item_final, 66.99);
    }
}
