//! Image Reference Repository

use super::RepoResult;
use shared::models::{ImageRef, ImageRefEntityType};
use sqlx::SqlitePool;
use std::collections::HashSet;

/// Sync image references for an entity.
/// Returns the list of removed hashes (for orphan detection).
pub async fn sync_refs(
    pool: &SqlitePool,
    entity_type: ImageRefEntityType,
    entity_id: &str,
    current_hashes: HashSet<String>,
) -> RepoResult<Vec<String>> {
    let entity_type_str = entity_type.as_str();

    // 1. Get existing refs
    let existing = get_entity_refs(pool, entity_type, entity_id).await?;
    let existing_hashes: HashSet<String> = existing.iter().map(|r| r.hash.clone()).collect();

    // 2. Diff
    let to_add: Vec<&String> = current_hashes.difference(&existing_hashes).collect();
    let to_remove: Vec<String> = existing_hashes.difference(&current_hashes).cloned().collect();

    // 3. Create new refs
    let now = shared::util::now_millis();
    for hash in &to_add {
        let hash_str = hash.as_str();
        sqlx::query!(
            "INSERT INTO image_ref (hash, entity_type, entity_id, created_at) VALUES (?, ?, ?, ?)",
            hash_str,
            entity_type_str,
            entity_id,
            now
        )
        .execute(pool)
        .await?;
    }

    // 4. Delete removed refs
    if !to_remove.is_empty() {
        for hash in &to_remove {
            sqlx::query!(
                "DELETE FROM image_ref WHERE entity_type = ? AND entity_id = ? AND hash = ?",
                entity_type_str,
                entity_id,
                hash
            )
            .execute(pool)
            .await?;
        }
        return Ok(to_remove);
    }

    Ok(vec![])
}

/// Delete all image references for an entity. Returns removed hashes.
pub async fn delete_entity_refs(
    pool: &SqlitePool,
    entity_type: ImageRefEntityType,
    entity_id: &str,
) -> RepoResult<Vec<String>> {
    let entity_type_str = entity_type.as_str();

    let refs = get_entity_refs(pool, entity_type, entity_id).await?;
    let hashes: Vec<String> = refs.into_iter().map(|r| r.hash).collect();

    sqlx::query!("DELETE FROM image_ref WHERE entity_type = ? AND entity_id = ?",
        entity_type_str,
        entity_id
    )
    .execute(pool)
    .await?;

    Ok(hashes)
}

/// Count references for a hash
pub async fn count_refs(pool: &SqlitePool, hash: &str) -> RepoResult<i64> {
    let count =
        sqlx::query_scalar!("SELECT COUNT(*) FROM image_ref WHERE hash = ?", hash)
            .fetch_one(pool)
            .await?;
    Ok(count)
}

/// Find orphan hashes (hashes with zero references)
pub async fn find_orphan_hashes(pool: &SqlitePool, hashes: &[String]) -> RepoResult<Vec<String>> {
    if hashes.is_empty() {
        return Ok(vec![]);
    }

    // Dynamic query: variable number of IN placeholders — keep as runtime query
    let placeholders = hashes.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT DISTINCT hash FROM image_ref WHERE hash IN ({placeholders})"
    );
    let mut query = sqlx::query_scalar::<_, String>(&sql);
    for hash in hashes {
        query = query.bind(hash);
    }
    let referenced: HashSet<String> = query.fetch_all(pool).await?.into_iter().collect();

    Ok(hashes
        .iter()
        .filter(|h| !referenced.contains(*h))
        .cloned()
        .collect())
}

/// Get all image references for an entity
pub async fn get_entity_refs(
    pool: &SqlitePool,
    entity_type: ImageRefEntityType,
    entity_id: &str,
) -> RepoResult<Vec<ImageRef>> {
    let entity_type_str = entity_type.as_str();
    let refs = sqlx::query_as::<_, ImageRef>(
        "SELECT id, hash, entity_type, entity_id, created_at FROM image_ref WHERE entity_type = ? AND entity_id = ?",
    )
    .bind(entity_type_str)
    .bind(entity_id)
    .fetch_all(pool)
    .await?;
    Ok(refs)
}
