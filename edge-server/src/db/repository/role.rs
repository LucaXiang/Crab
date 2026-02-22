//! Role Repository

use super::{RepoError, RepoResult};
use shared::error::ErrorCode;
use shared::models::{Role, RoleCreate, RoleUpdate};
use sqlx::SqlitePool;

pub async fn find_all(pool: &SqlitePool) -> RepoResult<Vec<Role>> {
    let roles = sqlx::query_as::<_, Role>(
        "SELECT id, name, display_name, description, permissions, is_system, is_active FROM role WHERE is_active = 1 ORDER BY name",
    )
    .fetch_all(pool)
    .await?;
    Ok(roles)
}

pub async fn find_all_with_inactive(pool: &SqlitePool) -> RepoResult<Vec<Role>> {
    let roles = sqlx::query_as::<_, Role>(
        "SELECT id, name, display_name, description, permissions, is_system, is_active FROM role ORDER BY name",
    )
    .fetch_all(pool)
    .await?;
    Ok(roles)
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<Role>> {
    let role = sqlx::query_as::<_, Role>(
        "SELECT id, name, display_name, description, permissions, is_system, is_active FROM role WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(role)
}

pub async fn find_by_name(pool: &SqlitePool, name: &str) -> RepoResult<Option<Role>> {
    let role = sqlx::query_as::<_, Role>(
        "SELECT id, name, display_name, description, permissions, is_system, is_active FROM role WHERE name = ? LIMIT 1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;
    Ok(role)
}

pub async fn create(pool: &SqlitePool, data: RoleCreate) -> RepoResult<Role> {
    let display_name = data.display_name.unwrap_or_else(|| data.name.clone());
    let permissions_json =
        serde_json::to_string(&data.permissions).unwrap_or_else(|_| "[]".to_string());

    let id = sqlx::query_scalar!(
        r#"INSERT INTO role (name, display_name, description, permissions) VALUES (?, ?, ?, ?) RETURNING id as "id!""#,
        data.name,
        display_name,
        data.description,
        permissions_json
    )
    .fetch_one(pool)
    .await?;

    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create role".into()))
}

pub async fn update(pool: &SqlitePool, id: i64, data: RoleUpdate) -> RepoResult<Role> {
    // Check is_system flag
    let existing = find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Role {id} not found")))?;

    if existing.is_system {
        return Err(RepoError::Business(
            ErrorCode::RoleIsSystem,
            "Cannot modify system role".into(),
        ));
    }

    let permissions_json = data
        .permissions
        .as_ref()
        .map(|p| serde_json::to_string(p).unwrap_or_else(|_| "[]".to_string()));

    let rows = sqlx::query!(
        "UPDATE role SET name = COALESCE(?1, name), display_name = COALESCE(?2, display_name), description = COALESCE(?3, description), permissions = COALESCE(?4, permissions), is_active = COALESCE(?5, is_active) WHERE id = ?6",
        data.name,
        data.display_name,
        data.description,
        permissions_json,
        data.is_active,
        id
    )
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!("Role {id} not found")));
    }
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Role {id} not found")))
}

pub async fn delete(pool: &SqlitePool, id: i64) -> RepoResult<bool> {
    let existing = find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Role {id} not found")))?;

    if existing.is_system {
        return Err(RepoError::Business(
            ErrorCode::RoleIsSystem,
            "Cannot delete system role".into(),
        ));
    }

    sqlx::query!("DELETE FROM role WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(true)
}
