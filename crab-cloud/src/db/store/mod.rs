//! Store resource database operations
//!
//! Two write paths:
//! 1. Edge sync: upsert_*_from_sync (edge → cloud)
//! 2. Console CRUD: create/update/delete_*_direct (cloud → PG, then RPC to edge)

pub mod attribute;
pub mod category;
pub mod daily_report;
pub mod dining_table;
pub mod employee;
pub mod label_template;
pub mod pending_ops;
pub mod price_rule;
pub mod product;
pub mod shift;
pub mod store_info;
pub mod tag;
pub mod zone;

pub use attribute::*;
pub use category::*;
pub use daily_report::*;
pub use dining_table::*;
pub use employee::*;
pub use label_template::*;
pub use price_rule::*;
pub use product::*;
pub use shift::*;
pub use store_info::*;
pub use tag::*;
pub use zone::*;

use sqlx::PgPool;

pub(crate) type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Generate a Snowflake-style i64 for use as source_id.
///
/// Layout (53 bits, fits in JavaScript's Number.MAX_SAFE_INTEGER):
///   - 41 bits: milliseconds since 2024-01-01 UTC (~69 years)
///   - 12 bits: random (4096 values per ms, collision-free at POS scale)
///
/// Properties: non-sequential (can't infer record count), roughly time-ordered,
/// i64 compatible, stateless (no counter to persist across restarts).
/// UNIQUE constraint on (edge_server_id, source_id) is the ultimate safety net.
pub(crate) fn snowflake_id() -> i64 {
    use rand::Rng;
    // Custom epoch: 2024-01-01 00:00:00 UTC
    const EPOCH_MS: i64 = 1_704_067_200_000;
    let now = shared::util::now_millis();
    let ts = (now - EPOCH_MS) & 0x1FF_FFFF_FFFF; // 41 bits
    let rand_bits: i64 = rand::thread_rng().gen_range(0..0x1000); // 12 bits
    (ts << 12) | rand_bits
}

/// Increment store version for an edge server, returning the new version.
pub async fn increment_store_version(pool: &PgPool, edge_server_id: i64) -> Result<i64, BoxError> {
    let now = shared::util::now_millis();
    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO store_versions (edge_server_id, version, updated_at)
        VALUES ($1, 1, $2)
        ON CONFLICT (edge_server_id) DO UPDATE SET
            version = store_versions.version + 1,
            updated_at = EXCLUDED.updated_at
        RETURNING version
        "#,
    )
    .bind(edge_server_id)
    .bind(now)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}
