//! Store info database operations
//!
//! After merging edge_servers + store_info → stores, these operate directly on the `stores` table.

use shared::models::store_info::StoreInfo;
use sqlx::PgPool;

use super::BoxError;

// ── Edge Sync ──

pub async fn upsert_store_info_from_sync(
    pool: &PgPool,
    store_id: i64,
    data: &serde_json::Value,
    now: i64,
) -> Result<(), BoxError> {
    let info: StoreInfo = serde_json::from_value(data.clone())?;
    sqlx::query(
        r#"
        UPDATE stores SET
            name = $2, address = $3, nif = $4,
            logo_url = $5, phone = $6,
            email = $7, website = $8,
            business_day_cutoff = $9,
            created_at = COALESCE(created_at, $10),
            updated_at = $11
        WHERE id = $1 AND (updated_at IS NULL OR updated_at <= $11)
        "#,
    )
    .bind(store_id)
    .bind(&info.name)
    .bind(&info.address)
    .bind(&info.nif)
    .bind(&info.logo_url)
    .bind(&info.phone)
    .bind(&info.email)
    .bind(&info.website)
    .bind(&info.business_day_cutoff)
    .bind(info.created_at)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Console Write ──

pub async fn update_store_info_direct(
    pool: &PgPool,
    store_id: i64,
    data: &shared::models::store_info::StoreInfoUpdate,
) -> Result<StoreInfo, BoxError> {
    let now = shared::util::now_millis();

    let info: StoreInfo = sqlx::query_as(
        r#"
        UPDATE stores SET
            name = COALESCE($2, name),
            address = COALESCE($3, address),
            nif = COALESCE($4, nif),
            logo_url = COALESCE($5, logo_url),
            phone = COALESCE($6, phone),
            email = COALESCE($7, email),
            website = COALESCE($8, website),
            business_day_cutoff = COALESCE($9, business_day_cutoff),
            updated_at = $10
        WHERE id = $1
        RETURNING 1::BIGINT AS id, name, address, nif, logo_url, phone, email, website, business_day_cutoff, created_at, updated_at
        "#,
    )
    .bind(store_id)
    .bind(&data.name)
    .bind(&data.address)
    .bind(&data.nif)
    .bind(&data.logo_url)
    .bind(&data.phone)
    .bind(&data.email)
    .bind(&data.website)
    .bind(&data.business_day_cutoff)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(info)
}

/// Update store alias (cloud-only field, not synced from edge)
pub async fn update_store_alias(pool: &PgPool, store_id: i64, alias: &str) -> Result<(), BoxError> {
    sqlx::query("UPDATE stores SET alias = $2 WHERE id = $1")
        .bind(store_id)
        .bind(alias)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Console Read ──

pub async fn get_store_info(pool: &PgPool, store_id: i64) -> Result<Option<StoreInfo>, BoxError> {
    let row: Option<StoreInfo> = sqlx::query_as(
        r#"
        SELECT 1::BIGINT AS id, name, address, nif, logo_url, phone, email, website,
               business_day_cutoff, created_at, updated_at
        FROM stores
        WHERE id = $1
        "#,
    )
    .bind(store_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
