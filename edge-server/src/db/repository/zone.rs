//! Zone Repository

use super::{RepoError, RepoResult};
use shared::models::{Zone, ZoneCreate, ZoneUpdate};
use sqlx::SqlitePool;

pub async fn find_all(pool: &SqlitePool) -> RepoResult<Vec<Zone>> {
    let zones = sqlx::query_as::<_, Zone>(
        "SELECT id, name, description, is_active FROM zone WHERE is_active = 1 ORDER BY name",
    )
    .fetch_all(pool)
    .await?;
    Ok(zones)
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<Zone>> {
    let zone =
        sqlx::query_as::<_, Zone>("SELECT id, name, description, is_active FROM zone WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    Ok(zone)
}

pub async fn find_by_name(pool: &SqlitePool, name: &str) -> RepoResult<Option<Zone>> {
    let zone = sqlx::query_as::<_, Zone>(
        "SELECT id, name, description, is_active FROM zone WHERE name = ? LIMIT 1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;
    Ok(zone)
}

pub async fn create(pool: &SqlitePool, data: ZoneCreate) -> RepoResult<Zone> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO zone (name, description) VALUES (?, ?) RETURNING id as "id!""#,
        data.name,
        data.description
    )
    .fetch_one(pool)
    .await?;
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create zone".into()))
}

pub async fn update(pool: &SqlitePool, id: i64, data: ZoneUpdate) -> RepoResult<Zone> {
    let rows = sqlx::query!(
        "UPDATE zone SET name = COALESCE(?1, name), description = COALESCE(?2, description), is_active = COALESCE(?3, is_active) WHERE id = ?4",
        data.name,
        data.description,
        data.is_active,
        id
    )
    .execute(pool)
    .await?;
    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!("Zone {id} not found")));
    }
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Zone {id} not found")))
}

pub async fn delete(pool: &SqlitePool, id: i64) -> RepoResult<bool> {
    // Check for active dining tables
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM dining_table WHERE zone_id = ? AND is_active = 1",
        id
    )
    .fetch_one(pool)
    .await?;
    if count > 0 {
        return Err(RepoError::Validation(
            "Cannot delete zone with active tables".into(),
        ));
    }
    sqlx::query!("DELETE FROM zone WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(true)
}
