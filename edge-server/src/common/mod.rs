//! Common utilities and shared infrastructure
//!
//! This module contains core infrastructure used across the application:
//! - Configuration management
//! - Logging setup
//! - Error handling
//! - Result types
//! - Shared types

pub mod audit;
// pub mod config;
pub mod logger;
pub mod result;
pub mod types;

// Import from shared module
pub use shared::error::ApiError;
pub use shared::response::ApiResponse;

// Type aliases for backward compatibility
pub type AppError = ApiError;
pub type AppResponse<T> = ApiResponse<T>;

// Helper functions
pub fn ok<T: serde::Serialize>(data: T) -> axum::Json<ApiResponse<T>> {
    axum::Json(ApiResponse::ok(data))
}

pub fn ok_with_message<T: serde::Serialize>(
    data: T,
    message: impl Into<String>,
) -> axum::Json<ApiResponse<T>> {
    axum::Json(ApiResponse::ok_with_message(data, message))
}

// Re-export commonly used items
// pub use config::AppConfig;
pub use logger::{init_logger, init_logger_with_file};
pub use result::AppResult;
