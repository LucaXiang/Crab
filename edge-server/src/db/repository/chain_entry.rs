//! Chain entry repository — unified hash chain sync queries

use sqlx::SqlitePool;

use super::RepoResult;

/// A chain entry row for sync purposes
#[derive(Debug, sqlx::FromRow)]
pub struct ChainEntryRow {
    pub id: i64,
    pub entry_type: String,
    pub entry_pk: i64,
    pub prev_hash: String,
    pub curr_hash: String,
    pub created_at: i64,
}

/// List unsynced chain entries ordered by id (strict ordering for hash chain integrity).
pub async fn list_unsynced(pool: &SqlitePool, limit: i64) -> RepoResult<Vec<ChainEntryRow>> {
    let rows = sqlx::query_as::<_, ChainEntryRow>(
        "SELECT id, entry_type, entry_pk, prev_hash, curr_hash, created_at \
         FROM chain_entry WHERE cloud_synced = 0 ORDER BY id LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Mark chain entries as cloud-synced.
pub async fn mark_synced(pool: &SqlitePool, ids: &[i64]) -> RepoResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    // Build placeholder list: ?,?,?...
    let placeholders: String = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("UPDATE chain_entry SET cloud_synced = 1 WHERE id IN ({placeholders})");
    let mut query = sqlx::query(&sql);
    for &id in ids {
        query = query.bind(id);
    }
    query.execute(pool).await?;
    Ok(())
}
