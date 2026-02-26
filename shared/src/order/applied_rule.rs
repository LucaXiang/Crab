//! Applied Rule - tracks which rules were applied to an item/order

use crate::models::price_rule::{AdjustmentType, ProductScope, RuleType};
use serde::{Deserialize, Serialize};

/// Applied rule record
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppliedRule {
    // === Rule Identity ===
    pub rule_id: i64,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_name: Option<String>,

    // === Rule Type ===
    pub rule_type: RuleType,
    pub adjustment_type: AdjustmentType,

    // === Scope Info ===
    pub product_scope: ProductScope,
    /// Zone scope: "all", "retail", or specific zone ID
    pub zone_scope: String,

    // === Calculation Info ===
    /// Original value (10 = 10% or €10)
    pub adjustment_value: f64,
    /// Calculated amount after applying rule
    pub calculated_amount: f64,
    pub is_stackable: bool,
    pub is_exclusive: bool,

    // === Control ===
    /// Whether this rule is skipped
    #[serde(default)]
    pub skipped: bool,
}

impl AppliedRule {
    /// Create from a PriceRule with calculated amount
    pub fn from_rule(rule: &crate::models::price_rule::PriceRule, calculated_amount: f64) -> Self {
        Self {
            rule_id: rule.id,
            name: rule.name.clone(),
            receipt_name: rule.receipt_name.clone(),
            rule_type: rule.rule_type.clone(),
            adjustment_type: rule.adjustment_type.clone(),
            product_scope: rule.product_scope.clone(),
            zone_scope: rule.zone_scope.clone(),
            adjustment_value: rule.adjustment_value,
            calculated_amount,
            is_stackable: rule.is_stackable,
            is_exclusive: rule.is_exclusive,
            skipped: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::price_rule::PriceRule;

    #[test]
    fn test_applied_rule_from_rule() {
        let rule = PriceRule {
            id: 1,
            name: "lunch".to_string(),
            receipt_name: Some("LUNCH".to_string()),
            description: None,
            rule_type: RuleType::Discount,
            product_scope: ProductScope::Global,
            target_id: None,
            zone_scope: "all".to_string(),
            adjustment_type: AdjustmentType::Percentage,
            adjustment_value: 10.0,
            is_stackable: true,
            is_exclusive: false,
            valid_from: None,
            valid_until: None,
            active_days: None,
            active_start_time: None,
            active_end_time: None,
            is_active: true,
            created_by: None,
            created_at: 1704067200000,
        };

        let applied = AppliedRule::from_rule(&rule, 5.0);

        assert_eq!(applied.rule_id, 1);
        assert_eq!(applied.name, "lunch");
        assert_eq!(applied.receipt_name, Some("LUNCH".to_string()));
        assert_eq!(applied.rule_type, RuleType::Discount);
        assert_eq!(applied.adjustment_type, AdjustmentType::Percentage);
        assert_eq!(applied.product_scope, ProductScope::Global);
        assert_eq!(applied.zone_scope, "all");
        assert_eq!(applied.adjustment_value, 10.0);
        assert_eq!(applied.calculated_amount, 5.0);
        assert!(applied.is_stackable);
        assert!(!applied.is_exclusive);
        assert!(!applied.skipped);
    }

    #[test]
    fn test_applied_rule_serialization() {
        let applied = AppliedRule {
            rule_id: 1,
            name: "test".to_string(),
            receipt_name: Some("TEST".to_string()),
            rule_type: RuleType::Discount,
            adjustment_type: AdjustmentType::Percentage,
            product_scope: ProductScope::Global,
            zone_scope: "all".to_string(),
            adjustment_value: 10.0,
            calculated_amount: 5.0,
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
        let json = r#"{
            "rule_id": 1,
            "name": "test",
            "receipt_name": "TEST",
            "rule_type": "DISCOUNT",
            "adjustment_type": "PERCENTAGE",
            "product_scope": "GLOBAL",
            "zone_scope": "all",
            "adjustment_value": 10.0,
            "calculated_amount": 5.0,
            "is_stackable": true,
            "is_exclusive": false
        }"#;

        let applied: AppliedRule = serde_json::from_str(json).unwrap();
        assert!(!applied.skipped);
    }
}
