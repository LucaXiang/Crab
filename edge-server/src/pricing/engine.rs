//! Price Rule Engine
//!
//! Main engine for applying price rules to cart items.

use crate::db::models::PriceRule;
use crate::db::repository::PriceRuleRepository;
use crate::services::CatalogService;
use shared::order::CartItemInput;
use std::sync::Arc;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

use super::calculator::calculate_adjustments;
use super::matcher::{is_time_valid, matches_product_scope, matches_zone_scope};

/// Price Rule Engine - applies price rules to cart items
#[derive(Clone)]
pub struct PriceRuleEngine {
    price_rule_repo: PriceRuleRepository,
    catalog_service: Arc<CatalogService>,
    tz: chrono_tz::Tz,
}

impl std::fmt::Debug for PriceRuleEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PriceRuleEngine")
            .field("price_rule_repo", &"<PriceRuleRepository>")
            .field("catalog_service", &"<CatalogService>")
            .finish()
    }
}

impl PriceRuleEngine {
    pub fn new(db: Surreal<Db>, catalog_service: Arc<CatalogService>, tz: chrono_tz::Tz) -> Self {
        Self {
            price_rule_repo: PriceRuleRepository::new(db),
            catalog_service,
            tz,
        }
    }

    /// Load active rules for a zone
    ///
    /// # Arguments
    /// * `zone_id` - The zone ID (None for retail)
    /// * `is_retail` - Whether this is a retail order
    pub async fn load_rules_for_zone(
        &self,
        zone_id: Option<&str>,
        is_retail: bool,
    ) -> Vec<PriceRule> {
        let all_rules = match self.price_rule_repo.find_all().await {
            Ok(rules) => rules,
            Err(e) => {
                tracing::error!("Failed to load price rules: {:?}", e);
                return vec![];
            }
        };

        // Filter rules by zone scope
        all_rules
            .into_iter()
            .filter(|rule| matches_zone_scope(rule, zone_id, is_retail))
            .collect()
    }

    /// Apply price rules to a list of cart items
    ///
    /// # Arguments
    /// * `items` - Input items (from frontend, without price rule adjustments)
    /// * `rules` - Active rules for the zone
    /// * `current_time` - Current timestamp for time-based rule validation
    ///
    /// # Returns
    /// Cart items with price rules applied (surcharge and manual_discount_percent set)
    pub fn apply_rules(
        &self,
        items: Vec<CartItemInput>,
        rules: &[PriceRule],
        current_time: i64,
    ) -> Vec<CartItemInput> {
        let mut result = Vec::with_capacity(items.len());

        for item in items {
            let processed = self.apply_rules_to_item(item, rules, current_time);
            result.push(processed);
        }

        result
    }

    /// Apply rules to a single item
    fn apply_rules_to_item(
        &self,
        item: CartItemInput,
        rules: &[PriceRule],
        current_time: i64,
    ) -> CartItemInput {
        // Get product metadata from CatalogService
        let meta = match self.catalog_service.get_product_meta(&item.product_id) {
            Some(m) => m,
            None => {
                tracing::warn!(
                    "Product {} not found in catalog, skipping price rules",
                    item.product_id
                );
                return item;
            }
        };

        // Match rules to this product
        let matched_rules = self.match_rules_for_item(
            &item.product_id,
            Some(&meta.category_id),
            &meta.tags,
            rules,
            current_time,
        );

        if matched_rules.is_empty() {
            return item;
        }

        // Calculate base price (original_price or price)
        let base_price = item.original_price.unwrap_or(item.price);

        // Apply options modifier to base price
        let options_modifier: f64 = item
            .selected_options
            .as_ref()
            .map(|opts| opts.iter().filter_map(|o| o.price_modifier).sum())
            .unwrap_or(0.0);
        let price_with_options = base_price + options_modifier;

        // Calculate adjustments
        let adjustment = calculate_adjustments(&matched_rules, price_with_options);

        // NOTE: This OLD engine is no longer called from bridge.
        // Rule adjustments are now handled by item_calculator in the reducer.
        // The adjustments are computed here but not applied to CartItemInput
        // (which lacks rule_discount_amount/rule_surcharge_amount fields).
        // This code is kept for compilation only; consider deleting this engine.
        let _ = adjustment;

        item
    }

    /// Match rules to a specific product
    fn match_rules_for_item<'a>(
        &self,
        product_id: &str,
        category_id: Option<&str>,
        tags: &[String],
        rules: &'a [PriceRule],
        current_time: i64,
    ) -> Vec<&'a PriceRule> {
        rules
            .iter()
            .filter(|rule| {
                // Check product scope
                if !matches_product_scope(rule, product_id, category_id, tags) {
                    return false;
                }

                // Check time validity
                if !is_time_valid(rule, current_time, self.tz) {
                    return false;
                }

                true
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would go here
    // Requires database setup
}
