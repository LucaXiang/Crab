//! Order snapshot utilities
//!
//! This module provides utilities for order snapshot computation:
//! - `generate_instance_id`: Generate content-addressed instance IDs for items
//! - `input_to_snapshot`: Convert CartItemInput to CartItemSnapshot
//! - `input_to_snapshot_with_rules`: Convert CartItemInput to CartItemSnapshot with price rules applied
//!
//! Note: Event application logic has been moved to the appliers module.
//! Use `EventAction` from `super::appliers` to apply events to snapshots.

use crate::db::models::PriceRule;
use crate::pricing::calculate_item_price;
use shared::order::CartItemSnapshot;

/// Generate a content-addressed instance_id from CartItemInput
///
/// The instance_id is a hash of the item's identity-defining properties:
/// - product_id: 商品唯一标识
/// - price: 输入价格（直接使用 CartItemInput.price）
/// - manual_discount_percent: 手动折扣
/// - selected_options: 选项（attribute_id + option_idx）
/// - selected_specification: 规格
///
/// Items with the same instance_id can be merged (quantities added together).
///
/// 注意：instance_id 完全基于 CartItemInput 字段生成，不受规则计算结果影响。
/// 这确保了同一商品在任何时刻（规则缓存是否存在）都能正确合并。
pub fn generate_instance_id(input: &shared::order::CartItemInput) -> String {
    generate_instance_id_from_parts(
        &input.product_id,
        input.price,
        input.manual_discount_percent,
        &input.selected_options,
        &input.selected_specification,
    )
}

/// Internal helper to generate instance_id from individual parts
///
/// This is used by `generate_instance_id` and also by modify_item when
/// computing instance_id for modified item portions.
pub(crate) fn generate_instance_id_from_parts(
    product_id: &str,
    price: f64,
    manual_discount_percent: Option<f64>,
    options: &Option<Vec<shared::order::ItemOption>>,
    specification: &Option<shared::order::SpecificationInfo>,
) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();

    hasher.update(product_id.as_bytes());
    hasher.update(price.to_be_bytes());

    if let Some(discount) = manual_discount_percent {
        hasher.update(discount.to_be_bytes());
    }

    if let Some(opts) = options {
        for opt in opts {
            hasher.update(opt.attribute_id.as_bytes());
            hasher.update(opt.option_idx.to_be_bytes());
        }
    }

    if let Some(spec) = specification {
        hasher.update(spec.id.as_bytes());
    }

    let result = hasher.finalize();
    hex::encode(&result[..16]) // Use first 16 bytes for shorter ID
}

/// Convert CartItemInput to CartItemSnapshot with generated instance_id
///
/// This is a convenience function that calls `input_to_snapshot_with_rules` with empty rules.
pub fn input_to_snapshot(input: &shared::order::CartItemInput) -> CartItemSnapshot {
    input_to_snapshot_with_rules(input, &[])
}

