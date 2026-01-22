//! Error types and API response structures

use super::codes::ErrorCode;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

/// Application error with structured error code and details
///
/// This is the primary error type for the Crab framework, providing:
/// - Standardized error codes via [`ErrorCode`]
/// - Human-readable messages
/// - Optional structured details for debugging
#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct AppError {
    /// The error code identifying the type of error
    pub code: ErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Optional additional details (field-level errors, context, etc.)
    pub details: Option<HashMap<String, Value>>,
}

impl AppError {
    /// Create a new error with the default message for the error code
    pub fn new(code: ErrorCode) -> Self {
        Self {
            message: code.message().to_string(),
            code,
            details: None,
        }
    }

    /// Create a new error with a custom message
    pub fn with_message(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    /// Add a detail entry to this error
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.details
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    /// Get the HTTP status code for this error
    pub fn http_status(&self) -> StatusCode {
        self.code.http_status()
    }

    // ==================== Convenience constructors ====================

    /// Create a validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::ValidationFailed, msg)
    }

    /// Create a not found error
    pub fn not_found(resource: impl Into<String>) -> Self {
        let r = resource.into();
        Self::with_message(ErrorCode::NotFound, format!("{} not found", r))
            .with_detail("resource", r)
    }

    /// Create a not authenticated error
    pub fn not_authenticated() -> Self {
        Self::new(ErrorCode::NotAuthenticated)
    }

    /// Create a permission denied error
    pub fn permission_denied(msg: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::PermissionDenied, msg)
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::InternalError, msg)
    }

    /// Create a database error
    pub fn database(msg: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::DatabaseError, msg)
    }

    /// Create an invalid request error
    pub fn invalid_request(msg: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::InvalidRequest, msg)
    }

    /// Create an already exists error
    pub fn already_exists(resource: impl Into<String>) -> Self {
        let r = resource.into();
        Self::with_message(ErrorCode::AlreadyExists, format!("{} already exists", r))
            .with_detail("resource", r)
    }

    /// Create an invalid request error (alias for invalid_request)
    pub fn invalid(msg: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::InvalidRequest, msg)
    }

    /// Create a forbidden/permission denied error (alias for permission_denied)
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::PermissionDenied, msg)
    }

    /// Create an invalid token error
    pub fn invalid_token(msg: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::TokenInvalid, msg)
    }

    /// Create a token expired error
    pub fn token_expired() -> Self {
        Self::new(ErrorCode::TokenExpired)
    }

    /// Create an unauthorized error (alias for not_authenticated)
    pub fn unauthorized() -> Self {
        Self::new(ErrorCode::NotAuthenticated)
    }

    /// Create an invalid credentials error
    pub fn invalid_credentials() -> Self {
        Self::new(ErrorCode::InvalidCredentials)
    }

    /// Create a conflict error
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::AlreadyExists, msg)
    }

    /// Create a business rule error
    pub fn business_rule(msg: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::ValidationFailed, msg)
    }

    /// Create a client disconnected error
    pub fn client_disconnected() -> Self {
        Self::new(ErrorCode::ClientDisconnected)
    }
}

/// Unified API response structure
///
/// Provides a consistent response format for all API endpoints:
/// - `code`: Error code (0 for success)
/// - `message`: Human-readable message
/// - `data`: Response payload (on success)
/// - `details`: Additional error details (on failure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Error code (0 for success, non-zero for errors)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<u16>,
    /// Human-readable message
    pub message: String,
    /// Response data (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Additional error details (present on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, Value>>,
}

impl<T> ApiResponse<T> {
    /// Create a success response with data
    pub fn success(data: T) -> Self {
        Self {
            code: Some(0),
            message: "OK".to_string(),
            data: Some(data),
            details: None,
        }
    }

    /// Create a success response with custom message and data
    pub fn success_with_message(message: impl Into<String>, data: T) -> Self {
        Self {
            code: Some(0),
            message: message.into(),
            data: Some(data),
            details: None,
        }
    }
}

impl ApiResponse<()> {
    /// Create a success response without data
    pub fn ok() -> Self {
        Self {
            code: Some(0),
            message: "OK".to_string(),
            data: None,
            details: None,
        }
    }

    /// Create an error response from an AppError
    pub fn error(err: &AppError) -> Self {
        Self {
            code: Some(err.code.code()),
            message: err.message.clone(),
            data: None,
            details: err.details.clone(),
        }
    }

    /// Create an error response from code and message
    pub fn error_with_message(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: Some(code.code()),
            message: message.into(),
            data: None,
            details: None,
        }
    }
}

impl<T> From<AppError> for ApiResponse<T> {
    fn from(err: AppError) -> Self {
        Self {
            code: Some(err.code.code()),
            message: err.message,
            data: None,
            details: err.details,
        }
    }
}

