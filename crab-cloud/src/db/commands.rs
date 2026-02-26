//! Cloud command CRUD operations (audit trail for RPC commands)

use sqlx::PgPool;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Create a new command record for an edge-server (audit trail)
pub async fn create_command(
    pool: &PgPool,
    store_id: i64,
    tenant_id: &str,
    command_type: &str,
    payload: &serde_json::Value,
    now: i64,
) -> Result<i64, BoxError> {
    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO store_commands (store_id, tenant_id, command_type, payload, status, created_at)
        VALUES ($1, $2, $3, $4, 'pending', $5)
        RETURNING id
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(command_type)
    .bind(payload)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

/// Complete a single command with its RPC result
pub async fn complete_command(
    pool: &PgPool,
    command_id: i64,
    success: bool,
    result: &serde_json::Value,
    now: i64,
) -> Result<(), BoxError> {
    let status = if success { "completed" } else { "failed" };

    sqlx::query(
        r#"
        UPDATE store_commands
        SET status = $1, result = $2, executed_at = $3
        WHERE id = $4
        "#,
    )
    .bind(status)
    .bind(result)
    .bind(now)
    .bind(command_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get command history for an edge-server (for tenant management API)
#[derive(sqlx::FromRow, serde::Serialize)]
pub struct CommandRecord {
    pub id: i64,
    pub command_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub created_at: i64,
    pub executed_at: Option<i64>,
    pub result: Option<serde_json::Value>,
}

pub async fn get_command_history(
    pool: &PgPool,
    store_id: i64,
    tenant_id: &str,
    limit: i32,
    offset: i32,
) -> Result<Vec<CommandRecord>, BoxError> {
    let rows: Vec<CommandRecord> = sqlx::query_as(
        r#"
        SELECT id, command_type, payload, status, created_at, executed_at, result
        FROM store_commands
        WHERE store_id = $1 AND tenant_id = $2
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
