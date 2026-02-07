//! Store Info Repository (Singleton)

use super::{RepoError, RepoResult};
use shared::models::{StoreInfo, StoreInfoUpdate};
use sqlx::SqlitePool;

const SINGLETON_ID: i64 = 1;

pub async fn get_or_create(pool: &SqlitePool) -> RepoResult<StoreInfo> {
    if let Some(info) = get(pool).await? {
        return Ok(info);
    }

    // Create singleton with defaults
    let now = shared::util::now_millis();
    sqlx::query(
        "INSERT INTO store_info (id, name, address, nif, business_day_cutoff, created_at, updated_at) VALUES (?, '', '', '', '02:00', ?, ?)",
    )
    .bind(SINGLETON_ID)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    get(pool)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create store info".into()))
}

pub async fn get(pool: &SqlitePool) -> RepoResult<Option<StoreInfo>> {
    let info = sqlx::query_as::<_, StoreInfo>(
        "SELECT id, name, address, nif, logo_url, phone, email, website, business_day_cutoff, created_at, updated_at FROM store_info WHERE id = ?",
    )
    .bind(SINGLETON_ID)
    .fetch_optional(pool)
    .await?;
    Ok(info)
}

pub async fn update(pool: &SqlitePool, data: StoreInfoUpdate) -> RepoResult<StoreInfo> {
    let now = shared::util::now_millis();
    let rows = sqlx::query(
        "UPDATE store_info SET name = COALESCE(?1, name), address = COALESCE(?2, address), nif = COALESCE(?3, nif), logo_url = COALESCE(?4, logo_url), phone = COALESCE(?5, phone), email = COALESCE(?6, email), website = COALESCE(?7, website), business_day_cutoff = COALESCE(?8, business_day_cutoff), updated_at = ?9 WHERE id = ?10",
    )
    .bind(&data.name)
    .bind(&data.address)
    .bind(&data.nif)
    .bind(&data.logo_url)
    .bind(&data.phone)
    .bind(&data.email)
    .bind(&data.website)
    .bind(&data.business_day_cutoff)
    .bind(now)
    .bind(SINGLETON_ID)
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        return Err(RepoError::Database("Failed to update store info".into()));
    }
    get(pool)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to read store info after update".into()))
}
