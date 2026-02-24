//! Zone database operations (normalized columns, not JSONB)

use shared::cloud::store_op::StoreOpData;
use shared::models::zone::{Zone, ZoneCreate};
use sqlx::PgPool;

use super::BoxError;

// ── Edge Sync ──

pub async fn upsert_zone_from_sync(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &serde_json::Value,
    now: i64,
) -> Result<(), BoxError> {
    let zone: Zone = serde_json::from_value(data.clone())?;
    sqlx::query(
        r#"
        INSERT INTO store_zones (
            edge_server_id, source_id, name, description, is_active, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (edge_server_id, source_id)
        DO UPDATE SET
            name = EXCLUDED.name, description = EXCLUDED.description,
            is_active = EXCLUDED.is_active, updated_at = EXCLUDED.updated_at
        WHERE store_zones.updated_at <= EXCLUDED.updated_at
        "#,
    )
    .bind(edge_server_id)
    .bind(source_id)
    .bind(&zone.name)
    .bind(&zone.description)
    .bind(zone.is_active)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Console Read ──

pub async fn list_zones(pool: &PgPool, edge_server_id: i64) -> Result<Vec<Zone>, BoxError> {
    let rows: Vec<Zone> = sqlx::query_as(
        r#"
        SELECT source_id AS id, name, description, is_active
        FROM store_zones
        WHERE edge_server_id = $1
        ORDER BY name
        "#,
    )
    .bind(edge_server_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ── Console CRUD ──

pub async fn create_zone_direct(
    pool: &PgPool,
    edge_server_id: i64,
    _tenant_id: &str,
    data: &ZoneCreate,
) -> Result<(i64, StoreOpData), BoxError> {
    let now = shared::util::now_millis();
    let mut tx = pool.begin().await?;

    let (pg_id,): (i64,) = sqlx::query_as(
        r#"
        INSERT INTO store_zones (
            edge_server_id, source_id, name, description, is_active, updated_at
        )
        VALUES ($1, 0, $2, $3, TRUE, $4)
        RETURNING id
        "#,
    )
    .bind(edge_server_id)
    .bind(&data.name)
    .bind(&data.description)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;

    let source_id = super::snowflake_id();
    sqlx::query("UPDATE store_zones SET source_id = $1 WHERE id = $2")
        .bind(source_id)
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let zone = Zone {
        id: source_id,
        name: data.name.clone(),
        description: data.description.clone(),
        is_active: true,
    };
    Ok((source_id, StoreOpData::Zone(zone)))
}

pub async fn update_zone_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &shared::models::zone::ZoneUpdate,
) -> Result<StoreOpData, BoxError> {
    let now = shared::util::now_millis();

    sqlx::query(
        r#"
        UPDATE store_zones SET
            name = COALESCE($1, name),
            description = COALESCE($2, description),
            is_active = COALESCE($3, is_active),
            updated_at = $4
        WHERE edge_server_id = $5 AND source_id = $6
        "#,
    )
    .bind(&data.name)
    .bind(&data.description)
    .bind(data.is_active)
    .bind(now)
    .bind(edge_server_id)
    .bind(source_id)
    .execute(pool)
    .await?;

    let zone: Zone = sqlx::query_as(
        r#"
        SELECT source_id AS id, name, description, is_active
        FROM store_zones
        WHERE edge_server_id = $1 AND source_id = $2
        "#,
    )
    .bind(edge_server_id)
    .bind(source_id)
    .fetch_optional(pool)
    .await?
    .ok_or("Zone not found")?;

    Ok(StoreOpData::Zone(zone))
}

pub async fn delete_zone_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    let rows = sqlx::query("DELETE FROM store_zones WHERE edge_server_id = $1 AND source_id = $2")
        .bind(edge_server_id)
        .bind(source_id)
        .execute(pool)
        .await?;
    if rows.rows_affected() == 0 {
        return Err("Zone not found".into());
    }
    Ok(())
}
