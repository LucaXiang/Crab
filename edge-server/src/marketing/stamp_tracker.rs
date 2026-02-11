//! Stamp Tracker
//!
//! Pure functions for stamp counting and reward selection.
//! Works with CartItemSnapshot from the order system.
//!
//! Note: CartItemSnapshot uses `id` for product_id and has no `category_id` field.
//! The caller must provide category_id via StampItemInfo or a lookup function.

use shared::models::{RewardStrategy, StampRewardTarget, StampTarget, StampTargetType};
use shared::order::CartItemSnapshot;

/// Lightweight info struct pairing a cart item with its category_id.
///
/// CartItemSnapshot lacks `category_id` (only has `category_name`).
/// The caller constructs this from cart items + CatalogService metadata.
pub struct StampItemInfo<'a> {
    pub item: &'a CartItemSnapshot,
    pub category_id: Option<i64>,
}

/// Count how many stamps an order earns for a given activity.
///
/// For each non-comped item, check if it matches any stamp target.
/// Sum up quantities of matching items.
/// Comped items never count — "买二送一带走三个 ≠ 买二带走两个".
pub fn count_stamps_for_order(
    items: &[StampItemInfo<'_>],
    stamp_targets: &[StampTarget],
) -> i32 {
    items
        .iter()
        .filter(|info| !info.item.is_comped && matches_stamp_target(info, stamp_targets))
        .map(|info| info.item.quantity)
        .sum()
}

/// Check if an item matches any stamp target
fn matches_stamp_target(info: &StampItemInfo<'_>, targets: &[StampTarget]) -> bool {
    targets.iter().any(|t| match t.target_type {
        StampTargetType::Product => t.target_id == info.item.id,
        StampTargetType::Category => Some(t.target_id) == info.category_id,
    })
}

/// Find the item to comp based on reward strategy.
///
/// Returns `instance_id` of the item to comp.
///
/// - Economizador: cheapest matching non-comped item
/// - Generoso: most expensive matching non-comped item
/// - Designated: handled at action level (returns None)
pub fn find_reward_item(
    items: &[StampItemInfo<'_>],
    reward_targets: &[StampRewardTarget],
    strategy: &RewardStrategy,
) -> Option<String> {
    match strategy {
        RewardStrategy::Economizador => items
            .iter()
            .filter(|info| !info.item.is_comped && matches_reward_target(info, reward_targets))
            .min_by(|a, b| {
                a.item
                    .unit_price
                    .partial_cmp(&b.item.unit_price)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|info| info.item.instance_id.clone()),
        RewardStrategy::Generoso => items
            .iter()
            .filter(|info| !info.item.is_comped && matches_reward_target(info, reward_targets))
            .max_by(|a, b| {
                a.item
                    .unit_price
                    .partial_cmp(&b.item.unit_price)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|info| info.item.instance_id.clone()),
        RewardStrategy::Designated => {
            // Designated uses designated_product_id, handled at action level
            None
        }
    }
}

/// Check if an item matches any reward target
fn matches_reward_target(info: &StampItemInfo<'_>, targets: &[StampRewardTarget]) -> bool {
    targets.iter().any(|t| match t.target_type {
        StampTargetType::Product => t.target_id == info.item.id,
        StampTargetType::Category => Some(t.target_id) == info.category_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::CartItemSnapshot;

    /// Helper to create a minimal CartItemSnapshot for testing
    fn make_item(
        product_id: i64,
        instance_id: &str,
        quantity: i32,
        unit_price: f64,
        is_comped: bool,
    ) -> CartItemSnapshot {
        CartItemSnapshot {
            id: product_id,
            instance_id: instance_id.to_string(),
            name: format!("Product {}", product_id),
            price: unit_price,
            original_price: unit_price,
            quantity,
            unpaid_quantity: quantity,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: 0.0,
            rule_surcharge_amount: 0.0,
            applied_rules: vec![],
            applied_mg_rules: vec![],
            mg_discount_amount: 0.0,
            unit_price,
            line_total: unit_price * quantity as f64,
            tax: 0.0,
            tax_rate: 0,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_id: None,
            category_name: None,
            is_comped,
        }
    }

    fn make_stamp_info<'a>(
        item: &'a CartItemSnapshot,
        category_id: Option<i64>,
    ) -> StampItemInfo<'a> {
        StampItemInfo { item, category_id }
    }

    fn make_stamp_target(target_type: StampTargetType, target_id: i64) -> StampTarget {
        StampTarget {
            id: 1,
            stamp_activity_id: 1,
            target_type,
            target_id,
        }
    }

    fn make_reward_target(target_type: StampTargetType, target_id: i64) -> StampRewardTarget {
        StampRewardTarget {
            id: 1,
            stamp_activity_id: 1,
            target_type,
            target_id,
        }
    }

    #[test]
    fn test_count_stamps_matching_category() {
        let item1 = make_item(1, "inst-1", 2, 10.0, false);
        let item2 = make_item(2, "inst-2", 3, 15.0, false);
        let items = vec![
            make_stamp_info(&item1, Some(100)),
            make_stamp_info(&item2, Some(200)),
        ];
        let targets = vec![make_stamp_target(StampTargetType::Category, 100)];

        let count = count_stamps_for_order(&items, &targets);

        // Only item1 (category 100) matches, quantity=2
        assert_eq!(count, 2);
    }

    #[test]
    fn test_count_stamps_matching_product() {
        let item1 = make_item(42, "inst-1", 1, 10.0, false);
        let item2 = make_item(43, "inst-2", 5, 15.0, false);
        let items = vec![
            make_stamp_info(&item1, Some(100)),
            make_stamp_info(&item2, Some(100)),
        ];
        let targets = vec![make_stamp_target(StampTargetType::Product, 42)];

        let count = count_stamps_for_order(&items, &targets);

        // Only item1 (product 42) matches, quantity=1
        assert_eq!(count, 1);
    }

    #[test]
    fn test_count_stamps_excludes_comp_items() {
        let item1 = make_item(42, "inst-1", 3, 10.0, true); // comped
        let item2 = make_item(42, "inst-2", 2, 10.0, false); // not comped
        let items = vec![
            make_stamp_info(&item1, Some(100)),
            make_stamp_info(&item2, Some(100)),
        ];
        let targets = vec![make_stamp_target(StampTargetType::Product, 42)];

        let count = count_stamps_for_order(&items, &targets);

        // Only item2 counts (item1 is comped), quantity=2
        assert_eq!(count, 2);
    }

    #[test]
    fn test_count_stamps_no_match() {
        let item1 = make_item(1, "inst-1", 5, 10.0, false);
        let items = vec![make_stamp_info(&item1, Some(100))];
        let targets = vec![make_stamp_target(StampTargetType::Product, 999)];

        let count = count_stamps_for_order(&items, &targets);

        assert_eq!(count, 0);
    }

    #[test]
    fn test_find_reward_economizador() {
        let item1 = make_item(1, "inst-1", 1, 20.0, false); // expensive
        let item2 = make_item(2, "inst-2", 1, 5.0, false); // cheap
        let item3 = make_item(3, "inst-3", 1, 15.0, false); // mid
        let items = vec![
            make_stamp_info(&item1, Some(100)),
            make_stamp_info(&item2, Some(100)),
            make_stamp_info(&item3, Some(100)),
        ];
        let targets = vec![make_reward_target(StampTargetType::Category, 100)];

        let result = find_reward_item(&items, &targets, &RewardStrategy::Economizador);

        // Cheapest item (5.0) should be selected
        assert_eq!(result, Some("inst-2".to_string()));
    }

    #[test]
    fn test_find_reward_generoso() {
        let item1 = make_item(1, "inst-1", 1, 20.0, false); // expensive
        let item2 = make_item(2, "inst-2", 1, 5.0, false); // cheap
        let item3 = make_item(3, "inst-3", 1, 15.0, false); // mid
        let items = vec![
            make_stamp_info(&item1, Some(100)),
            make_stamp_info(&item2, Some(100)),
            make_stamp_info(&item3, Some(100)),
        ];
        let targets = vec![make_reward_target(StampTargetType::Category, 100)];

        let result = find_reward_item(&items, &targets, &RewardStrategy::Generoso);

        // Most expensive item (20.0) should be selected
        assert_eq!(result, Some("inst-1".to_string()));
    }

    #[test]
    fn test_find_reward_designated_returns_none() {
        let item1 = make_item(1, "inst-1", 1, 10.0, false);
        let items = vec![make_stamp_info(&item1, Some(100))];
        let targets = vec![make_reward_target(StampTargetType::Category, 100)];

        let result = find_reward_item(&items, &targets, &RewardStrategy::Designated);

        // Designated strategy returns None (handled at action level)
        assert_eq!(result, None);
    }

    // ========== Edge Cases ==========

    #[test]
    fn test_count_stamps_empty_order() {
        let items: Vec<StampItemInfo<'_>> = vec![];
        let targets = vec![make_stamp_target(StampTargetType::Product, 1)];
        assert_eq!(count_stamps_for_order(&items, &targets), 0);
    }

    #[test]
    fn test_count_stamps_empty_targets() {
        let item = make_item(1, "inst-1", 5, 10.0, false);
        let items = vec![make_stamp_info(&item, Some(100))];
        assert_eq!(count_stamps_for_order(&items, &[]), 0);
    }

    #[test]
    fn test_count_stamps_all_comped() {
        let item1 = make_item(1, "inst-1", 3, 10.0, true);
        let item2 = make_item(2, "inst-2", 2, 10.0, true);
        let items = vec![
            make_stamp_info(&item1, Some(100)),
            make_stamp_info(&item2, Some(100)),
        ];
        let targets = vec![make_stamp_target(StampTargetType::Category, 100)];
        assert_eq!(count_stamps_for_order(&items, &targets), 0);
    }

    #[test]
    fn test_count_stamps_multiple_targets_product_and_category() {
        let item1 = make_item(1, "inst-1", 2, 10.0, false); // cat 100
        let item2 = make_item(2, "inst-2", 3, 15.0, false); // cat 200
        let item3 = make_item(3, "inst-3", 1, 5.0, false);  // cat 300
        let items = vec![
            make_stamp_info(&item1, Some(100)),
            make_stamp_info(&item2, Some(200)),
            make_stamp_info(&item3, Some(300)),
        ];
        // Target: product 2 OR category 100
        let targets = vec![
            make_stamp_target(StampTargetType::Product, 2),
            make_stamp_target(StampTargetType::Category, 100),
        ];
        // item1 matches cat 100 (qty=2), item2 matches product 2 (qty=3)
        assert_eq!(count_stamps_for_order(&items, &targets), 5);
    }

    #[test]
    fn test_count_stamps_large_quantities() {
        let item = make_item(1, "inst-1", 100, 10.0, false);
        let items = vec![make_stamp_info(&item, Some(100))];
        let targets = vec![make_stamp_target(StampTargetType::Category, 100)];
        assert_eq!(count_stamps_for_order(&items, &targets), 100);
    }

    #[test]
    fn test_count_stamps_item_without_category_matches_product_target() {
        // item has no category_id but matches product target
        let item = make_item(42, "inst-1", 3, 10.0, false);
        let items = vec![make_stamp_info(&item, None)]; // no category
        let targets = vec![make_stamp_target(StampTargetType::Product, 42)];
        assert_eq!(count_stamps_for_order(&items, &targets), 3);
    }

    #[test]
    fn test_count_stamps_item_without_category_no_match_category_target() {
        // item has no category_id, category target can't match
        let item = make_item(1, "inst-1", 5, 10.0, false);
        let items = vec![make_stamp_info(&item, None)];
        let targets = vec![make_stamp_target(StampTargetType::Category, 100)];
        assert_eq!(count_stamps_for_order(&items, &targets), 0);
    }

    #[test]
    fn test_find_reward_all_comped_returns_none() {
        let mut item = make_item(1, "inst-1", 1, 10.0, true);
        item.is_comped = true;
        let items = vec![make_stamp_info(&item, Some(100))];
        let targets = vec![make_reward_target(StampTargetType::Category, 100)];
        assert_eq!(find_reward_item(&items, &targets, &RewardStrategy::Economizador), None);
    }

    #[test]
    fn test_find_reward_same_price_picks_first() {
        let item1 = make_item(1, "inst-1", 1, 10.0, false);
        let item2 = make_item(2, "inst-2", 1, 10.0, false);
        let items = vec![
            make_stamp_info(&item1, Some(100)),
            make_stamp_info(&item2, Some(100)),
        ];
        let targets = vec![make_reward_target(StampTargetType::Category, 100)];

        // Same price → min_by returns first match
        let result = find_reward_item(&items, &targets, &RewardStrategy::Economizador);
        assert_eq!(result, Some("inst-1".to_string()));
    }
}
