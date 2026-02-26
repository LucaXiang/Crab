//! Employee database operations (normalized columns, not JSONB)

use shared::cloud::store_op::StoreOpData;
use shared::models::employee::{Employee, EmployeeCreate};
use sqlx::PgPool;

use super::BoxError;

// ── Edge Sync ──

pub async fn upsert_employee_from_sync(
    pool: &PgPool,
    store_id: i64,
    source_id: i64,
    data: &serde_json::Value,
    now: i64,
) -> Result<(), BoxError> {
    let emp: EmployeeSyncData = serde_json::from_value(data.clone())?;
    sqlx::query(
        r#"
        INSERT INTO store_employees (
            store_id, source_id, username, hash_pass, name,
            role_id, is_system, is_active, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (store_id, source_id)
        DO UPDATE SET
            username = EXCLUDED.username, hash_pass = EXCLUDED.hash_pass,
            name = EXCLUDED.name, role_id = EXCLUDED.role_id,
            is_system = EXCLUDED.is_system, is_active = EXCLUDED.is_active,
            updated_at = EXCLUDED.updated_at
        WHERE store_employees.updated_at <= EXCLUDED.updated_at
        "#,
    )
    .bind(store_id)
    .bind(source_id)
    .bind(&emp.username)
    .bind(&emp.hash_pass)
    .bind(&emp.name)
    .bind(emp.role_id)
    .bind(emp.is_system)
    .bind(emp.is_active)
    .bind(emp.created_at)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Full employee data from edge sync (includes hash_pass)
#[derive(serde::Deserialize)]
struct EmployeeSyncData {
    username: String,
    #[serde(default)]
    hash_pass: String,
    #[serde(default)]
    name: String,
    role_id: i64,
    #[serde(default)]
    is_system: bool,
    #[serde(default = "default_true")]
    is_active: bool,
    #[serde(default)]
    created_at: i64,
}

fn default_true() -> bool {
    true
}

// ── Console Read ──

pub async fn list_employees(pool: &PgPool, store_id: i64) -> Result<Vec<Employee>, BoxError> {
    let rows: Vec<Employee> = sqlx::query_as(
        r#"
        SELECT source_id AS id, username, name, role_id,
               is_system, is_active, created_at
        FROM store_employees
        WHERE store_id = $1
        ORDER BY created_at
        "#,
    )
    .bind(store_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ── Console CRUD ──

pub async fn create_employee_direct(
    pool: &PgPool,
    store_id: i64,
    _tenant_id: &str,
    data: &EmployeeCreate,
) -> Result<(i64, StoreOpData), BoxError> {
    let now = shared::util::now_millis();
    let name = data.name.as_deref().unwrap_or(&data.username);

    let hash_pass = {
        use argon2::{
            Argon2,
            password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
        };
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(data.password.as_bytes(), &salt)
            .map_err(|e| format!("Failed to hash password: {e}"))?
            .to_string()
    };

    let source_id = super::snowflake_id();

    sqlx::query(
        r#"
        INSERT INTO store_employees (
            store_id, source_id, username, hash_pass, name,
            role_id, is_system, is_active, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, FALSE, TRUE, $7, $7)
        "#,
    )
    .bind(store_id)
    .bind(source_id)
    .bind(&data.username)
    .bind(&hash_pass)
    .bind(name)
    .bind(data.role_id)
    .bind(now)
    .execute(pool)
    .await?;

    let employee = Employee {
        id: source_id,
        username: data.username.clone(),
        name: name.to_string(),
        role_id: data.role_id,
        is_system: false,
        is_active: true,
        created_at: now,
    };
    Ok((source_id, StoreOpData::Employee(employee)))
}

pub async fn update_employee_direct(
    pool: &PgPool,
    store_id: i64,
    source_id: i64,
    data: &shared::models::employee::EmployeeUpdate,
) -> Result<StoreOpData, BoxError> {
    let now = shared::util::now_millis();
    let mut tx = pool.begin().await?;

    // Hash new password if provided
    let hash_pass = if let Some(ref password) = data.password {
        use argon2::{
            Argon2,
            password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
        };
        let salt = SaltString::generate(&mut OsRng);
        Some(
            Argon2::default()
                .hash_password(password.as_bytes(), &salt)
                .map_err(|e| format!("Failed to hash password: {e}"))?
                .to_string(),
        )
    } else {
        None
    };

    sqlx::query(
        r#"
        UPDATE store_employees SET
            username = COALESCE($1, username),
            name = COALESCE($2, name),
            role_id = COALESCE($3, role_id),
            is_active = COALESCE($4, is_active),
            hash_pass = COALESCE($5, hash_pass),
            updated_at = $6
        WHERE store_id = $7 AND source_id = $8
        "#,
    )
    .bind(&data.username)
    .bind(&data.name)
    .bind(data.role_id)
    .bind(data.is_active)
    .bind(&hash_pass)
    .bind(now)
    .bind(store_id)
    .bind(source_id)
    .execute(&mut *tx)
    .await?;

    // Read back updated employee
    let employee: Employee = sqlx::query_as(
        r#"
        SELECT source_id AS id, username, name, role_id,
               is_system, is_active, created_at
        FROM store_employees
        WHERE store_id = $1 AND source_id = $2
        "#,
    )
    .bind(store_id)
    .bind(source_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or("Employee not found")?;

    tx.commit().await?;
    Ok(StoreOpData::Employee(employee))
}

pub async fn delete_employee_direct(
    pool: &PgPool,
    store_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    let rows = sqlx::query("DELETE FROM store_employees WHERE store_id = $1 AND source_id = $2")
        .bind(store_id)
        .bind(source_id)
        .execute(pool)
        .await?;
    if rows.rows_affected() == 0 {
        return Err("Employee not found".into());
    }
    Ok(())
}
