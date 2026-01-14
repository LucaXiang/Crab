//! Unified Error Handling
//!
//! Provides application-wide error types and response structures

use axum::{
    Json,
    extract::multipart::MultipartError,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use tracing::error;

/// Unified API response structure
#[derive(Debug, Serialize)]
pub struct AppResponse<T> {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

/// Application-level error type
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    // ========== Authentication Errors ==========
    #[error("Authentication required")]
    Unauthorized,

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Permission denied: {0}")]
    Forbidden(String),

    // ========== Business Logic Errors ==========
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Resource already exists: {0}")]
    Conflict(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Business rule violation: {0}")]
    BusinessRule(String),

    // ========== System Errors ==========
    #[error("Database error: {0}")]
    Database(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Invalid request: {0}")]
    Invalid(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            // Authentication errors (401)
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "E3001", "Please login first"),
            AppError::TokenExpired => (StatusCode::UNAUTHORIZED, "E3003", "Token expired"),
            AppError::invalid_token => (StatusCode::UNAUTHORIZED, "E3002", "Invalid token"),

            // Authorization errors (403)
            AppError::forbidden(msg) => (StatusCode::FORBIDDEN, "E2001", msg.as_str()),

            // Not found (404)
            AppError::not_found(msg) => (StatusCode::NOT_FOUND, "E0003", msg.as_str()),

            // Conflict (409)
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "E0004", msg.as_str()),

            // Validation (400)
            AppError::validation(msg) => (StatusCode::BAD_REQUEST, "E0002", msg.as_str()),

            // Business rule (422)
            AppError::BusinessRule(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "E0005", msg.as_str())
            }

            // Database errors (500)
            AppError::database(msg) => {
                error!(target: "database", error = %msg, "Database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, "E9002", "Database error")
            }

            // Internal errors (500)
            AppError::internal(msg) => {
                error!(target: "internal", error = %msg, "Internal error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, "E9001", "Internal server error")
            }

            // Invalid request (400)
            AppError::invalid(msg) => (StatusCode::BAD_REQUEST, "E0006", msg.as_str()),
        };

        let body = Json(AppResponse::<()> {
            code: code.to_string(),
            message: message.to_string(),
            data: None,
            trace_id: None, // TODO: Extract from request context
        });

        (status, body).into_response()
    }
}

impl From<MultipartError> for AppError {
    fn from(e: MultipartError) -> Self {
        AppError::validation(format!("Multipart error: {}", e))
    }
}

// ========== Conversions from existing error types ==========

// NOTE: Uncomment these when integrating with existing error types
//
// impl From<crate::error::ServiceError> for AppError {
//     fn from(e: crate::error::ServiceError) -> Self {
//         match e {
//             crate::error::ServiceError::NotFound { entity } => {
//                 AppError::not_found(format!("{} not found", entity))
//             }
//             crate::error::ServiceError::AlreadyExists { entity, value } => {
//                 AppError::Conflict(format!("{} already exists: {}", entity, value))
//             }
//             crate::error::ServiceError::InvalidInput { field, reason } => {
//                 AppError::validation(format!("Invalid {}: {}", field, reason))
//             }
//             crate::error::ServiceError::InvalidCredentials => {
//                 AppError::Unauthorized
//             }
//             crate::error::ServiceError::InactiveUser => {
//                 AppError::forbidden("User account is inactive".to_string())
//             }
//             crate::error::ServiceError::PasswordMismatch => {
//                 AppError::Unauthorized
//             }
//             crate::error::ServiceError::PermissionDenied(msg) => {
//                 AppError::forbidden(msg)
//             }
//             crate::error::ServiceError::BusinessRule(msg) => {
//                 AppError::BusinessRule(msg)
//             }
//             crate::error::ServiceError::Database(msg) => {
//                 AppError::database(msg)
//             }
//             crate::error::ServiceError::Transaction(msg) => {
//                 AppError::database(msg)
//             }
//             crate::error::ServiceError::Other(msg) => {
//                 AppError::internal(msg)
//             }
//         }
//     }
// }

// ========== Helper functions ==========

/// Create a successful response
pub fn ok<T: Serialize>(data: T) -> Json<AppResponse<T>> {
    Json(AppResponse {
        code: "E0000".to_string(),
        message: "Success".to_string(),
        data: Some(data),
        trace_id: None,
    })
}

/// Create a successful response with custom message
pub fn ok_with_message<T: Serialize>(data: T, message: impl Into<String>) -> Json<AppResponse<T>> {
    Json(AppResponse {
        code: "E0000".to_string(),
        message: message.into(),
        data: Some(data),
        trace_id: None,
    })
}
