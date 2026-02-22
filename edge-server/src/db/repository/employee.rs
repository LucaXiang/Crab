//! Employee Repository

use super::{RepoError, RepoResult};
use shared::error::ErrorCode;
use shared::models::{Employee, EmployeeCreate, EmployeeUpdate};
use sqlx::SqlitePool;

/// Internal type that includes hash_pass (never returned to API)
#[derive(Debug, sqlx::FromRow)]
pub struct EmployeeWithHash {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub hash_pass: String,
    pub role_id: i64,
    pub is_system: bool,
    pub is_active: bool,
    pub created_at: i64,
}

/// Hash password using argon2
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    use argon2::{
        Argon2,
        password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
    };
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(password_hash.to_string())
}

/// Verify password against argon2 hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    use argon2::{
        Argon2,
        password_hash::{PasswordHash, PasswordVerifier},
    };
    let parsed_hash = PasswordHash::new(hash)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub async fn find_all(pool: &SqlitePool) -> RepoResult<Vec<Employee>> {
    let employees = sqlx::query_as::<_, Employee>(
        "SELECT id, username, display_name, role_id, is_system, is_active, created_at FROM employee WHERE is_active = 1 ORDER BY username",
    )
    .fetch_all(pool)
    .await?;
    Ok(employees)
}

pub async fn find_all_with_inactive(pool: &SqlitePool) -> RepoResult<Vec<Employee>> {
    let employees = sqlx::query_as::<_, Employee>(
        "SELECT id, username, display_name, role_id, is_system, is_active, created_at FROM employee ORDER BY username",
    )
    .fetch_all(pool)
    .await?;
    Ok(employees)
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<Employee>> {
    let employee = sqlx::query_as::<_, Employee>(
        "SELECT id, username, display_name, role_id, is_system, is_active, created_at FROM employee WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(employee)
}

pub async fn find_by_username(pool: &SqlitePool, username: &str) -> RepoResult<Option<Employee>> {
    let employee = sqlx::query_as::<_, Employee>(
        "SELECT id, username, display_name, role_id, is_system, is_active, created_at FROM employee WHERE username = ? LIMIT 1",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    Ok(employee)
}

/// Find by username with hash_pass (for auth)
pub async fn find_by_username_with_hash(
    pool: &SqlitePool,
    username: &str,
) -> RepoResult<Option<EmployeeWithHash>> {
    let employee = sqlx::query_as::<_, EmployeeWithHash>(
        "SELECT id, username, display_name, hash_pass, role_id, is_system, is_active, created_at FROM employee WHERE username = ? LIMIT 1",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    Ok(employee)
}

pub async fn create(pool: &SqlitePool, data: EmployeeCreate) -> RepoResult<Employee> {
    let hash_pass = hash_password(&data.password)
        .map_err(|e| RepoError::Database(format!("Failed to hash password: {e}")))?;
    let display_name = data.display_name.unwrap_or_else(|| data.username.clone());
    let now = shared::util::now_millis();

    let id = sqlx::query_scalar!(
        r#"INSERT INTO employee (username, hash_pass, display_name, role_id, is_system, is_active, created_at) VALUES (?, ?, ?, ?, 0, 1, ?) RETURNING id as "id!""#,
        data.username,
        hash_pass,
        display_name,
        data.role_id,
        now
    )
    .fetch_one(pool)
    .await?;

    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create employee".into()))
}

pub async fn update(pool: &SqlitePool, id: i64, data: EmployeeUpdate) -> RepoResult<Employee> {
    // Check is_system flag
    let existing = find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Employee {id} not found")))?;

    if existing.is_system
        && (data.username.is_some()
            || data.role_id.is_some()
            || data.is_active.is_some()
            || data.display_name.is_some())
    {
        return Err(RepoError::Business(
            ErrorCode::EmployeeIsSystem,
            "System user can only change password".into(),
        ));
    }

    // Hash password if provided
    let hash_pass = if let Some(ref password) = data.password {
        Some(
            hash_password(password)
                .map_err(|e| RepoError::Database(format!("Failed to hash password: {e}")))?,
        )
    } else {
        None
    };

    let rows = sqlx::query!(
        "UPDATE employee SET username = COALESCE(?1, username), display_name = COALESCE(?2, display_name), hash_pass = COALESCE(?3, hash_pass), role_id = COALESCE(?4, role_id), is_active = COALESCE(?5, is_active) WHERE id = ?6",
        data.username,
        data.display_name,
        hash_pass,
        data.role_id,
        data.is_active,
        id
    )
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!("Employee {id} not found")));
    }
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Employee {id} not found")))
}

pub async fn delete(pool: &SqlitePool, id: i64) -> RepoResult<bool> {
    let existing = find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Employee {id} not found")))?;

    if existing.is_system {
        return Err(RepoError::Business(
            ErrorCode::EmployeeIsSystem,
            "Cannot delete system user".into(),
        ));
    }

    sqlx::query!("DELETE FROM employee WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(true)
}
