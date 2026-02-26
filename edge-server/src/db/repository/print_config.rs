//! Print Config Repository (Singleton)
//!
//! Persists system default print destination IDs.

use super::RepoResult;
use sqlx::{FromRow, SqlitePool};

const SINGLETON_ID: i64 = 1;

#[derive(Debug, Clone, FromRow)]
pub struct PrintConfigRow {
    pub default_kitchen_printer: Option<String>,
    pub default_label_printer: Option<String>,
}

pub async fn get(pool: &SqlitePool) -> RepoResult<PrintConfigRow> {
    let row = sqlx::query_as::<_, PrintConfigRow>(
        "SELECT default_kitchen_printer, default_label_printer FROM print_config WHERE id = ?",
    )
    .bind(SINGLETON_ID)
    .fetch_optional(pool)
    .await?;

    Ok(row.unwrap_or(PrintConfigRow {
        default_kitchen_printer: None,
        default_label_printer: None,
    }))
}

pub async fn update(
    pool: &SqlitePool,
    kitchen: Option<&str>,
    label: Option<&str>,
) -> RepoResult<()> {
    let now = shared::util::now_millis();
    sqlx::query(
        "INSERT INTO print_config (id, default_kitchen_printer, default_label_printer, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET
           default_kitchen_printer = excluded.default_kitchen_printer,
           default_label_printer = excluded.default_label_printer,
           updated_at = excluded.updated_at",
    )
    .bind(SINGLETON_ID)
    .bind(kitchen)
    .bind(label)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}
