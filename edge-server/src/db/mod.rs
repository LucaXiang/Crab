//! Database Module
//!
//! Handles SurrealDB connection and provides database service

pub mod models;

use crate::common::AppError;
use std::path::PathBuf;
use surrealdb::{
    Surreal,
    engine::local::{Db, RocksDb},
};

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
            .map_err(|e| AppError::Database(format!("Failed to open database: {}", e)))?;

        // Use namespace and database
        db.use_ns("edge_server")
            .use_db("edge_server")
            .await
            .map_err(|e| AppError::Database(format!("Failed to use ns/db: {}", e)))?;

        tracing::info!("Database connection established");

        Ok(Self { db })
    }
}
