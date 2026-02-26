//! Tenant image reference tracking for S3 orphan cleanup
// TODO: Wire up image cleanup pipeline
#![allow(dead_code)]

use sqlx::PgPool;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Register an image hash on upload (idempotent). Does NOT increment ref_count.
pub async fn register(
    pool: &PgPool,
    tenant_id: &str,
    hash: &str,
    now: i64,
) -> Result<(), BoxError> {
    sqlx::query(
        r#"
        INSERT INTO tenant_images (tenant_id, hash, ref_count, created_at)
        VALUES ($1, $2, 0, $3)
        ON CONFLICT (tenant_id, hash) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .bind(hash)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Increment ref_count for a hash (product created/updated with this image).
/// Also clears orphaned_at if it was previously marked.
pub async fn increment_ref(pool: &PgPool, tenant_id: &str, hash: &str) -> Result<(), BoxError> {
    sqlx::query(
        r#"
        UPDATE tenant_images
        SET ref_count = ref_count + 1, orphaned_at = NULL
        WHERE tenant_id = $1 AND hash = $2
        "#,
    )
    .bind(tenant_id)
    .bind(hash)
    .execute(pool)
    .await?;
    Ok(())
}

/// Decrement ref_count for a hash. If it reaches 0, mark as orphaned.
pub async fn decrement_ref(
    pool: &PgPool,
    tenant_id: &str,
    hash: &str,
    now: i64,
) -> Result<(), BoxError> {
    sqlx::query(
        r#"
        UPDATE tenant_images
        SET ref_count = GREATEST(ref_count - 1, 0),
            orphaned_at = CASE WHEN ref_count <= 1 THEN $3 ELSE orphaned_at END
        WHERE tenant_id = $1 AND hash = $2
        "#,
    )
    .bind(tenant_id)
    .bind(hash)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetch orphaned images older than the given cutoff timestamp.
/// Returns (tenant_id, hash) pairs for S3 deletion.
pub async fn fetch_orphans(
    pool: &PgPool,
    cutoff: i64,
    limit: i32,
) -> Result<Vec<(String, String)>, BoxError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT tenant_id, hash FROM tenant_images
        WHERE orphaned_at IS NOT NULL AND orphaned_at < $1
        LIMIT $2
        "#,
    )
    .bind(cutoff)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Delete image records after S3 cleanup.
pub async fn delete_records(pool: &PgPool, tenant_id: &str, hash: &str) -> Result<(), BoxError> {
    sqlx::query("DELETE FROM tenant_images WHERE tenant_id = $1 AND hash = $2")
        .bind(tenant_id)
        .bind(hash)
        .execute(pool)
        .await?;
    Ok(())
}

/// Delete all image records for a tenant (for tenant data purge).
/// Returns the hashes that were deleted (for S3 cleanup).
pub async fn delete_all_for_tenant(
    pool: &PgPool,
    tenant_id: &str,
) -> Result<Vec<String>, BoxError> {
    let hashes: Vec<String> =
        sqlx::query_scalar("DELETE FROM tenant_images WHERE tenant_id = $1 RETURNING hash")
            .bind(tenant_id)
            .fetch_all(pool)
            .await?;
    Ok(hashes)
}

/// Get the image hash for a product (for capturing old hash before update).
pub async fn get_product_image(
    pool: &PgPool,
    store_id: i64,
    source_id: i64,
) -> Result<Option<String>, BoxError> {
    let hash: Option<String> = sqlx::query_scalar(
        "SELECT image FROM store_products WHERE store_id = $1 AND source_id = $2",
    )
    .bind(store_id)
    .bind(source_id)
    .fetch_optional(pool)
    .await?;
    // Filter out empty strings
    Ok(hash.filter(|h| !h.is_empty()))
}

/// Get image hashes for multiple products (for bulk delete).
pub async fn get_product_images_bulk(
    pool: &PgPool,
    store_id: i64,
    source_ids: &[i64],
) -> Result<Vec<String>, BoxError> {
    if source_ids.is_empty() {
        return Ok(vec![]);
    }
    let hashes: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT DISTINCT image FROM store_products
        WHERE store_id = $1 AND source_id = ANY($2) AND image != ''
        "#,
    )
    .bind(store_id)
    .bind(source_ids)
    .fetch_all(pool)
    .await?;
    Ok(hashes)
}
