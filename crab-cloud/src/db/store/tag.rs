//! Tag database operations

use serde::{Deserialize, Serialize};
use shared::cloud::store_op::StoreOpData;
use shared::models::tag::{Tag, TagCreate};
use sqlx::PgPool;

use super::BoxError;

// ── Edge Sync ──

pub async fn upsert_tag_from_sync(
    pool: &PgPool,
    store_id: i64,
    source_id: i64,
    data: &serde_json::Value,
    now: i64,
) -> Result<(), BoxError> {
    let tag: Tag = serde_json::from_value(data.clone())?;
    sqlx::query(
        r#"
        INSERT INTO store_tags (store_id, source_id, name, color, display_order, is_active, is_system, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (store_id, source_id)
        DO UPDATE SET name = EXCLUDED.name, color = EXCLUDED.color,
                      display_order = EXCLUDED.display_order, is_active = EXCLUDED.is_active,
                      is_system = EXCLUDED.is_system, updated_at = EXCLUDED.updated_at
        WHERE store_tags.updated_at <= EXCLUDED.updated_at
        "#,
    )
    .bind(store_id)
    .bind(source_id)
    .bind(&tag.name)
    .bind(&tag.color)
    .bind(tag.display_order)
    .bind(tag.is_active)
    .bind(tag.is_system)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Console Read Types ──

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StoreTag {
    pub source_id: i64,
    pub name: String,
    pub color: String,
    pub display_order: i32,
    pub is_active: bool,
    pub is_system: bool,
}

// ── Console Read ──

pub async fn list_tags(pool: &PgPool, store_id: i64) -> Result<Vec<StoreTag>, BoxError> {
    let rows: Vec<StoreTag> = sqlx::query_as(
        r#"
        SELECT source_id, name, color, display_order, is_active, is_system
        FROM store_tags
        WHERE store_id = $1
        ORDER BY display_order, source_id
        "#,
    )
    .bind(store_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ── Console CRUD ──

pub async fn create_tag_direct(
    pool: &PgPool,
    store_id: i64,
    data: &TagCreate,
) -> Result<(i64, StoreOpData), BoxError> {
    let now = shared::util::now_millis();
    let color = data.color.as_deref().unwrap_or("#3B82F6");
    let display_order = data.display_order.unwrap_or(0);

    let source_id = super::snowflake_id();

    sqlx::query(
        r#"
        INSERT INTO store_tags (store_id, source_id, name, color, display_order, is_active, is_system, updated_at)
        VALUES ($1, $2, $3, $4, $5, TRUE, FALSE, $6)
        "#,
    )
    .bind(store_id)
    .bind(source_id)
    .bind(&data.name)
    .bind(color)
    .bind(display_order)
    .bind(now)
    .execute(pool)
    .await?;

    let tag = Tag {
        id: source_id,
        name: data.name.clone(),
        color: color.to_string(),
        display_order,
        is_active: true,
        is_system: false,
    };
    Ok((source_id, StoreOpData::Tag(tag)))
}

pub async fn update_tag_direct(
    pool: &PgPool,
    store_id: i64,
    source_id: i64,
    data: &shared::models::tag::TagUpdate,
) -> Result<(), BoxError> {
    let now = shared::util::now_millis();
    let rows = sqlx::query("UPDATE store_tags SET name = COALESCE($1, name), color = COALESCE($2, color), display_order = COALESCE($3, display_order), is_active = COALESCE($4, is_active), updated_at = $5 WHERE store_id = $6 AND source_id = $7")
        .bind(&data.name).bind(&data.color).bind(data.display_order).bind(data.is_active).bind(now).bind(store_id).bind(source_id)
        .execute(pool).await?;
    if rows.rows_affected() == 0 {
        return Err("Tag not found".into());
    }
    Ok(())
}

pub async fn delete_tag_direct(
    pool: &PgPool,
    store_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    let rows = sqlx::query("DELETE FROM store_tags WHERE store_id = $1 AND source_id = $2")
        .bind(store_id)
        .bind(source_id)
        .execute(pool)
        .await?;
    if rows.rows_affected() == 0 {
        return Err("Tag not found".into());
    }
    Ok(())
}
