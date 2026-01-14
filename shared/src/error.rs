//! Error types for the shared crate
//!
//! Standardized error types that can be used across the entire framework

use crate::{http::{StatusCode, Response}, response::ApiResponse};
use thiserror::Error;

/// Standard API error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiErrorCode {
    /// Success
    Success,
    /// Validation error (400)
    Validation,
    /// Authentication required (401)
    Unauthorized,
    /// Invalid token (401)
    InvalidToken,
    /// Token expired (401)
    TokenExpired,
    /// Permission denied (403)
    Forbidden,
    /// Resource not found (404)
    NotFound,
    /// Resource already exists (409)
    Conflict,
    /// Business rule violation (422)
    BusinessRule,
    /// Internal server error (500)
    Internal,
    /// Database error (500)
    Database,
    /// Invalid request (400)
    Invalid,
}

impl ApiErrorCode {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Success => StatusCode::OK,
            Self::Validation => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::InvalidToken => StatusCode::UNAUTHORIZED,
            Self::TokenExpired => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict => StatusCode::CONFLICT,
            Self::BusinessRule => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Database => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Invalid => StatusCode::BAD_REQUEST,
        }
    }

    /// Get the default message for this error
    pub fn default_message(&self) -> &'static str {
        match self {
            Self::Success => "Success",
            Self::Validation => "Validation failed",
            Self::Unauthorized => "Authentication required",
            Self::InvalidToken => "Invalid token",
            Self::TokenExpired => "Token expired",
            Self::Forbidden => "Permission denied",
            Self::NotFound => "Resource not found",
            Self::Conflict => "Resource already exists",
            Self::BusinessRule => "Business rule violation",
            Self::Internal => "Internal server error",
            Self::Database => "Database error",
            Self::Invalid => "Invalid request",
        }
    }

    /// Get the error code string
    pub fn code(&self) -> &'static str {
        match self {
            Self::Success => "E0000",
            Self::Validation => "E0002",
            Self::Unauthorized => "E3001",
            Self::InvalidToken => "E3002",
            Self::TokenExpired => "E3003",
            Self::Forbidden => "E2001",
            Self::NotFound => "E0003",
            Self::Conflict => "E0004",
            Self::BusinessRule => "E0005",
            Self::Internal => "E9001",
            Self::Database => "E9002",
            Self::Invalid => "E0006",
        }
    }
}

impl std::fmt::Display for ApiErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Unified error type for the framework
#[derive(Debug, Error)]
pub enum ApiError {
    /// Validation error
    #[error("{message}")]
    Validation {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Authentication required
    #[error("Authentication required")]
    Unauthorized,

    /// Invalid token
    #[error("Invalid token: {message}")]
    InvalidToken { message: String },

    /// Token expired
    #[error("Token expired")]
    TokenExpired,

    /// Permission denied
    #[error("Permission denied: {message}")]
    Forbidden { message: String },

    /// Resource not found
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    /// Resource already exists
    #[error("Resource already exists: {resource}")]
    Conflict { resource: String },

    /// Business rule violation
    #[error("Business rule violation: {message}")]
    BusinessRule { message: String },

    /// Database error
    #[error("Database error: {message}")]
    Database { message: String },

    /// Internal server error
    #[error("Internal error: {message}")]
    Internal { message: String },

    /// Invalid request
    #[error("Invalid request: {message}")]
    Invalid { message: String },
}

impl ApiError {
    // ========== Convenient constructors ==========

    /// Create an Internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal { message: message.into() }
    }

    /// Create a Database error
    pub fn database(message: impl Into<String>) -> Self {
        Self::Database { message: message.into() }
    }

    /// Create a Validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation { message: message.into(), source: None }
    }

    /// Create a Forbidden error
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden { message: message.into() }
    }

    /// Create a NotFound error
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound { resource: resource.into() }
    }

    /// Create a Conflict error
    pub fn conflict(resource: impl Into<String>) -> Self {
        Self::Conflict { resource: resource.into() }
    }

    /// Create an Invalid error
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid { message: message.into() }
    }

    /// Create a BusinessRule error
    pub fn business_rule(message: impl Into<String>) -> Self {
        Self::BusinessRule { message: message.into() }
    }

    /// Create an InvalidToken error
    pub fn invalid_token(message: impl Into<String>) -> Self {
        Self::InvalidToken { message: message.into() }
    }

    // ========== Error inspection methods ==========

    /// Get the error code for this error
    pub fn error_code(&self) -> ApiErrorCode {
        match self {
            Self::Validation { .. } => ApiErrorCode::Validation,
            Self::Unauthorized => ApiErrorCode::Unauthorized,
            Self::InvalidToken { .. } => ApiErrorCode::InvalidToken,
            Self::TokenExpired => ApiErrorCode::TokenExpired,
            Self::Forbidden { .. } => ApiErrorCode::Forbidden,
            Self::NotFound { .. } => ApiErrorCode::NotFound,
            Self::Conflict { .. } => ApiErrorCode::Conflict,
            Self::BusinessRule { .. } => ApiErrorCode::BusinessRule,
            Self::Database { .. } => ApiErrorCode::Database,
            Self::Internal { .. } => ApiErrorCode::Internal,
            Self::Invalid { .. } => ApiErrorCode::Invalid,
        }
    }

    /// Get the error message
    pub fn message(&self) -> String {
        match self {
            Self::Validation { message, .. } => message.clone(),
            Self::Unauthorized => "Please login first".to_string(),
            Self::InvalidToken { message } => message.clone(),
            Self::TokenExpired => "Token expired".to_string(),
            Self::Forbidden { message } => message.clone(),
            Self::NotFound { resource } => format!("{} not found", resource),
            Self::Conflict { resource } => format!("{} already exists", resource),
            Self::BusinessRule { message } => message.clone(),
            Self::Database { message } => message.clone(),
            Self::Internal { message } => message.clone(),
            Self::Invalid { message } => message.clone(),
        }
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> Response<axum::body::Body> {
        let code = self.error_code();
        let status = code.status_code();
        let message = self.message();

        let body = ApiResponse::<()>::error(code.code(), message);
        let json_body = serde_json::to_string(&body).unwrap_or_default();

        let body = json_body.into();

        http::Response::builder()
            .status(status)
            .body(body)
            .unwrap_or_else(|_| {
                let body = "Internal error".into();
                http::Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(body)
                    .unwrap()
            })
    }
}

/// Result type for API operations
pub type ApiResult<T> = Result<T, ApiError>;

// ========== From implementations ==========

// Note: Multipart error conversion would require axum::extract::multipart::MultipartError
// This is implemented in edge-server's upload handler instead for better feature isolation
