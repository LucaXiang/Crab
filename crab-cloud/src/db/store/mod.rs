//! Store resource database operations
//!
//! Two write paths:
//! 1. Edge sync: upsert_*_from_sync (edge → cloud)
//! 2. Console CRUD: create/update/delete_*_direct (cloud → PG, then RPC to edge)

pub mod attribute;
pub mod category;
pub mod daily_report;
pub mod data_transfer;
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

/// Generate a Snowflake-style i64 — delegates to shared::util::snowflake_id().
pub(crate) fn snowflake_id() -> i64 {
    shared::util::snowflake_id()
}

/// Increment store version for an edge server, returning the new version.
pub async fn increment_store_version(pool: &PgPool, store_id: i64) -> Result<i64, BoxError> {
    let now = shared::util::now_millis();
    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO store_versions (store_id, version, updated_at)
        VALUES ($1, 1, $2)
        ON CONFLICT (store_id) DO UPDATE SET
            version = store_versions.version + 1,
            updated_at = EXCLUDED.updated_at
        RETURNING version
        "#,
    )
    .bind(store_id)
    .bind(now)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}
