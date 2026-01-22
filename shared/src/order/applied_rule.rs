//! Applied Rule - tracks which rules were applied to an item/order

use crate::models::price_rule::{AdjustmentType, ProductScope, RuleType};
use serde::{Deserialize, Serialize};

/// Applied rule record
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppliedRule {
    // === Rule Identity ===
    pub rule_id: String,
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,

    // === Rule Type ===
    pub rule_type: RuleType,
    pub adjustment_type: AdjustmentType,

    // === Scope Info ===
    pub product_scope: ProductScope,
    pub zone_scope: i32,

    // === Calculation Info ===
    /// Original value (10 = 10% or ¥10)
    pub adjustment_value: f64,
    /// Calculated amount after applying rule
    pub calculated_amount: f64,
    pub priority: i32,
    pub is_stackable: bool,
    pub is_exclusive: bool,

    // === Control ===
    /// Whether this rule is skipped
    #[serde(default)]
    pub skipped: bool,
}

impl AppliedRule {
    /// Create from a PriceRule with calculated amount
    pub fn from_rule(
        rule: &crate::models::price_rule::PriceRule,
        calculated_amount: f64,
    ) -> Self {
        Self {
            rule_id: rule.id.clone().unwrap_or_default(),
            name: rule.name.clone(),
            display_name: rule.display_name.clone(),
            receipt_name: rule.receipt_name.clone(),
            rule_type: rule.rule_type.clone(),
            adjustment_type: rule.adjustment_type.clone(),
            product_scope: rule.product_scope.clone(),
            zone_scope: rule.zone_scope,
            adjustment_value: rule.adjustment_value,
            calculated_amount,
            priority: rule.priority,
            is_stackable: rule.is_stackable,
            is_exclusive: rule.is_exclusive,
            skipped: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::price_rule::{PriceRule, TimeMode};

    #[test]
    fn test_applied_rule_from_rule() {
        let rule = PriceRule {
            id: Some("rule-1".to_string()),
            name: "lunch".to_string(),
            display_name: "Lunch Discount".to_string(),
            receipt_name: "LUNCH".to_string(),
            description: None,
            rule_type: RuleType::Discount,
            product_scope: ProductScope::Global,
            target: None,
            zone_scope: -1,
            adjustment_type: AdjustmentType::Percentage,
            adjustment_value: 10.0,
            priority: 0,
            is_stackable: true,
            is_exclusive: false,
            valid_from: None,
            valid_until: None,
            active_days: None,
            active_start_time: None,
            active_end_time: None,
            is_active: true,
            created_by: None,
            created_at: 0,
            time_mode: TimeMode::Always,
            start_time: None,
            end_time: None,
            schedule_config: None,
        };

        let applied = AppliedRule::from_rule(&rule, 5.0);

        assert_eq!(applied.rule_id, "rule-1");
        assert_eq!(applied.name, "lunch");
        assert_eq!(applied.display_name, "Lunch Discount");
        assert_eq!(applied.receipt_name, "LUNCH");
        assert_eq!(applied.rule_type, RuleType::Discount);
        assert_eq!(applied.adjustment_type, AdjustmentType::Percentage);
        assert_eq!(applied.product_scope, ProductScope::Global);
        assert_eq!(applied.zone_scope, -1);
        assert_eq!(applied.adjustment_value, 10.0);
        assert_eq!(applied.calculated_amount, 5.0);
        assert_eq!(applied.priority, 0);
        assert!(applied.is_stackable);
        assert!(!applied.is_exclusive);
        assert!(!applied.skipped);
    }

    #[test]
    fn test_applied_rule_serialization() {
        let applied = AppliedRule {
            rule_id: "rule-1".to_string(),
            name: "test".to_string(),
            display_name: "Test".to_string(),
            receipt_name: "TEST".to_string(),
            rule_type: RuleType::Discount,
            adjustment_type: AdjustmentType::Percentage,
            product_scope: ProductScope::Global,
            zone_scope: -1,
            adjustment_value: 10.0,
            calculated_amount: 5.0,
            priority: 0,
            is_stackable: true,
            is_exclusive: false,
            skipped: false,
        };

        let json = serde_json::to_string(&applied).unwrap();
        let deserialized: AppliedRule = serde_json::from_str(&json).unwrap();

        assert_eq!(applied, deserialized);
    }

    #[test]
    fn test_applied_rule_skipped_default() {
        // Test that skipped defaults to false when deserializing without it
        let json = r#"{
            "rule_id": "rule-1",
            "name": "test",
            "display_name": "Test",
            "receipt_name": "TEST",
            "rule_type": "DISCOUNT",
            "adjustment_type": "PERCENTAGE",
            "product_scope": "GLOBAL",
            "zone_scope": -1,
            "adjustment_value": 10.0,
            "calculated_amount": 5.0,
            "priority": 0,
            "is_stackable": true,
            "is_exclusive": false
        }"#;

        let applied: AppliedRule = serde_json::from_str(json).unwrap();
        assert!(!applied.skipped);
    }
}
