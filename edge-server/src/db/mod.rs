//! Database Module
//!
//! Handles SQLite connection pool and migrations

pub mod id_migration;
pub mod repository;

use crate::utils::AppError;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::str::FromStr;

/// Database service — owns a SQLite connection pool
#[derive(Clone)]
pub struct DbService {
    pub pool: SqlitePool,
}

impl DbService {
    /// Create a new database service with WAL mode and separate read/write pools
    pub async fn new(db_path: &str) -> Result<Self, AppError> {
        // Build connection options: WAL, foreign keys, normal sync
        let options = SqliteConnectOptions::from_str(&format!("sqlite:{db_path}"))
            .map_err(|e| AppError::database(format!("Invalid database path: {e}")))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .pragma("foreign_keys", "ON")
            .optimize_on_close(true, None);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| AppError::database(format!("Failed to open database: {e}")))?;

        // busy_timeout: 写冲突时等待 5s 而非立即失败
        sqlx::query("PRAGMA busy_timeout = 5000;")
            .execute(&pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to set busy_timeout: {e}")))?;

        tracing::info!("Database connection established (SQLite WAL, busy_timeout=5000ms)");

        // Run migrations (ignore previously applied but now removed migrations)
        sqlx::migrate!("./migrations")
            .set_ignore_missing(true)
            .run(&pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to apply migrations: {e}")))?;
        tracing::info!("Database migrations applied");

        // One-time migration: remap legacy autoincrement IDs to snowflake IDs
        id_migration::migrate_ids_if_needed(&pool)
            .await
            .map_err(|e| AppError::database(format!("ID migration failed: {e}")))?;

        Ok(Self { pool })
    }
}