/// Convert CartItemInput to CartItemSnapshot with price rules applied
///
/// This function calculates the final price considering:
/// 1. Base price (original_price or price) + options modifier
/// 2. Manual discount percentage
/// 3. Rule-based discounts and surcharges
///
/// # Arguments
/// * `input` - The cart item input to convert
/// * `rules` - Matched price rules to apply
///
/// # Returns
/// A CartItemSnapshot with calculated prices and applied rules
pub fn input_to_snapshot_with_rules(
    input: &shared::order::CartItemInput,
    rules: &[&PriceRule],
) -> CartItemSnapshot {
    // Calculate options modifier from selected_options
    let options_modifier: f64 = input
        .selected_options
        .as_ref()
        .map(|opts| opts.iter().filter_map(|o| o.price_modifier).sum())
        .unwrap_or(0.0);

    let manual_discount = input.manual_discount_percent.unwrap_or(0.0);
    let base_price = input.original_price.unwrap_or(input.price);

    // Calculate item price with rules
    let calc_result = calculate_item_price(base_price, options_modifier, manual_discount, rules);

    // Generate instance_id directly from CartItemInput
    // instance_id 完全基于 CartItemInput 字段生成，确保一致性
    let instance_id = generate_instance_id(input);

    CartItemSnapshot {
        id: input.product_id.clone(),
        instance_id,
        name: input.name.clone(),
        price: calc_result.item_final,
        original_price: input.original_price,
        quantity: input.quantity,
        unpaid_quantity: input.quantity, // Initially all unpaid
        selected_options: input.selected_options.clone(),
        selected_specification: input.selected_specification.clone(),
        manual_discount_percent: input.manual_discount_percent,
        rule_discount_amount: if calc_result.rule_discount_amount > 0.0 {
            Some(calc_result.rule_discount_amount)
        } else {
            None
        },
        rule_surcharge_amount: if calc_result.rule_surcharge_amount > 0.0 {
            Some(calc_result.rule_surcharge_amount)
        } else {
            None
        },
        applied_rules: if calc_result.applied_rules.is_empty() {
            None
        } else {
            Some(calc_result.applied_rules)
        },
        surcharge: input.surcharge,
        note: input.note.clone(),
        authorizer_id: input.authorizer_id.clone(),
        authorizer_name: input.authorizer_name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_instance_id_from_parts() {
        let id1 = generate_instance_id_from_parts("product-1", 10.0, None, &None, &None);
        let id2 = generate_instance_id_from_parts("product-1", 10.0, None, &None, &None);
        let id3 = generate_instance_id_from_parts("product-1", 10.0, Some(50.0), &None, &None);

        // Same inputs should produce same ID
        assert_eq!(id1, id2);

        // Different inputs should produce different ID
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_generate_instance_id_with_price_difference() {
        let id1 = generate_instance_id_from_parts("product-1", 10.0, None, &None, &None);
        let id2 = generate_instance_id_from_parts("product-1", 15.0, None, &None, &None);

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_instance_id_with_options() {
        let opts = Some(vec![shared::order::ItemOption {
            attribute_id: "size".to_string(),
            attribute_name: "Size".to_string(),
            option_idx: 1,
            option_name: "Large".to_string(),
            price_modifier: Some(2.0),
        }]);

        let id1 = generate_instance_id_from_parts("product-1", 10.0, None, &None, &None);
        let id2 = generate_instance_id_from_parts("product-1", 10.0, None, &opts, &None);

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_instance_id_from_input() {
        // Test the public API that takes CartItemInput
        let input = shared::order::CartItemInput {
            product_id: "product-1".to_string(),
            name: "Test Product".to_string(),
            price: 10.0,
            original_price: None,
            quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            surcharge: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let id1 = generate_instance_id(&input);
        let id2 = generate_instance_id(&input);

        // Same input should produce same ID
        assert_eq!(id1, id2);

        // Should match the from_parts version
        let id_from_parts = generate_instance_id_from_parts(
            &input.product_id,
            input.price,
            input.manual_discount_percent,
            &input.selected_options,
            &input.selected_specification,
        );
        assert_eq!(id1, id_from_parts);
    }

    #[test]
    fn test_input_to_snapshot() {
        let input = shared::order::CartItemInput {
            product_id: "product-1".to_string(),
            name: "Test Product".to_string(),
            price: 10.0,
            original_price: None,
            quantity: 2,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: Some(10.0),
            surcharge: None,
            note: Some("Test note".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        };

        let snapshot = input_to_snapshot(&input);

        assert_eq!(snapshot.id, "product-1");
        assert_eq!(snapshot.name, "Test Product");
        // Price is now calculated: base $10, 10% manual discount = $9
        assert_eq!(snapshot.price, 9.0);
        assert_eq!(snapshot.quantity, 2);
        assert_eq!(snapshot.unpaid_quantity, 2);
        assert_eq!(snapshot.manual_discount_percent, Some(10.0));
        assert_eq!(snapshot.note, Some("Test note".to_string()));
        assert!(!snapshot.instance_id.is_empty());
    }

    #[test]
    fn test_input_to_snapshot_with_rules_no_rules() {
        let input = shared::order::CartItemInput {
            product_id: "product-1".to_string(),
            name: "Test Product".to_string(),
            price: 100.0,
            original_price: None,
            quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            surcharge: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        let snapshot = input_to_snapshot_with_rules(&input, &[]);

        assert_eq!(snapshot.price, 100.0);
        assert!(snapshot.rule_discount_amount.is_none());
        assert!(snapshot.rule_surcharge_amount.is_none());
        assert!(snapshot.applied_rules.is_none());
    }

    #[test]
    fn test_input_to_snapshot_with_rules_discount() {
        use crate::db::models::{AdjustmentType, ProductScope, RuleType, TimeMode};

        let input = shared::order::CartItemInput {
            product_id: "product-1".to_string(),
            name: "Test Product".to_string(),
            price: 100.0,
            original_price: None,
            quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            surcharge: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        // 10% discount rule
        let discount_rule = PriceRule {
            id: None,
            name: "test_discount".to_string(),
            display_name: "Test Discount".to_string(),
            receipt_name: "TD".to_string(),
            description: None,
            rule_type: RuleType::Discount,
            product_scope: ProductScope::Global,
            target: None,
            zone_scope: -1,
            adjustment_type: AdjustmentType::Percentage,
            adjustment_value: 10,
            priority: 0,
            is_stackable: true,
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
        };

        let rules: Vec<&PriceRule> = vec![&discount_rule];
        let snapshot = input_to_snapshot_with_rules(&input, &rules);

        // $100 - 10% = $90
        assert_eq!(snapshot.price, 90.0);
        assert_eq!(snapshot.rule_discount_amount, Some(10.0));
        assert!(snapshot.rule_surcharge_amount.is_none());
        assert!(snapshot.applied_rules.is_some());
        assert_eq!(snapshot.applied_rules.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_input_to_snapshot_with_rules_and_options() {
        use crate::db::models::{AdjustmentType, ProductScope, RuleType, TimeMode};
        use shared::order::ItemOption;

        let input = shared::order::CartItemInput {
            product_id: "product-1".to_string(),
            name: "Test Product".to_string(),
            price: 100.0,
            original_price: None,
            quantity: 1,
            selected_options: Some(vec![
                ItemOption {
                    attribute_id: "attr-1".to_string(),
                    attribute_name: "Size".to_string(),
                    option_idx: 1,
                    option_name: "Large".to_string(),
                    price_modifier: Some(5.0),
                },
                ItemOption {
                    attribute_id: "attr-2".to_string(),
                    attribute_name: "Extra".to_string(),
                    option_idx: 0,
                    option_name: "Cheese".to_string(),
                    price_modifier: Some(2.0),
                },
            ]),
            selected_specification: None,
            manual_discount_percent: None,
            surcharge: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        // 10% discount rule
        let discount_rule = PriceRule {
            id: None,
            name: "test_discount".to_string(),
            display_name: "Test Discount".to_string(),
            receipt_name: "TD".to_string(),
            description: None,
            rule_type: RuleType::Discount,
            product_scope: ProductScope::Global,
            target: None,
            zone_scope: -1,
            adjustment_type: AdjustmentType::Percentage,
            adjustment_value: 10,
            priority: 0,
            is_stackable: true,
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
        };

        let rules: Vec<&PriceRule> = vec![&discount_rule];
        let snapshot = input_to_snapshot_with_rules(&input, &rules);

        // Base: $100 + $5 + $2 = $107
        // 10% discount on $107 = $10.70
        // Final: $107 - $10.70 = $96.30
        assert_eq!(snapshot.price, 96.3);
        assert_eq!(snapshot.rule_discount_amount, Some(10.7));
    }

    #[test]
    fn test_input_to_snapshot_with_manual_and_rule_discount() {
        use crate::db::models::{AdjustmentType, ProductScope, RuleType, TimeMode};

        let input = shared::order::CartItemInput {
            product_id: "product-1".to_string(),
            name: "Test Product".to_string(),
            price: 100.0,
            original_price: None,
            quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: Some(10.0), // 10% manual discount
            surcharge: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        // 10% rule discount
        let discount_rule = PriceRule {
            id: None,
            name: "test_discount".to_string(),
            display_name: "Test Discount".to_string(),
            receipt_name: "TD".to_string(),
            description: None,
            rule_type: RuleType::Discount,
            product_scope: ProductScope::Global,
            target: None,
            zone_scope: -1,
            adjustment_type: AdjustmentType::Percentage,
            adjustment_value: 10,
            priority: 0,
            is_stackable: true,
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
        };

        let rules: Vec<&PriceRule> = vec![&discount_rule];
        let snapshot = input_to_snapshot_with_rules(&input, &rules);

        // $100 base
        // 10% manual discount -> $90
        // 10% rule discount on $90 -> $9 discount -> $81
        assert_eq!(snapshot.price, 81.0);
        assert_eq!(snapshot.manual_discount_percent, Some(10.0));
        assert_eq!(snapshot.rule_discount_amount, Some(9.0));
    }

    #[test]
    fn test_instance_id_consistent_with_or_without_rules() {
        use crate::db::models::{AdjustmentType, ProductScope, RuleType, TimeMode};

        // Same input for both cases
        let input = shared::order::CartItemInput {
            product_id: "product-1".to_string(),
            name: "Test Product".to_string(),
            price: 100.0,
            original_price: None,
            quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            surcharge: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        // Case 1: Without rules (e.g., cache miss)
        let snapshot_no_rules = input_to_snapshot_with_rules(&input, &[]);

        // Case 2: With a 10% discount rule
        let discount_rule = PriceRule {
            id: None,
            name: "test_discount".to_string(),
            display_name: "Test Discount".to_string(),
            receipt_name: "TD".to_string(),
            description: None,
            rule_type: RuleType::Discount,
            product_scope: ProductScope::Global,
            target: None,
            zone_scope: -1,
            adjustment_type: AdjustmentType::Percentage,
            adjustment_value: 10,
            priority: 0,
            is_stackable: true,
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
        };

        let rules: Vec<&PriceRule> = vec![&discount_rule];
        let snapshot_with_rules = input_to_snapshot_with_rules(&input, &rules);

        // Prices are different (as expected)
        assert_eq!(snapshot_no_rules.price, 100.0);
        assert_eq!(snapshot_with_rules.price, 90.0);

        // But instance_id MUST be the same!
        // This ensures hash chain consistency regardless of rule cache state
        assert_eq!(
            snapshot_no_rules.instance_id, snapshot_with_rules.instance_id,
            "instance_id should be the same regardless of rules applied"
        );
    }
}
