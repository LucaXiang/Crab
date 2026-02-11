//! Repository Module
//!
//! Free functions for SQLite CRUD via sqlx.
//! Each sub-module exposes `pub async fn` taking `&SqlitePool`.

// Auth
pub mod employee;
pub mod role;

// Product Domain
pub mod attribute;
pub mod print_destination;
pub mod tag;

// Location
pub mod dining_table;
pub mod zone;

// Pricing
pub mod price_rule;

// Orders
pub mod order;

// Payments
pub mod payment;

// System
pub mod store_info;
pub mod label_template;
pub mod system_state;
pub mod system_issue;

// Image
pub mod image_ref;

// Marketing & Membership
pub mod marketing_group;
pub mod member;
pub mod stamp;

// Operations (班次与日结)
pub mod shift;
pub mod daily_report;

use shared::error::{AppError, ErrorCode};
use thiserror::Error;

/// Repository error types
#[derive(Debug, Error)]
pub enum RepoError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Duplicate: {0}")]
    Duplicate(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

impl From<sqlx::Error> for RepoError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => RepoError::NotFound("Record not found".into()),
            sqlx::Error::Database(db_err) => {
                let msg = db_err.message().to_string();
                if msg.contains("UNIQUE constraint failed") {
                    RepoError::Duplicate(msg)
                } else if msg.contains("FOREIGN KEY constraint failed") {
                    RepoError::Validation(msg)
                } else {
                    RepoError::Database(msg)
                }
            }
            _ => RepoError::Database(err.to_string()),
        }
    }
}

impl From<RepoError> for AppError {
    fn from(err: RepoError) -> Self {
        match err {
            RepoError::NotFound(msg) => AppError::not_found(msg),
            RepoError::Duplicate(msg) => AppError::with_message(ErrorCode::AlreadyExists, msg),
            RepoError::Database(msg) => AppError::database(msg),
            RepoError::Validation(msg) => AppError::validation(msg),
        }
    }
}

/// Result type for repository operations
pub type RepoResult<T> = Result<T, RepoError>;
