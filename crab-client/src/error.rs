//! Unified error types for crab-client.
//!
//! This module provides a single `ClientError` type that covers all error
//! cases across HTTP, message bus, and certificate operations.

use thiserror::Error;

/// Unified error type for all client operations.
#[derive(Debug, Error)]
pub enum ClientError {
    // ===== Configuration Errors =====
    /// Invalid client configuration.
    #[error("Invalid configuration: {0}")]
    Config(String),

    // ===== Connection Errors =====
    /// Failed to establish connection.
    #[error("Connection failed: {0}")]
    Connection(String),

    /// TLS handshake failed.
    #[error("TLS error: {0}")]
    Tls(String),

    /// Connection was closed unexpectedly.
    #[error("Connection closed: {0}")]
    ConnectionClosed(String),

    // ===== Authentication Errors =====
    /// Authentication failed (wrong credentials).
    #[error("Authentication failed: {0}")]
    Auth(String),

    /// Session has expired.
    #[error("Session expired")]
    SessionExpired,

    /// Not authorized to perform this action.
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Access forbidden.
    #[error("Forbidden: {0}")]
    Forbidden(String),

    // ===== Request Errors =====
    /// Request timed out.
    #[error("Request timeout: {0}")]
    Timeout(String),

    /// Request failed.
    #[error("Request failed: {0}")]
    Request(String),

    /// Resource not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Validation error.
    #[error("Validation error: {0}")]
    Validation(String),

    /// API error with code (from server).
    #[error("API error {code}: {message}")]
    Api {
        code: i32,
        message: String,
        details: Option<serde_json::Value>,
    },

    // ===== Protocol Errors =====
    /// Invalid message format.
    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    /// Protocol error.
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Invalid response from server.
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    // ===== Certificate Errors =====
    /// Certificate error.
    #[error("Certificate error: {0}")]
    Certificate(String),

    /// No cached certificates available.
    #[error("No cached certificates. Call setup() first.")]
    NoCertificates,

    /// Certificate has expired.
    #[error("Certificate expired")]
    CertificateExpired,

    // ===== State Errors =====
    /// Operation not supported in current mode.
    #[error("Operation not supported: {0}")]
    NotSupported(String),

    /// Client is in wrong state for this operation.
    #[error("Invalid state: {0}")]
    InvalidState(String),

    // ===== Internal Errors =====
    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),

}

// ============================================================================
// From implementations
// ============================================================================

impl From<reqwest::Error> for ClientError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            ClientError::Timeout(e.to_string())
        } else if e.is_connect() {
            ClientError::Connection(e.to_string())
        } else if e.is_status() {
            match e.status() {
                Some(status) if status == reqwest::StatusCode::BAD_REQUEST => {
                    ClientError::Validation(format!("HTTP 400: {e}"))
                }
                Some(status) if status == reqwest::StatusCode::UNAUTHORIZED => {
                    ClientError::Unauthorized("HTTP 401".into())
                }
                Some(status) if status == reqwest::StatusCode::FORBIDDEN => {
                    ClientError::Forbidden("HTTP 403".into())
                }
                Some(status) if status == reqwest::StatusCode::NOT_FOUND => {
                    ClientError::NotFound("HTTP 404".into())
                }
                Some(status) if status == reqwest::StatusCode::CONFLICT => {
                    ClientError::Request(format!("HTTP 409 Conflict: {e}"))
                }
                Some(status) if status == reqwest::StatusCode::TOO_MANY_REQUESTS => {
                    ClientError::Request(format!("HTTP 429 Too Many Requests: {e}"))
                }
                Some(status) if status == reqwest::StatusCode::SERVICE_UNAVAILABLE => {
                    ClientError::Connection(format!("HTTP 503 Service Unavailable: {e}"))
                }
                _ => ClientError::Request(e.to_string()),
            }
        } else {
            ClientError::Request(e.to_string())
        }
    }
}

impl From<serde_json::Error> for ClientError {
    fn from(e: serde_json::Error) -> Self {
        ClientError::Serialization(e.to_string())
    }
}

impl From<tokio::sync::broadcast::error::RecvError> for ClientError {
    fn from(e: tokio::sync::broadcast::error::RecvError) -> Self {
        ClientError::Connection(e.to_string())
    }
}

// ============================================================================
// HTTP Response Handling (shared by NetworkHttpClient and Remote mode)
// ============================================================================

/// 服务端返回的错误响应格式
#[derive(serde::Deserialize)]
pub(crate) struct ApiErrorResponse {
    pub code: i32,
    pub message: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
}

/// 处理 reqwest::Response，统一错误映射。
///
/// NetworkHttpClient 和 CrabClient<Remote, Authenticated> 共用此函数，
/// 确保两种模式的错误类型完全一致。
pub(crate) async fn handle_reqwest_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> ClientResult<T> {
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await?;
        // 尝试解析为 API 错误响应
        if let Ok(api_err) = serde_json::from_str::<ApiErrorResponse>(&text) {
            return Err(ClientError::Api {
                code: api_err.code,
                message: api_err.message,
                details: api_err.details,
            });
        }
        // 降级到 HTTP 状态码映射
        return match status {
            reqwest::StatusCode::UNAUTHORIZED => {
                Err(ClientError::Unauthorized("Unauthorized".into()))
            }
            reqwest::StatusCode::FORBIDDEN => Err(ClientError::Forbidden(text)),
            reqwest::StatusCode::NOT_FOUND => Err(ClientError::NotFound(text)),
            reqwest::StatusCode::BAD_REQUEST => Err(ClientError::Validation(text)),
            _ => Err(ClientError::Internal(text)),
        };
    }
    response
        .json()
        .await
        .map_err(|e| ClientError::InvalidResponse(format!("JSON parse error: {}", e)))
}

// ============================================================================
// Result type aliases
// ============================================================================

/// Result type for client operations.
pub type ClientResult<T> = Result<T, ClientError>;
