//! Cloud command CRUD operations
//!
//! Commands flow: pending → delivered → completed/failed

use shared::cloud::CloudCommandResult;
use sqlx::PgPool;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Create a new command for an edge-server
pub async fn create_command(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
    command_type: &str,
    payload: &serde_json::Value,
    now: i64,
) -> Result<i64, BoxError> {
    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO cloud_commands (edge_server_id, tenant_id, command_type, payload, status, created_at)
        VALUES ($1, $2, $3, $4, 'pending', $5)
        RETURNING id
        "#,
    )
    .bind(edge_server_id)
    .bind(tenant_id)
    .bind(command_type)
    .bind(payload)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

/// Get pending commands for an edge-server (up to limit)
#[derive(sqlx::FromRow)]
pub struct PendingCommand {
    pub id: i64,
    pub command_type: String,
    pub payload: serde_json::Value,
    pub created_at: i64,
}

pub async fn get_pending(
    pool: &PgPool,
    edge_server_id: i64,
    limit: i32,
) -> Result<Vec<PendingCommand>, BoxError> {
    let rows: Vec<PendingCommand> = sqlx::query_as(
        r#"
        SELECT id, command_type, payload, created_at
        FROM cloud_commands
        WHERE edge_server_id = $1 AND status = 'pending'
        ORDER BY created_at ASC
        LIMIT $2
        "#,
    )
    .bind(edge_server_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Mark commands as delivered (sent to edge-server)
pub async fn mark_delivered(pool: &PgPool, ids: &[i64]) -> Result<(), BoxError> {
    if ids.is_empty() {
        return Ok(());
    }

    sqlx::query("UPDATE cloud_commands SET status = 'delivered' WHERE id = ANY($1)")
        .bind(ids)
        .execute(pool)
        .await?;

    Ok(())
}

/// Complete commands with results from edge-server
pub async fn complete_commands(
    pool: &PgPool,
    results: &[CloudCommandResult],
    now: i64,
) -> Result<(), BoxError> {
    for result in results {
        let status = if result.success {
            "completed"
        } else {
            "failed"
        };

        let result_json = serde_json::json!({
            "success": result.success,
            "data": result.data,
            "error": result.error,
        });

        let command_id = match result.command_id.parse::<i64>() {
            Ok(id) => id,
            Err(e) => {
                tracing::warn!(
                    command_id = %result.command_id,
                    "Invalid command_id in result, skipping: {e}"
                );
                continue;
            }
        };

        sqlx::query(
            r#"
            UPDATE cloud_commands
            SET status = $1, result = $2, executed_at = $3
            WHERE id = $4
            "#,
        )
        .bind(status)
        .bind(&result_json)
        .bind(now)
        .bind(command_id)
        .execute(pool)
        .await?;
    }

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
    edge_server_id: i64,
    tenant_id: &str,
    limit: i32,
    offset: i32,
) -> Result<Vec<CommandRecord>, BoxError> {
    let rows: Vec<CommandRecord> = sqlx::query_as(
        r#"
        SELECT id, command_type, payload, status, created_at, executed_at, result
        FROM cloud_commands
        WHERE edge_server_id = $1 AND tenant_id = $2
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(edge_server_id)
    .bind(tenant_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
