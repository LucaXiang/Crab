//! Store info database operations (edge → cloud sync only, singleton per edge_server)

use shared::models::store_info::StoreInfo;
use sqlx::PgPool;

use super::BoxError;

// ── Edge Sync ──

pub async fn upsert_store_info_from_sync(
    pool: &PgPool,
    edge_server_id: i64,
    data: &serde_json::Value,
    now: i64,
) -> Result<(), BoxError> {
    let info: StoreInfo = serde_json::from_value(data.clone())?;
    sqlx::query(
        r#"
        INSERT INTO store_info (
            edge_server_id, name, address, nif, logo_url,
            phone, email, website, business_day_cutoff,
            created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (edge_server_id)
        DO UPDATE SET
            name = EXCLUDED.name, address = EXCLUDED.address, nif = EXCLUDED.nif,
            logo_url = EXCLUDED.logo_url, phone = EXCLUDED.phone,
            email = EXCLUDED.email, website = EXCLUDED.website,
            business_day_cutoff = EXCLUDED.business_day_cutoff,
            updated_at = EXCLUDED.updated_at
        WHERE store_info.updated_at <= EXCLUDED.updated_at
        "#,
    )
    .bind(edge_server_id)
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

// ── Console Read ──

pub async fn get_store_info(
    pool: &PgPool,
    edge_server_id: i64,
) -> Result<Option<StoreInfo>, BoxError> {
    let row: Option<StoreInfo> = sqlx::query_as(
        r#"
        SELECT 1 AS id, name, address, nif, logo_url, phone, email, website,
               business_day_cutoff, created_at, updated_at
        FROM store_info
        WHERE edge_server_id = $1
        "#,
    )
    .bind(edge_server_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
