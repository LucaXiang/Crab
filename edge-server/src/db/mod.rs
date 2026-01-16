//! Database Module
//!
//! Handles SurrealDB connection and provides database service

pub mod models;

use crate::common::AppError;
use include_dir::{Dir, include_dir};
use std::path::PathBuf;
use surrealdb::{
    Surreal,
    engine::local::{Db, RocksDb},
};
use surrealdb_migrations::MigrationRunner;

static MIGRATIONS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");

/// Database service wrapper
#[derive(Clone)]
pub struct DbService {
    pub db: Surreal<Db>,
}

impl DbService {
    /// Create a new database service
    pub async fn new(db_path: &str) -> Result<Self, AppError> {
        let path = PathBuf::from(db_path);

        let db = Surreal::new::<RocksDb>(path)
            .await
            .map_err(|e| AppError::database(format!("Failed to open database: {}", e)))?;

        // Use namespace and database
        db.use_ns("edge_server")
            .use_db("edge_server")
            .await
            .map_err(|e| AppError::database(format!("Failed to use ns/db: {}", e)))?;
        tracing::info!("Database connection established");

        // Apply migrations
        tracing::info!("Applying database migrations...");

        MigrationRunner::new(&db)
            .load_files(&MIGRATIONS_DIR)
            .up()
            .await
            .map_err(|e| AppError::database(format!("Failed to apply migrations: {}", e)))?;
        tracing::info!("Database migrations applied successfully");

        Ok(Self { db })
    }
}
