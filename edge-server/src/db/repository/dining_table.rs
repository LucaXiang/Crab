//! Dining Table Repository

use super::{RepoError, RepoResult};
use shared::models::{DiningTable, DiningTableCreate, DiningTableUpdate};
use sqlx::SqlitePool;

pub async fn find_all(pool: &SqlitePool) -> RepoResult<Vec<DiningTable>> {
    let tables = sqlx::query_as::<_, DiningTable>(
        "SELECT id, name, zone_id, capacity, is_active FROM dining_table WHERE is_active = 1 ORDER BY name",
    )
    .fetch_all(pool)
    .await?;
    Ok(tables)
}

pub async fn find_all_with_inactive(pool: &SqlitePool) -> RepoResult<Vec<DiningTable>> {
    let tables = sqlx::query_as::<_, DiningTable>(
        "SELECT id, name, zone_id, capacity, is_active FROM dining_table ORDER BY name",
    )
    .fetch_all(pool)
    .await?;
    Ok(tables)
}

pub async fn find_by_zone(pool: &SqlitePool, zone_id: i64) -> RepoResult<Vec<DiningTable>> {
    let tables = sqlx::query_as::<_, DiningTable>(
        "SELECT id, name, zone_id, capacity, is_active FROM dining_table WHERE zone_id = ? AND is_active = 1 ORDER BY name",
    )
    .bind(zone_id)
    .fetch_all(pool)
    .await?;
    Ok(tables)
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<DiningTable>> {
    let table = sqlx::query_as::<_, DiningTable>(
        "SELECT id, name, zone_id, capacity, is_active FROM dining_table WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(table)
}

pub async fn find_by_name_in_zone(
    pool: &SqlitePool,
    zone_id: i64,
    name: &str,
) -> RepoResult<Option<DiningTable>> {
    let table = sqlx::query_as::<_, DiningTable>(
        "SELECT id, name, zone_id, capacity, is_active FROM dining_table WHERE zone_id = ? AND name = ? LIMIT 1",
    )
    .bind(zone_id)
    .bind(name)
    .fetch_optional(pool)
    .await?;
    Ok(table)
}

pub async fn create(
    pool: &SqlitePool,
    assigned_id: Option<i64>,
    data: DiningTableCreate,
) -> RepoResult<DiningTable> {
    let capacity = data.capacity.unwrap_or(4);
    let id = assigned_id.unwrap_or_else(shared::util::snowflake_id);
    let now = shared::util::now_millis();
    sqlx::query(
        "INSERT INTO dining_table (id, name, zone_id, capacity, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(&data.name)
    .bind(data.zone_id)
    .bind(capacity)
    .bind(now)
    .execute(pool)
    .await?;
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create dining table".into()))
}

pub async fn update(
    pool: &SqlitePool,
    id: i64,
    data: DiningTableUpdate,
) -> RepoResult<DiningTable> {
    let now = shared::util::now_millis();
    let rows = sqlx::query!(
        "UPDATE dining_table SET name = COALESCE(?1, name), zone_id = COALESCE(?2, zone_id), capacity = COALESCE(?3, capacity), is_active = COALESCE(?4, is_active), updated_at = ?5 WHERE id = ?6",
        data.name,
        data.zone_id,
        data.capacity,
        data.is_active,
        now,
        id
    )
    .execute(pool)
    .await?;
    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!("Dining table {id} not found")));
    }
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Dining table {id} not found")))
}

pub async fn delete(pool: &SqlitePool, id: i64) -> RepoResult<bool> {
    sqlx::query!("DELETE FROM dining_table WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(true)
}
