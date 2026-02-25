//! Tag Repository

use super::{RepoError, RepoResult};
use shared::models::{Tag, TagCreate, TagUpdate};
use sqlx::SqlitePool;

pub async fn find_all(pool: &SqlitePool) -> RepoResult<Vec<Tag>> {
    let tags = sqlx::query_as::<_, Tag>(
        "SELECT id, name, color, display_order, is_active, is_system FROM tag WHERE is_active = 1 ORDER BY display_order",
    )
    .fetch_all(pool)
    .await?;
    Ok(tags)
}

pub async fn find_all_with_inactive(pool: &SqlitePool) -> RepoResult<Vec<Tag>> {
    let tags = sqlx::query_as::<_, Tag>(
        "SELECT id, name, color, display_order, is_active, is_system FROM tag ORDER BY display_order",
    )
    .fetch_all(pool)
    .await?;
    Ok(tags)
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<Tag>> {
    let tag = sqlx::query_as::<_, Tag>(
        "SELECT id, name, color, display_order, is_active, is_system FROM tag WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(tag)
}

pub async fn find_by_name(pool: &SqlitePool, name: &str) -> RepoResult<Option<Tag>> {
    let tag = sqlx::query_as::<_, Tag>(
        "SELECT id, name, color, display_order, is_active, is_system FROM tag WHERE name = ? LIMIT 1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;
    Ok(tag)
}

pub async fn create(
    pool: &SqlitePool,
    assigned_id: Option<i64>,
    data: TagCreate,
) -> RepoResult<Tag> {
    let color = data.color.unwrap_or_else(|| "#3B82F6".to_string());
    let display_order = data.display_order.unwrap_or(0);
    let id = assigned_id.unwrap_or_else(shared::util::snowflake_id);
    sqlx::query("INSERT INTO tag (id, name, color, display_order) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind(&data.name)
        .bind(&color)
        .bind(display_order)
        .execute(pool)
        .await?;
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create tag".into()))
}

pub async fn update(pool: &SqlitePool, id: i64, data: TagUpdate) -> RepoResult<Tag> {
    let rows = sqlx::query!(
        "UPDATE tag SET name = COALESCE(?1, name), color = COALESCE(?2, color), display_order = COALESCE(?3, display_order), is_active = COALESCE(?4, is_active) WHERE id = ?5",
        data.name,
        data.color,
        data.display_order,
        data.is_active,
        id
    )
    .execute(pool)
    .await?;
    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!("Tag {id} not found")));
    }
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Tag {id} not found")))
}

pub async fn delete(pool: &SqlitePool, id: i64) -> RepoResult<bool> {
    sqlx::query!("DELETE FROM tag WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(true)
}
