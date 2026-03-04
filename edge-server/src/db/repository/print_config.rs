//! Print Config Repository (Singleton)
//!
//! Persists system default print destination IDs.

use super::RepoResult;
use sqlx::{FromRow, SqlitePool};

const SINGLETON_ID: i64 = 1;

#[derive(Debug, Clone, FromRow)]
pub struct PrintConfigRow {
    pub kitchen_enabled: bool,
    pub default_kitchen_printer: Option<String>,
    pub label_enabled: bool,
    pub default_label_printer: Option<String>,
}

pub async fn get(pool: &SqlitePool) -> RepoResult<PrintConfigRow> {
    let row = sqlx::query_as::<_, PrintConfigRow>(
        "SELECT kitchen_enabled, default_kitchen_printer, label_enabled, default_label_printer FROM print_config WHERE id = ?",
    )
    .bind(SINGLETON_ID)
    .fetch_optional(pool)
    .await?;

    Ok(row.unwrap_or(PrintConfigRow {
        kitchen_enabled: true,
        default_kitchen_printer: None,
        label_enabled: true,
        default_label_printer: None,
    }))
}

pub async fn update(
    pool: &SqlitePool,
    kitchen_enabled: bool,
    kitchen: Option<&str>,
    label_enabled: bool,
    label: Option<&str>,
) -> RepoResult<()> {
    let now = shared::util::now_millis();
    sqlx::query(
        "INSERT INTO print_config (id, kitchen_enabled, default_kitchen_printer, label_enabled, default_label_printer, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
           kitchen_enabled = excluded.kitchen_enabled,
           default_kitchen_printer = excluded.default_kitchen_printer,
           label_enabled = excluded.label_enabled,
           default_label_printer = excluded.default_label_printer,
           updated_at = excluded.updated_at",
    )
    .bind(SINGLETON_ID)
    .bind(kitchen_enabled)
    .bind(kitchen)
    .bind(label_enabled)
    .bind(label)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}
