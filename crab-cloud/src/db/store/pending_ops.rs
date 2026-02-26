//! Pending ops queue — stores Console edits when edge is offline
//!
//! On edge reconnect, drain and replay via WebSocket.

use shared::cloud::store_op::StoreOp;
use sqlx::PgPool;

use super::BoxError;

/// Queue a StoreOp for later delivery to an offline edge.
pub async fn insert(
    pool: &PgPool,
    store_id: i64,
    op: &StoreOp,
    changed_at: i64,
) -> Result<(), BoxError> {
    let now = shared::util::now_millis();
    let op_json = serde_json::to_value(op)?;

    sqlx::query(
        "INSERT INTO store_pending_ops (store_id, op, changed_at, created_at) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(store_id)
    .bind(op_json)
    .bind(changed_at)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Fetch all pending ops for an edge server in FIFO order (by id).
///
/// Returns (row_id, op, changed_at). Caller is responsible for deleting
/// each row after successful delivery via `delete_one`.
pub async fn fetch_ordered(
    pool: &PgPool,
    store_id: i64,
) -> Result<Vec<(i64, StoreOp, i64)>, BoxError> {
    let rows: Vec<(i64, serde_json::Value, i64)> = sqlx::query_as(
        "SELECT id, op, changed_at FROM store_pending_ops \
         WHERE store_id = $1 \
         ORDER BY id",
    )
    .bind(store_id)
    .fetch_all(pool)
    .await?;

    let mut result = Vec::with_capacity(rows.len());
    for (row_id, op_json, changed_at) in rows {
        match serde_json::from_value::<StoreOp>(op_json) {
            Ok(op) => result.push((row_id, op, changed_at)),
            Err(e) => {
                tracing::warn!(row_id, "Failed to deserialize pending op, skipping: {e}");
                // Delete the bad row so it doesn't block future drains
                let _ = delete_one(pool, row_id).await;
            }
        }
    }

    Ok(result)
}

/// Delete a single pending op by row id (called after successful delivery).
pub async fn delete_one(pool: &PgPool, row_id: i64) -> Result<(), BoxError> {
    sqlx::query("DELETE FROM store_pending_ops WHERE id = $1")
        .bind(row_id)
        .execute(pool)
        .await?;
    Ok(())
}
