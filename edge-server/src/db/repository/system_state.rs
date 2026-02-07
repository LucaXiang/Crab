//! System State Repository (Singleton)

use super::{RepoError, RepoResult};
use shared::models::{SystemState, SystemStateUpdate};
use sqlx::SqlitePool;

const SINGLETON_ID: i64 = 1;

pub async fn get_or_create(pool: &SqlitePool) -> RepoResult<SystemState> {
    if let Some(state) = get(pool).await? {
        return Ok(state);
    }

    let now = shared::util::now_millis();
    sqlx::query(
        "INSERT INTO system_state (id, order_count, created_at, updated_at) VALUES (?, 0, ?, ?)",
    )
    .bind(SINGLETON_ID)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    get(pool)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create system state".into()))
}

pub async fn get(pool: &SqlitePool) -> RepoResult<Option<SystemState>> {
    let state = sqlx::query_as::<_, SystemState>(
        "SELECT id, genesis_hash, last_order_id, last_order_hash, synced_up_to_id, synced_up_to_hash, last_sync_time, order_count, created_at, updated_at FROM system_state WHERE id = ?",
    )
    .bind(SINGLETON_ID)
    .fetch_optional(pool)
    .await?;
    Ok(state)
}

pub async fn update(pool: &SqlitePool, data: SystemStateUpdate) -> RepoResult<SystemState> {
    let now = shared::util::now_millis();
    let rows = sqlx::query(
        "UPDATE system_state SET genesis_hash = COALESCE(?1, genesis_hash), last_order_id = COALESCE(?2, last_order_id), last_order_hash = COALESCE(?3, last_order_hash), synced_up_to_id = COALESCE(?4, synced_up_to_id), synced_up_to_hash = COALESCE(?5, synced_up_to_hash), last_sync_time = COALESCE(?6, last_sync_time), order_count = COALESCE(?7, order_count), updated_at = ?8 WHERE id = ?9",
    )
    .bind(&data.genesis_hash)
    .bind(&data.last_order_id)
    .bind(&data.last_order_hash)
    .bind(&data.synced_up_to_id)
    .bind(&data.synced_up_to_hash)
    .bind(data.last_sync_time)
    .bind(data.order_count)
    .bind(now)
    .bind(SINGLETON_ID)
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        return Err(RepoError::Database("Failed to update system state".into()));
    }
    get(pool)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to read system state after update".into()))
}

pub async fn init_genesis(pool: &SqlitePool, genesis_hash: String) -> RepoResult<SystemState> {
    update(
        pool,
        SystemStateUpdate {
            genesis_hash: Some(genesis_hash),
            ..Default::default()
        },
    )
    .await
}

/// Atomically increment order_count and return the new value
pub async fn get_next_order_number(pool: &SqlitePool) -> RepoResult<i32> {
    let now = shared::util::now_millis();
    let new_count = sqlx::query_scalar::<_, i32>(
        "UPDATE system_state SET order_count = order_count + 1, updated_at = ?1 WHERE id = ?2 RETURNING order_count",
    )
    .bind(now)
    .bind(SINGLETON_ID)
    .fetch_one(pool)
    .await?;
    Ok(new_count)
}

/// Update last order info with atomic order_count increment
pub async fn update_last_order(
    pool: &SqlitePool,
    order_id: &str,
    order_hash: String,
) -> RepoResult<SystemState> {
    let now = shared::util::now_millis();
    sqlx::query(
        "UPDATE system_state SET last_order_id = ?1, last_order_hash = ?2, order_count = order_count + 1, updated_at = ?3 WHERE id = ?4",
    )
    .bind(order_id)
    .bind(&order_hash)
    .bind(now)
    .bind(SINGLETON_ID)
    .execute(pool)
    .await?;

    get(pool)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to update system state".into()))
}

/// Update sync state
pub async fn update_sync_state(
    pool: &SqlitePool,
    synced_up_to_id: &str,
    synced_up_to_hash: String,
) -> RepoResult<SystemState> {
    update(
        pool,
        SystemStateUpdate {
            synced_up_to_id: Some(synced_up_to_id.to_string()),
            synced_up_to_hash: Some(synced_up_to_hash),
            last_sync_time: Some(shared::util::now_millis()),
            ..Default::default()
        },
    )
    .await
}
