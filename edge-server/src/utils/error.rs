//! Edge server error handling - re-exports from shared crate
//!
//! This module provides a thin wrapper around the unified error system
//! from the shared crate, with edge-server specific conveniences.

pub use shared::error::{ApiResponse, AppError, AppResult, ErrorCategory, ErrorCode};

// Legacy type alias for backward compatibility
pub type AppResponse2<T> = ApiResponse<T>;

/// Convenience functions for creating JSON responses
pub mod response {
    use super::*;
    use axum::Json;

    pub fn ok<T: serde::Serialize>(data: T) -> Json<ApiResponse<T>> {
        Json(ApiResponse::success(data))
    }

    pub fn ok_with_message<T: serde::Serialize>(data: T, message: impl Into<String>) -> Json<ApiResponse<T>> {
        Json(ApiResponse::success_with_message(message, data))
    }
}

pub use response::{ok, ok_with_message};
