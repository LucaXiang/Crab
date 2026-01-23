//! Price Rule Repository

use super::{BaseRepository, RepoError, RepoResult, make_thing, strip_table_prefix, parse_thing};
use crate::db::models::{PriceRule, PriceRuleCreate, PriceRuleUpdate, ProductScope, TimeMode};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

const TABLE: &str = "price_rule";

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
                LET $tags = (SELECT tags FROM product_specification WHERE product = $prod).tags;

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
        let pure_id = strip_table_prefix(TABLE, id);
        let rule: Option<PriceRule> = self.base.db().select((TABLE, pure_id)).await?;
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

        // Convert target string to Thing if provided
        let target_thing = data.target.as_ref().and_then(|t| parse_thing(t));

        let rule = PriceRule {
            id: None,
            name: data.name,
            display_name: data.display_name,
            receipt_name: data.receipt_name,
            description: data.description,
            rule_type: data.rule_type,
            product_scope: data.product_scope,
            target: target_thing,
            zone_scope: data.zone_scope.clone().unwrap_or_else(|| crate::db::models::ZONE_SCOPE_ALL.to_string()),
            adjustment_type: data.adjustment_type,
            adjustment_value: data.adjustment_value,
            priority: data.priority.unwrap_or(0),
            is_stackable: data.is_stackable.unwrap_or(true),
            is_exclusive: data.is_exclusive.unwrap_or(false),
            time_mode: data.time_mode.unwrap_or(TimeMode::Always),
            start_time: data.start_time,
            end_time: data.end_time,
            schedule_config: data.schedule_config,
            valid_from: data.valid_from,
            valid_until: data.valid_until,
            active_days: data.active_days,
            active_start_time: data.active_start_time,
            active_end_time: data.active_end_time,
            is_active: true,
            created_by: data.created_by,
            created_at: chrono::Utc::now().timestamp_millis(),
        };

        let created: Option<PriceRule> = self.base.db().create(TABLE).content(rule).await?;
        created.ok_or_else(|| RepoError::Database("Failed to create price rule".to_string()))
    }

    /// Update a price rule
    pub async fn update(&self, id: &str, data: PriceRuleUpdate) -> RepoResult<PriceRule> {
        let pure_id = strip_table_prefix(TABLE, id);
        let existing = self
            .find_by_id(pure_id)
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

        // Convert target string to Thing if provided
        let target_thing = data.target.as_ref().and_then(|t| parse_thing(t));

        // Create internal update struct with Thing type for target
        #[derive(serde::Serialize)]
        struct InternalUpdate {
            #[serde(skip_serializing_if = "Option::is_none")]
            name: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            display_name: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            receipt_name: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            description: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            rule_type: Option<crate::db::models::RuleType>,
            #[serde(skip_serializing_if = "Option::is_none")]
            product_scope: Option<ProductScope>,
            #[serde(skip_serializing_if = "Option::is_none")]
            target: Option<surrealdb::sql::Thing>,
            #[serde(skip_serializing_if = "Option::is_none")]
            zone_scope: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            adjustment_type: Option<crate::db::models::AdjustmentType>,
            #[serde(skip_serializing_if = "Option::is_none")]
            adjustment_value: Option<f64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            priority: Option<i32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_stackable: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_exclusive: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            time_mode: Option<TimeMode>,
            #[serde(skip_serializing_if = "Option::is_none")]
            start_time: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            end_time: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            schedule_config: Option<crate::db::models::ScheduleConfig>,
            #[serde(skip_serializing_if = "Option::is_none")]
            valid_from: Option<i64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            valid_until: Option<i64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            active_days: Option<Vec<u8>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            active_start_time: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            active_end_time: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_active: Option<bool>,
        }

        let internal = InternalUpdate {
            name: data.name,
            display_name: data.display_name,
            receipt_name: data.receipt_name,
            description: data.description,
            rule_type: data.rule_type,
            product_scope: data.product_scope,
            target: target_thing,
            zone_scope: data.zone_scope,
            adjustment_type: data.adjustment_type,
            adjustment_value: data.adjustment_value,
            priority: data.priority,
            is_stackable: data.is_stackable,
            is_exclusive: data.is_exclusive,
            time_mode: data.time_mode,
            start_time: data.start_time,
            end_time: data.end_time,
            schedule_config: data.schedule_config,
            valid_from: data.valid_from,
            valid_until: data.valid_until,
            active_days: data.active_days,
            active_start_time: data.active_start_time,
            active_end_time: data.active_end_time,
            is_active: data.is_active,
        };

        let thing = make_thing(TABLE, pure_id);
        self.base
            .db()
            .query("UPDATE $thing MERGE $data")
            .bind(("thing", thing))
            .bind(("data", internal))
            .await?;

        self.find_by_id(pure_id)
            .await?
            .ok_or_else(|| RepoError::NotFound(format!("Price rule {} not found", id)))
    }

    /// Hard delete a price rule
    pub async fn delete(&self, id: &str) -> RepoResult<bool> {
        let pure_id = strip_table_prefix(TABLE, id);
        let thing = make_thing(TABLE, pure_id);
        self.base
            .db()
            .query("DELETE $thing")
            .bind(("thing", thing))
            .await?;
        Ok(true)
    }
}
