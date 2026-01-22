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

    /// Message bus error.
    #[error("Message error: {0}")]
    Message(#[from] MessageError),
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
                Some(status) if status == reqwest::StatusCode::UNAUTHORIZED => {
                    ClientError::Unauthorized("HTTP 401".into())
                }
                Some(status) if status == reqwest::StatusCode::FORBIDDEN => {
                    ClientError::Forbidden("HTTP 403".into())
                }
                Some(status) if status == reqwest::StatusCode::NOT_FOUND => {
                    ClientError::NotFound("HTTP 404".into())
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
// Legacy compatibility - MessageError
// ============================================================================

/// Legacy error type for message operations.
/// Deprecated: Use `ClientError` instead.
#[derive(Debug, Error)]
pub enum MessageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Request timed out: {0}")]
    Timeout(String),

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("Protocol error: {0}")]
    Protocol(String),
}

impl From<tokio::sync::broadcast::error::RecvError> for MessageError {
    fn from(e: tokio::sync::broadcast::error::RecvError) -> Self {
        MessageError::Connection(e.to_string())
    }
}

impl From<serde_json::Error> for MessageError {
    fn from(e: serde_json::Error) -> Self {
        MessageError::InvalidMessage(e.to_string())
    }
}

// ============================================================================
// Result type aliases
// ============================================================================

/// Result type for client operations.
pub type ClientResult<T> = Result<T, ClientError>;

/// Result type for message operations (legacy).
pub type MessageResult<T> = Result<T, MessageError>;
