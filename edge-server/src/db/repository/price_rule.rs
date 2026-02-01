//! Price Rule Repository

use super::{BaseRepository, RepoError, RepoResult};
use crate::db::models::{PriceRule, PriceRuleCreate, PriceRuleUpdate, ProductScope};
use surrealdb::engine::local::Db;
use surrealdb::{RecordId, Surreal};

#[derive(Clone)]
pub struct PriceRuleRepository {
    base: BaseRepository,
}

impl PriceRuleRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// Find all active price rules
    pub async fn find_all(&self) -> RepoResult<Vec<PriceRule>> {
        let rules: Vec<PriceRule> = self
            .base
            .db()
            .query("SELECT * FROM price_rule WHERE is_active = true ORDER BY priority DESC")
            .await?
            .take(0)?;
        Ok(rules)
    }

    /// Find active rules by scope
    pub async fn find_by_scope(&self, scope: ProductScope) -> RepoResult<Vec<PriceRule>> {
        let rules: Vec<PriceRule> = self
            .base
            .db()
            .query("SELECT * FROM price_rule WHERE is_active = true AND product_scope = $scope ORDER BY priority DESC")
            .bind(("scope", scope))
            .await?
            .take(0)?;
        Ok(rules)
    }

    /// Find rules applicable to a product (global + category + tag + product-specific)
    pub async fn find_for_product(&self, product_id: &str) -> RepoResult<Vec<PriceRule>> {
        let pid_owned = product_id.to_string();
        let mut result = self
            .base
            .db()
            .query(
                r#"
                LET $prod = type::thing("product", $pid);
                LET $product = (SELECT * FROM product WHERE id = $prod)[0];
                LET $cat = $product.category;
                LET $tags = $product.tags;

                SELECT * FROM price_rule
                WHERE is_active = true AND (
                    product_scope = "GLOBAL" OR
                    (product_scope = "PRODUCT" AND target = $prod) OR
                    (product_scope = "CATEGORY" AND target = $cat) OR
                    (product_scope = "TAG" AND target IN $tags)
                )
                ORDER BY priority DESC;
                "#,
            )
            .bind(("pid", pid_owned))
            .await?;
        let rules: Vec<PriceRule> = result.take(0)?;
        Ok(rules)
    }

    /// Find rule by id
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<PriceRule>> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;
        let rule: Option<PriceRule> = self.base.db().select(thing).await?;
        Ok(rule)
    }

    /// Find rule by name
    pub async fn find_by_name(&self, name: &str) -> RepoResult<Option<PriceRule>> {
        let name_owned = name.to_string();
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM price_rule WHERE name = $name LIMIT 1")
            .bind(("name", name_owned))
            .await?;
        let rules: Vec<PriceRule> = result.take(0)?;
        Ok(rules.into_iter().next())
    }

    /// Create a new price rule
    pub async fn create(&self, data: PriceRuleCreate) -> RepoResult<PriceRule> {
        // Check duplicate name
        if self.find_by_name(&data.name).await?.is_some() {
            return Err(RepoError::Duplicate(format!(
                "Price rule '{}' already exists",
                data.name
            )));
        }

        // Convert target string to RecordId if provided
        let target_thing: Option<RecordId> = data.target.as_ref().and_then(|t| t.parse().ok());

        let mut result = self
            .base
            .db()
            .query(
                r#"CREATE price_rule SET
                    name = $name,
                    display_name = $display_name,
                    receipt_name = $receipt_name,
                    description = $description,
                    rule_type = $rule_type,
                    product_scope = $product_scope,
                    target = $target,
                    zone_scope = $zone_scope,
                    adjustment_type = $adjustment_type,
                    adjustment_value = $adjustment_value,
                    priority = $priority,
                    is_stackable = $is_stackable,
                    is_exclusive = $is_exclusive,
                    valid_from = $valid_from,
                    valid_until = $valid_until,
                    active_days = $active_days,
                    active_start_time = $active_start_time,
                    active_end_time = $active_end_time,
                    is_active = true,
                    created_by = $created_by,
                    created_at = $now
                RETURN AFTER"#,
            )
            .bind(("name", data.name))
            .bind(("display_name", data.display_name))
            .bind(("receipt_name", data.receipt_name))
            .bind(("description", data.description))
            .bind(("rule_type", data.rule_type))
            .bind(("product_scope", data.product_scope))
            .bind(("target", target_thing))
            .bind(("zone_scope", data.zone_scope.unwrap_or_else(|| crate::db::models::ZONE_SCOPE_ALL.to_string())))
            .bind(("adjustment_type", data.adjustment_type))
            .bind(("adjustment_value", data.adjustment_value))
            .bind(("priority", data.priority.unwrap_or(0)))
            .bind(("is_stackable", data.is_stackable.unwrap_or(true)))
            .bind(("is_exclusive", data.is_exclusive.unwrap_or(false)))
            .bind(("valid_from", data.valid_from))
            .bind(("valid_until", data.valid_until))
            .bind(("active_days", data.active_days))
            .bind(("active_start_time", data.active_start_time))
            .bind(("active_end_time", data.active_end_time))
            .bind(("created_by", data.created_by))
            .bind(("now", shared::util::now_millis()))
            .await?;

        let created: Option<PriceRule> = result.take(0)?;
        created.ok_or_else(|| RepoError::Database("Failed to create price rule".to_string()))
    }

    /// Update a price rule
    pub async fn update(&self, id: &str, data: PriceRuleUpdate) -> RepoResult<PriceRule> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;
        let existing = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepoError::NotFound(format!("Price rule {} not found", id)))?;

        // Check duplicate name if changing
        if let Some(ref new_name) = data.name
            && new_name != &existing.name
            && self.find_by_name(new_name).await?.is_some()
        {
            return Err(RepoError::Duplicate(format!(
                "Price rule '{}' already exists",
                new_name
            )));
        }

        // Convert target string to RecordId if provided
        let target_thing: Option<RecordId> = data.target.as_ref().and_then(|t| t.parse().ok());

        let mut result = self.base
            .db()
            .query(
                r#"UPDATE $thing SET
                    name = $name OR name,
                    display_name = $display_name OR display_name,
                    receipt_name = $receipt_name OR receipt_name,
                    description = $description OR description,
                    rule_type = $rule_type OR rule_type,
                    product_scope = $product_scope OR product_scope,
                    target = IF $has_target THEN $target ELSE target END,
                    zone_scope = $zone_scope OR zone_scope,
                    adjustment_type = $adjustment_type OR adjustment_type,
                    adjustment_value = IF $has_adj_value THEN $adjustment_value ELSE adjustment_value END,
                    priority = $priority OR priority,
                    is_stackable = IF $has_stackable THEN $is_stackable ELSE is_stackable END,
                    is_exclusive = IF $has_exclusive THEN $is_exclusive ELSE is_exclusive END,
                    valid_from = IF $has_valid_from THEN $valid_from ELSE valid_from END,
                    valid_until = IF $has_valid_until THEN $valid_until ELSE valid_until END,
                    active_days = IF $has_active_days THEN $active_days ELSE active_days END,
                    active_start_time = $active_start_time OR active_start_time,
                    active_end_time = $active_end_time OR active_end_time,
                    is_active = IF $has_is_active THEN $is_active ELSE is_active END
                RETURN AFTER"#,
            )
            .bind(("thing", thing))
            .bind(("name", data.name))
            .bind(("display_name", data.display_name))
            .bind(("receipt_name", data.receipt_name))
            .bind(("description", data.description))
            .bind(("rule_type", data.rule_type))
            .bind(("product_scope", data.product_scope))
            .bind(("has_target", data.target.is_some()))
            .bind(("target", target_thing))
            .bind(("zone_scope", data.zone_scope))
            .bind(("adjustment_type", data.adjustment_type))
            .bind(("has_adj_value", data.adjustment_value.is_some()))
            .bind(("adjustment_value", data.adjustment_value))
            .bind(("priority", data.priority))
            .bind(("has_stackable", data.is_stackable.is_some()))
            .bind(("is_stackable", data.is_stackable))
            .bind(("has_exclusive", data.is_exclusive.is_some()))
            .bind(("is_exclusive", data.is_exclusive))
            .bind(("has_valid_from", data.valid_from.is_some()))
            .bind(("valid_from", data.valid_from))
            .bind(("has_valid_until", data.valid_until.is_some()))
            .bind(("valid_until", data.valid_until))
            .bind(("has_active_days", data.active_days.is_some()))
            .bind(("active_days", data.active_days))
            .bind(("active_start_time", data.active_start_time))
            .bind(("active_end_time", data.active_end_time))
            .bind(("has_is_active", data.is_active.is_some()))
            .bind(("is_active", data.is_active))
            .await?;

        result.take::<Option<PriceRule>>(0)?
            .ok_or_else(|| RepoError::NotFound(format!("Price rule {} not found", id)))
    }

    /// Hard delete a price rule
    pub async fn delete(&self, id: &str) -> RepoResult<bool> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;
        self.base
            .db()
            .query("DELETE $thing")
            .bind(("thing", thing))
            .await?;
        Ok(true)
    }
}
