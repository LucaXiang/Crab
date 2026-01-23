//! Price Rule Engine
//!
//! Main engine for applying price rules to cart items.

use crate::db::models::{PriceRule, Product};
use crate::db::repository::PriceRuleRepository;
use shared::order::CartItemInput;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use super::calculator::calculate_adjustments;
use super::matcher::{is_time_valid, matches_product_scope, matches_zone_scope};

/// Price Rule Engine - applies price rules to cart items
#[derive(Clone)]
pub struct PriceRuleEngine {
    db: Surreal<Db>,
    price_rule_repo: PriceRuleRepository,
}

impl std::fmt::Debug for PriceRuleEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PriceRuleEngine")
            .field("db", &"<Surreal<Db>>")
            .field("price_rule_repo", &"<PriceRuleRepository>")
            .finish()
    }
}

impl PriceRuleEngine {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            db: db.clone(),
            price_rule_repo: PriceRuleRepository::new(db),
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
    pub async fn apply_rules(
        &self,
        items: Vec<CartItemInput>,
        rules: &[PriceRule],
        current_time: i64,
    ) -> Vec<CartItemInput> {
        let mut result = Vec::with_capacity(items.len());

        for item in items {
            let processed = self.apply_rules_to_item(item, rules, current_time).await;
            result.push(processed);
        }

        result
    }

    /// Apply rules to a single item
    async fn apply_rules_to_item(
        &self,
        mut item: CartItemInput,
        rules: &[PriceRule],
        current_time: i64,
    ) -> CartItemInput {
        // Get product info for category/tag matching
        let product: Option<Product> = match self.db.select(("product", &item.product_id)).await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Failed to fetch product {}: {:?}", item.product_id, e);
                return item;
            }
        };

        let product = match product {
            Some(p) => p,
            None => {
                tracing::warn!(
                    "Product {} not found, skipping price rules",
                    item.product_id
                );
                return item;
            }
        };

        // Get product tags
        let tags = self.get_product_tags(&item.product_id).await;

        // Get category ID (full "table:id" format)
        let category_id = product.category.to_string();

        // Match rules to this product
        let matched_rules = self.match_rules_for_item(
            &item.product_id,
            Some(&category_id),
            &tags,
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

        // Apply adjustments to item
        // Note: We store the surcharge and manual_discount_percent, and let the reducer calculate final price
        if adjustment.surcharge > 0.0 {
            item.surcharge = Some(item.surcharge.unwrap_or(0.0) + adjustment.surcharge);
        }

        // For percentage discount from price rules
        if adjustment.manual_discount_percent > 0.0 {
            // Combine with any existing discount (manual discount from cart)
            let existing = item.manual_discount_percent.unwrap_or(0.0);
            // We need to track price rule discount separately or combine
            // For simplicity, we'll add them together (both are percentages)
            item.manual_discount_percent = Some(existing + adjustment.manual_discount_percent);
        }

        // For fixed discount, we convert to surcharge (negative surcharge = discount)
        // Actually, better to adjust the price directly since fixed discount is absolute
        if adjustment.discount_fixed > 0.0 {
            // Apply fixed discount as negative surcharge
            let current_surcharge = item.surcharge.unwrap_or(0.0);
            item.surcharge = Some(current_surcharge - adjustment.discount_fixed);
        }

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
                if !is_time_valid(rule, current_time) {
                    return false;
                }

                true
            })
            .collect()
    }

    /// Get product tags from specifications
    async fn get_product_tags(&self, product_id: &str) -> Vec<String> {
        // Query product specification for tags
        // This is a simplified version - in production, you'd want to cache this
        match self
            .db
            .query(
                r#"
                SELECT tags FROM product_specification
                WHERE product = type::thing("product", $pid)
                "#,
            )
            .bind(("pid", product_id.to_string()))
            .await
        {
            Ok(mut result) => {
                let tags: Vec<Vec<String>> = result.take(0).unwrap_or_default();
                tags.into_iter().flatten().collect()
            }
            Err(_) => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would go here
    // Requires database setup
}
