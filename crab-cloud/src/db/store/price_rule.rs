//! Price rule database operations

use shared::cloud::store_op::StoreOpData;
use shared::models::price_rule::PriceRuleCreate;
use sqlx::PgPool;

use super::BoxError;

// ── Edge Sync ──

pub async fn upsert_price_rule_from_sync(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &serde_json::Value,
    now: i64,
) -> Result<(), BoxError> {
    let rule: shared::models::PriceRule = serde_json::from_value(data.clone())?;
    let active_days_mask: Option<i32> = rule
        .active_days
        .as_ref()
        .map(|days| days.iter().fold(0i32, |mask, &day| mask | (1 << day)));

    sqlx::query(
        r#"
        INSERT INTO store_price_rules (
            edge_server_id, source_id, name, display_name, receipt_name, description,
            rule_type, product_scope, target_id, zone_scope,
            adjustment_type, adjustment_value, is_stackable, is_exclusive,
            valid_from, valid_until, active_days, active_start_time, active_end_time,
            is_active, created_by, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)
        ON CONFLICT (edge_server_id, source_id)
        DO UPDATE SET
            name = EXCLUDED.name, display_name = EXCLUDED.display_name,
            receipt_name = EXCLUDED.receipt_name, description = EXCLUDED.description,
            rule_type = EXCLUDED.rule_type, product_scope = EXCLUDED.product_scope,
            target_id = EXCLUDED.target_id, zone_scope = EXCLUDED.zone_scope,
            adjustment_type = EXCLUDED.adjustment_type, adjustment_value = EXCLUDED.adjustment_value,
            is_stackable = EXCLUDED.is_stackable, is_exclusive = EXCLUDED.is_exclusive,
            valid_from = EXCLUDED.valid_from, valid_until = EXCLUDED.valid_until,
            active_days = EXCLUDED.active_days, active_start_time = EXCLUDED.active_start_time,
            active_end_time = EXCLUDED.active_end_time, is_active = EXCLUDED.is_active,
            updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(edge_server_id)
    .bind(source_id)
    .bind(&rule.name)
    .bind(&rule.display_name)
    .bind(&rule.receipt_name)
    .bind(&rule.description)
    .bind(serde_json::to_value(&rule.rule_type).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default())
    .bind(serde_json::to_value(&rule.product_scope).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default())
    .bind(rule.target_id)
    .bind(&rule.zone_scope)
    .bind(serde_json::to_value(&rule.adjustment_type).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default())
    .bind(rule.adjustment_value)
    .bind(rule.is_stackable)
    .bind(rule.is_exclusive)
    .bind(rule.valid_from)
    .bind(rule.valid_until)
    .bind(active_days_mask)
    .bind(&rule.active_start_time)
    .bind(&rule.active_end_time)
    .bind(rule.is_active)
    .bind(rule.created_by)
    .bind(rule.created_at)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Console Read ──

#[derive(sqlx::FromRow)]
struct PriceRuleRow {
    source_id: i64,
    name: String,
    display_name: String,
    receipt_name: String,
    description: Option<String>,
    rule_type: String,
    product_scope: String,
    target_id: Option<i64>,
    zone_scope: String,
    adjustment_type: String,
    adjustment_value: f64,
    is_stackable: bool,
    is_exclusive: bool,
    valid_from: Option<i64>,
    valid_until: Option<i64>,
    active_days: Option<i32>,
    active_start_time: Option<String>,
    active_end_time: Option<String>,
    is_active: bool,
    created_by: Option<i64>,
    created_at: i64,
}

impl PriceRuleRow {
    fn into_price_rule(self) -> shared::models::PriceRule {
        use shared::models::price_rule::{AdjustmentType, ProductScope, RuleType};

        shared::models::PriceRule {
            id: self.source_id,
            name: self.name,
            display_name: self.display_name,
            receipt_name: self.receipt_name,
            description: self.description,
            rule_type: serde_json::from_value::<RuleType>(serde_json::Value::String(
                self.rule_type.clone(),
            ))
            .unwrap_or_else(|e| {
                tracing::warn!(rule_type = %self.rule_type, error = %e, "Invalid rule_type, defaulting to Discount");
                RuleType::Discount
            }),
            product_scope: serde_json::from_value::<ProductScope>(serde_json::Value::String(
                self.product_scope.clone(),
            ))
            .unwrap_or_else(|e| {
                tracing::warn!(product_scope = %self.product_scope, error = %e, "Invalid product_scope, defaulting to Global");
                ProductScope::Global
            }),
            target_id: self.target_id,
            zone_scope: self.zone_scope,
            adjustment_type: serde_json::from_value::<AdjustmentType>(
                serde_json::Value::String(self.adjustment_type.clone()),
            )
            .unwrap_or_else(|e| {
                tracing::warn!(adjustment_type = %self.adjustment_type, error = %e, "Invalid adjustment_type, defaulting to Percentage");
                AdjustmentType::Percentage
            }),
            adjustment_value: self.adjustment_value,
            is_stackable: self.is_stackable,
            is_exclusive: self.is_exclusive,
            valid_from: self.valid_from,
            valid_until: self.valid_until,
            active_days: self.active_days.map(|mask| {
                (0..7).filter(|bit| mask & (1 << bit) != 0).collect()
            }),
            active_start_time: self.active_start_time,
            active_end_time: self.active_end_time,
            is_active: self.is_active,
            created_by: self.created_by,
            created_at: self.created_at,
        }
    }
}

pub async fn list_price_rules(
    pool: &PgPool,
    edge_server_id: i64,
) -> Result<Vec<shared::models::PriceRule>, BoxError> {
    let rows = sqlx::query_as::<_, PriceRuleRow>(
        r#"
        SELECT source_id, name, display_name, receipt_name, description,
               rule_type, product_scope, target_id, zone_scope,
               adjustment_type, adjustment_value, is_stackable, is_exclusive,
               valid_from, valid_until, active_days, active_start_time, active_end_time,
               is_active, created_by, created_at
        FROM store_price_rules
        WHERE edge_server_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(edge_server_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into_price_rule()).collect())
}

// ── Console CRUD ──

pub async fn create_price_rule_direct(
    pool: &PgPool,
    edge_server_id: i64,
    data: &PriceRuleCreate,
) -> Result<(i64, StoreOpData), BoxError> {
    let now = shared::util::now_millis();
    let zone_scope = data.zone_scope.as_deref().unwrap_or("all");
    let is_stackable = data.is_stackable.unwrap_or(true);
    let is_exclusive = data.is_exclusive.unwrap_or(false);
    let active_days_mask: Option<i32> = data
        .active_days
        .as_ref()
        .map(|days| days.iter().fold(0i32, |mask, &day| mask | (1 << day)));
    let rule_type_str = serde_json::to_value(&data.rule_type)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    let product_scope_str = serde_json::to_value(&data.product_scope)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    let adjustment_type_str = serde_json::to_value(&data.adjustment_type)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    let mut tx = pool.begin().await?;

    let (pg_id,): (i64,) = sqlx::query_as(
        r#"INSERT INTO store_price_rules (edge_server_id, source_id, name, display_name, receipt_name, description, rule_type, product_scope, target_id, zone_scope, adjustment_type, adjustment_value, is_stackable, is_exclusive, valid_from, valid_until, active_days, active_start_time, active_end_time, is_active, created_by, created_at, updated_at) VALUES ($1, 0, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, TRUE, $19, $20, $20) RETURNING id"#,
    )
    .bind(edge_server_id).bind(&data.name).bind(&data.display_name).bind(&data.receipt_name).bind(&data.description).bind(&rule_type_str).bind(&product_scope_str).bind(data.target_id).bind(zone_scope).bind(&adjustment_type_str).bind(data.adjustment_value).bind(is_stackable).bind(is_exclusive).bind(data.valid_from).bind(data.valid_until).bind(active_days_mask).bind(&data.active_start_time).bind(&data.active_end_time).bind(data.created_by).bind(now)
    .fetch_one(&mut *tx).await?;

    let source_id = super::snowflake_id();
    sqlx::query("UPDATE store_price_rules SET source_id = $1 WHERE id = $2")
        .bind(source_id)
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let rule = shared::models::PriceRule {
        id: source_id,
        name: data.name.clone(),
        display_name: data.display_name.clone(),
        receipt_name: data.receipt_name.clone(),
        description: data.description.clone(),
        rule_type: data.rule_type.clone(),
        product_scope: data.product_scope.clone(),
        target_id: data.target_id,
        zone_scope: zone_scope.to_string(),
        adjustment_type: data.adjustment_type.clone(),
        adjustment_value: data.adjustment_value,
        is_stackable,
        is_exclusive,
        valid_from: data.valid_from,
        valid_until: data.valid_until,
        active_days: data.active_days.clone(),
        active_start_time: data.active_start_time.clone(),
        active_end_time: data.active_end_time.clone(),
        is_active: true,
        created_by: data.created_by,
        created_at: now,
    };
    Ok((source_id, StoreOpData::PriceRule(rule)))
}

pub async fn update_price_rule_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &shared::models::price_rule::PriceRuleUpdate,
) -> Result<(), BoxError> {
    let now = shared::util::now_millis();
    let rule_type_str = data
        .rule_type
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .and_then(|v| v.as_str().map(String::from));
    let product_scope_str = data
        .product_scope
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .and_then(|v| v.as_str().map(String::from));
    let adjustment_type_str = data
        .adjustment_type
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .and_then(|v| v.as_str().map(String::from));
    let active_days_mask: Option<i32> = data
        .active_days
        .as_ref()
        .map(|days| days.iter().fold(0i32, |mask, &day| mask | (1 << day)));

    let rows = sqlx::query("UPDATE store_price_rules SET name = COALESCE($1, name), display_name = COALESCE($2, display_name), receipt_name = COALESCE($3, receipt_name), description = COALESCE($4, description), rule_type = COALESCE($5, rule_type), product_scope = COALESCE($6, product_scope), target_id = COALESCE($7, target_id), zone_scope = COALESCE($8, zone_scope), adjustment_type = COALESCE($9, adjustment_type), adjustment_value = COALESCE($10, adjustment_value), is_stackable = COALESCE($11, is_stackable), is_exclusive = COALESCE($12, is_exclusive), valid_from = COALESCE($13, valid_from), valid_until = COALESCE($14, valid_until), active_days = COALESCE($15, active_days), active_start_time = COALESCE($16, active_start_time), active_end_time = COALESCE($17, active_end_time), is_active = COALESCE($18, is_active), updated_at = $19 WHERE edge_server_id = $20 AND source_id = $21")
        .bind(&data.name).bind(&data.display_name).bind(&data.receipt_name).bind(&data.description).bind(&rule_type_str).bind(&product_scope_str).bind(data.target_id).bind(&data.zone_scope).bind(&adjustment_type_str).bind(data.adjustment_value).bind(data.is_stackable).bind(data.is_exclusive).bind(data.valid_from).bind(data.valid_until).bind(active_days_mask).bind(&data.active_start_time).bind(&data.active_end_time).bind(data.is_active).bind(now).bind(edge_server_id).bind(source_id)
        .execute(pool).await?;
    if rows.rows_affected() == 0 {
        return Err("Price rule not found".into());
    }
    Ok(())
}

pub async fn delete_price_rule_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    let rows =
        sqlx::query("DELETE FROM store_price_rules WHERE edge_server_id = $1 AND source_id = $2")
            .bind(edge_server_id)
            .bind(source_id)
            .execute(pool)
            .await?;
    if rows.rows_affected() == 0 {
        return Err("Price rule not found".into());
    }
    Ok(())
}
