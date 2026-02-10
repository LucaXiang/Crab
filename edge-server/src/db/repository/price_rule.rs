//! Price Rule Repository

use super::{RepoError, RepoResult};
use shared::models::{PriceRule, PriceRuleCreate, PriceRuleUpdate, ProductScope, ZONE_SCOPE_ALL};
use sqlx::SqlitePool;

pub async fn find_all(pool: &SqlitePool) -> RepoResult<Vec<PriceRule>> {
    let rules = sqlx::query_as::<_, PriceRule>(
        "SELECT id, name, display_name, receipt_name, description, rule_type, product_scope, target_id, zone_scope, adjustment_type, adjustment_value, is_stackable, is_exclusive, valid_from, valid_until, COALESCE(active_days, 'null') as active_days, active_start_time, active_end_time, is_active, created_by, created_at FROM price_rule WHERE is_active = 1 ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rules)
}

/// Find active rules by zone scope (DB-level filtering)
pub async fn find_by_zone(
    pool: &SqlitePool,
    zone_id: Option<i64>,
    is_retail: bool,
) -> RepoResult<Vec<PriceRule>> {
    let zone_id_str = zone_id.map(|id| id.to_string()).unwrap_or_default();
    let rules = sqlx::query_as::<_, PriceRule>(
        "SELECT id, name, display_name, receipt_name, description, rule_type, product_scope, target_id, zone_scope, adjustment_type, adjustment_value, is_stackable, is_exclusive, valid_from, valid_until, COALESCE(active_days, 'null') as active_days, active_start_time, active_end_time, is_active, created_by, created_at FROM price_rule WHERE is_active = 1 AND (zone_scope = 'all' OR (zone_scope = 'retail' AND ?1 = 1) OR zone_scope = ?2) ORDER BY created_at DESC",
    )
    .bind(is_retail)
    .bind(&zone_id_str)
    .fetch_all(pool)
    .await?;
    Ok(rules)
}

pub async fn find_by_scope(pool: &SqlitePool, scope: ProductScope) -> RepoResult<Vec<PriceRule>> {
    let rules = sqlx::query_as::<_, PriceRule>(
        "SELECT id, name, display_name, receipt_name, description, rule_type, product_scope, target_id, zone_scope, adjustment_type, adjustment_value, is_stackable, is_exclusive, valid_from, valid_until, COALESCE(active_days, 'null') as active_days, active_start_time, active_end_time, is_active, created_by, created_at FROM price_rule WHERE is_active = 1 AND product_scope = ? ORDER BY created_at DESC",
    )
    .bind(scope)
    .fetch_all(pool)
    .await?;
    Ok(rules)
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<PriceRule>> {
    let rule = sqlx::query_as::<_, PriceRule>(
        "SELECT id, name, display_name, receipt_name, description, rule_type, product_scope, target_id, zone_scope, adjustment_type, adjustment_value, is_stackable, is_exclusive, valid_from, valid_until, COALESCE(active_days, 'null') as active_days, active_start_time, active_end_time, is_active, created_by, created_at FROM price_rule WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(rule)
}

pub async fn find_by_name(pool: &SqlitePool, name: &str) -> RepoResult<Option<PriceRule>> {
    let rule = sqlx::query_as::<_, PriceRule>(
        "SELECT id, name, display_name, receipt_name, description, rule_type, product_scope, target_id, zone_scope, adjustment_type, adjustment_value, is_stackable, is_exclusive, valid_from, valid_until, COALESCE(active_days, 'null') as active_days, active_start_time, active_end_time, is_active, created_by, created_at FROM price_rule WHERE name = ? LIMIT 1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;
    Ok(rule)
}

pub async fn create(pool: &SqlitePool, data: PriceRuleCreate) -> RepoResult<PriceRule> {
    let now = shared::util::now_millis();
    let zone_scope = data.zone_scope.unwrap_or_else(|| ZONE_SCOPE_ALL.to_string());
    let active_days_json = data
        .active_days
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string()));

    let is_stackable = data.is_stackable.unwrap_or(true);
    let is_exclusive = data.is_exclusive.unwrap_or(false);
    let id = sqlx::query_scalar!(
        r#"INSERT INTO price_rule (name, display_name, receipt_name, description, rule_type, product_scope, target_id, zone_scope, adjustment_type, adjustment_value, is_stackable, is_exclusive, valid_from, valid_until, active_days, active_start_time, active_end_time, is_active, created_by, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, 1, ?18, ?19) RETURNING id as "id!""#,
        data.name,
        data.display_name,
        data.receipt_name,
        data.description,
        data.rule_type,
        data.product_scope,
        data.target_id,
        zone_scope,
        data.adjustment_type,
        data.adjustment_value,
        is_stackable,
        is_exclusive,
        data.valid_from,
        data.valid_until,
        active_days_json,
        data.active_start_time,
        data.active_end_time,
        data.created_by,
        now
    )
    .fetch_one(pool)
    .await?;

    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create price rule".into()))
}

pub async fn update(pool: &SqlitePool, id: i64, data: PriceRuleUpdate) -> RepoResult<PriceRule> {
    let active_days_json = data
        .active_days
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string()));

    let rows = sqlx::query!(
        "UPDATE price_rule SET name = COALESCE(?1, name), display_name = COALESCE(?2, display_name), receipt_name = COALESCE(?3, receipt_name), description = COALESCE(?4, description), rule_type = COALESCE(?5, rule_type), product_scope = COALESCE(?6, product_scope), target_id = COALESCE(?7, target_id), zone_scope = COALESCE(?8, zone_scope), adjustment_type = COALESCE(?9, adjustment_type), adjustment_value = COALESCE(?10, adjustment_value), is_stackable = COALESCE(?11, is_stackable), is_exclusive = COALESCE(?12, is_exclusive), valid_from = COALESCE(?13, valid_from), valid_until = COALESCE(?14, valid_until), active_days = COALESCE(?15, active_days), active_start_time = COALESCE(?16, active_start_time), active_end_time = COALESCE(?17, active_end_time), is_active = COALESCE(?18, is_active) WHERE id = ?19",
        data.name,
        data.display_name,
        data.receipt_name,
        data.description,
        data.rule_type,
        data.product_scope,
        data.target_id,
        data.zone_scope,
        data.adjustment_type,
        data.adjustment_value,
        data.is_stackable,
        data.is_exclusive,
        data.valid_from,
        data.valid_until,
        active_days_json,
        data.active_start_time,
        data.active_end_time,
        data.is_active,
        id
    )
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!("Price rule {id} not found")));
    }
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Price rule {id} not found")))
}

pub async fn delete(pool: &SqlitePool, id: i64) -> RepoResult<bool> {
    sqlx::query!("DELETE FROM price_rule WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(true)
}
