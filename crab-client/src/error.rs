//! Client error types

use thiserror::Error;

/// Client error type
#[derive(Debug, Error)]
pub enum ClientError {
    /// HTTP request failed
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Invalid response format
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Authentication required
    #[error("Authentication required")]
    Unauthorized,

    /// Permission denied
    #[error("Permission denied: {0}")]
    Forbidden(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type for client operations
pub type ClientResult<T> = Result<T, ClientError>;
