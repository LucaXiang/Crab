//! Dining table database operations (normalized columns, not JSONB)

use shared::cloud::store_op::StoreOpData;
use shared::models::dining_table::{DiningTable, DiningTableCreate};
use sqlx::PgPool;

use super::BoxError;

// ── Edge Sync ──

pub async fn upsert_dining_table_from_sync(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &serde_json::Value,
    now: i64,
) -> Result<(), BoxError> {
    let table: DiningTable = serde_json::from_value(data.clone())?;
    sqlx::query(
        r#"
        INSERT INTO store_dining_tables (
            edge_server_id, source_id, name, zone_source_id, capacity, is_active, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (edge_server_id, source_id)
        DO UPDATE SET
            name = EXCLUDED.name, zone_source_id = EXCLUDED.zone_source_id,
            capacity = EXCLUDED.capacity, is_active = EXCLUDED.is_active,
            updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(edge_server_id)
    .bind(source_id)
    .bind(&table.name)
    .bind(table.zone_id)
    .bind(table.capacity)
    .bind(table.is_active)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Console Read ──

pub async fn list_tables(pool: &PgPool, edge_server_id: i64) -> Result<Vec<DiningTable>, BoxError> {
    let rows: Vec<DiningTable> = sqlx::query_as(
        r#"
        SELECT source_id AS id, name, zone_source_id AS zone_id, capacity, is_active
        FROM store_dining_tables
        WHERE edge_server_id = $1
        ORDER BY zone_source_id, name
        "#,
    )
    .bind(edge_server_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ── Console CRUD ──

pub async fn create_table_direct(
    pool: &PgPool,
    edge_server_id: i64,
    _tenant_id: &str,
    data: &DiningTableCreate,
) -> Result<(i64, StoreOpData), BoxError> {
    let now = shared::util::now_millis();
    let capacity = data.capacity.unwrap_or(4);
    let mut tx = pool.begin().await?;

    let (pg_id,): (i64,) = sqlx::query_as(
        r#"
        INSERT INTO store_dining_tables (
            edge_server_id, source_id, name, zone_source_id, capacity, is_active, updated_at
        )
        VALUES ($1, 0, $2, $3, $4, TRUE, $5)
        RETURNING id
        "#,
    )
    .bind(edge_server_id)
    .bind(&data.name)
    .bind(data.zone_id)
    .bind(capacity)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;

    let source_id = super::snowflake_id();
    sqlx::query("UPDATE store_dining_tables SET source_id = $1 WHERE id = $2")
        .bind(source_id)
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let table = DiningTable {
        id: source_id,
        name: data.name.clone(),
        zone_id: data.zone_id,
        capacity,
        is_active: true,
    };
    Ok((source_id, StoreOpData::Table(table)))
}

pub async fn update_table_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &shared::models::dining_table::DiningTableUpdate,
) -> Result<StoreOpData, BoxError> {
    let now = shared::util::now_millis();

    sqlx::query(
        r#"
        UPDATE store_dining_tables SET
            name = COALESCE($1, name),
            zone_source_id = COALESCE($2, zone_source_id),
            capacity = COALESCE($3, capacity),
            is_active = COALESCE($4, is_active),
            updated_at = $5
        WHERE edge_server_id = $6 AND source_id = $7
        "#,
    )
    .bind(&data.name)
    .bind(data.zone_id)
    .bind(data.capacity)
    .bind(data.is_active)
    .bind(now)
    .bind(edge_server_id)
    .bind(source_id)
    .execute(pool)
    .await?;

    let table: DiningTable = sqlx::query_as(
        r#"
        SELECT source_id AS id, name, zone_source_id AS zone_id, capacity, is_active
        FROM store_dining_tables
        WHERE edge_server_id = $1 AND source_id = $2
        "#,
    )
    .bind(edge_server_id)
    .bind(source_id)
    .fetch_optional(pool)
    .await?
    .ok_or("Dining table not found")?;

    Ok(StoreOpData::Table(table))
}

pub async fn delete_table_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    let rows =
        sqlx::query("DELETE FROM store_dining_tables WHERE edge_server_id = $1 AND source_id = $2")
            .bind(edge_server_id)
            .bind(source_id)
            .execute(pool)
            .await?;
    if rows.rows_affected() == 0 {
        return Err("Dining table not found".into());
    }
    Ok(())
}