/// Type alias for Result with AppError
pub type AppResult<T> = Result<T, AppError>;

// ===== Axum Integration =====

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        use axum::Json;

        let status = self.http_status();
        let body = ApiResponse::<()>::error(&self);

        // Log system errors
        if matches!(self.code.category(), super::category::ErrorCategory::System) {
            tracing::error!(
                code = %self.code,
                message = %self.message,
                "System error occurred"
            );
        }

        (status, Json(body)).into_response()
    }
}

impl<T: Serialize> axum::response::IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        use super::codes::ErrorCode;
        use axum::Json;

        let status = if self.code == Some(0) || self.code.is_none() {
            http::StatusCode::OK
        } else {
            ErrorCode::try_from(self.code.unwrap_or(1))
                .map(|c| c.http_status())
                .unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR)
        };

        (status, Json(self)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_error_new() {
        let err = AppError::new(ErrorCode::NotFound);
        assert_eq!(err.code, ErrorCode::NotFound);
        assert_eq!(err.message, "Resource not found");
        assert!(err.details.is_none());
    }

    #[test]
    fn test_app_error_with_message() {
        let err = AppError::with_message(ErrorCode::ValidationFailed, "Invalid email format");
        assert_eq!(err.code, ErrorCode::ValidationFailed);
        assert_eq!(err.message, "Invalid email format");
    }

    #[test]
    fn test_app_error_with_detail() {
        let err = AppError::validation("Missing required fields")
            .with_detail("field", "email")
            .with_detail("reason", "required");

        assert_eq!(err.code, ErrorCode::ValidationFailed);
        let details = err.details.unwrap();
        assert_eq!(details.get("field").unwrap(), "email");
        assert_eq!(details.get("reason").unwrap(), "required");
    }

    #[test]
    fn test_app_error_http_status() {
        assert_eq!(
            AppError::new(ErrorCode::NotFound).http_status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AppError::new(ErrorCode::NotAuthenticated).http_status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::new(ErrorCode::PermissionDenied).http_status(),
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn test_app_error_convenience_constructors() {
        let err = AppError::not_found("User");
        assert_eq!(err.code, ErrorCode::NotFound);
        assert_eq!(err.message, "User not found");
        assert!(err.details.as_ref().unwrap().contains_key("resource"));

        let err = AppError::validation("Invalid input");
        assert_eq!(err.code, ErrorCode::ValidationFailed);
        assert_eq!(err.message, "Invalid input");

        let err = AppError::not_authenticated();
        assert_eq!(err.code, ErrorCode::NotAuthenticated);

        let err = AppError::permission_denied("Admin only");
        assert_eq!(err.code, ErrorCode::PermissionDenied);
        assert_eq!(err.message, "Admin only");

        let err = AppError::internal("Something went wrong");
        assert_eq!(err.code, ErrorCode::InternalError);

        let err = AppError::database("Connection failed");
        assert_eq!(err.code, ErrorCode::DatabaseError);
    }

    #[test]
    fn test_app_error_display() {
        let err = AppError::with_message(ErrorCode::NotFound, "Order not found");
        assert_eq!(format!("{}", err), "Order not found");
    }

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success(42);
        assert_eq!(response.code, Some(0));
        assert_eq!(response.message, "OK");
        assert_eq!(response.data, Some(42));
        assert!(response.details.is_none());
    }

    #[test]
    fn test_api_response_ok() {
        let response = ApiResponse::<()>::ok();
        assert_eq!(response.code, Some(0));
        assert_eq!(response.message, "OK");
        assert!(response.data.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let err =
            AppError::with_message(ErrorCode::NotFound, "User not found").with_detail("id", "123");
        let response = ApiResponse::<()>::error(&err);

        assert_eq!(response.code, Some(3)); // NotFound = 3
        assert_eq!(response.message, "User not found");
        assert!(response.data.is_none());
        assert!(response.details.is_some());
    }

    #[test]
    fn test_api_response_from_error() {
        let err = AppError::new(ErrorCode::InternalError);
        let response: ApiResponse<String> = err.into();

        assert_eq!(response.code, Some(9001));
        assert_eq!(response.message, "Internal server error");
        assert!(response.data.is_none());
    }

    #[test]
    fn test_api_response_serialize() {
        let response = ApiResponse::success("hello");
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"code\":0"));
        assert!(json.contains("\"message\":\"OK\""));
        assert!(json.contains("\"data\":\"hello\""));
    }

    #[test]
    fn test_api_response_deserialize() {
        let json = r#"{"code":0,"message":"OK","data":42}"#;
        let response: ApiResponse<i32> = serde_json::from_str(json).unwrap();
        assert_eq!(response.code, Some(0));
        assert_eq!(response.data, Some(42));
    }
}
