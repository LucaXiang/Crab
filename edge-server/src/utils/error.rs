//! 统一错误处理
//!
//! 提供应用级错误类型和响应结构：
//! - [`AppError`] - 应用错误枚举
//! - [`AppResponse`] - API 响应结构
//!
//! # 错误码规范
//!
//! | 前缀 | 分类 | 示例 |
//! |------|------|------|
//! | E1xxx | 认证错误 | E3001 未登录 |
//! | E2xxx | 权限错误 | E2001 无权限 |
//! | E3xxx | 认证令牌错误 | E3002 无效令牌 |
//! | Exxxx | 系统错误 | E0001 数据库错误 |
//!
//! # 使用示例
//!
//! ```ignore
//! // 返回错误
//! Err(AppError::not_found("User not found"))
//!
//! // 返回成功响应
//! Ok(Json(AppResponse::success(data)))
//! ```

use axum::{
    Json,
    extract::multipart::MultipartError,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use tracing::error;

/// API 统一响应结构
///
/// ```json
/// {
///   "code": "0000",
///   "message": "success",
///   "data": { ... }
/// }
/// ```
#[derive(Debug, Serialize)]
pub struct AppResponse<T> {
    /// 错误码 (0000 表示成功)
    pub code: String,
    /// 消息
    pub message: String,
    /// 响应数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// 追踪 ID (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

/// 应用错误枚举
///
/// # 错误分类
///
/// | 分类 | 说明 |
/// |------|------|
/// | 认证错误 | 未登录、令牌过期、无效令牌 |
/// | 业务逻辑错误 | 资源不存在、验证失败、规则冲突 |
/// | 系统错误 | 数据库错误、内部错误、无效请求 |
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    // ========== 认证错误 (4xx) ==========
    #[error("Authentication required")]
    /// 未登录 (401)
    Unauthorized,

    #[error("Token expired")]
    /// 令牌过期 (401)
    TokenExpired,

    #[error("Invalid token")]
    /// 无效令牌 (401)
    InvalidToken,

    #[error("Permission denied: {0}")]
    /// 无权限 (403)
    Forbidden(String),

    // ========== 业务逻辑错误 (4xx) ==========
    #[error("Resource not found: {0}")]
    /// 资源不存在 (404)
    NotFound(String),

    #[error("Resource already exists: {0}")]
    /// 资源冲突 (409)
    Conflict(String),

    #[error("Validation failed: {0}")]
    /// 验证失败 (400)
    Validation(String),

    #[error("Business rule violation: {0}")]
    /// 业务规则违反 (400)
    BusinessRule(String),

    // ========== 系统错误 (5xx) ==========
    #[error("Database error: {0}")]
    /// 数据库错误 (500)
    Database(String),

    #[error("Internal server error: {0}")]
    /// 内部错误 (500)
    Internal(String),

    #[error("Invalid request: {0}")]
    /// 无效请求 (400)
    Invalid(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            // Authentication errors (401)
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "E3001", "Please login first"),
            AppError::TokenExpired => (StatusCode::UNAUTHORIZED, "E3003", "Token expired"),
            AppError::InvalidToken => (StatusCode::UNAUTHORIZED, "E3002", "Invalid token"),

            // Authorization errors (403)
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "E2001", msg.as_str()),

            // Not found (404)
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "E0003", msg.as_str()),

            // Conflict (409)
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "E0004", msg.as_str()),

            // Validation (400)
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, "E0002", msg.as_str()),

            // Business rule (422)
            AppError::BusinessRule(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "E0005", msg.as_str())
            }

            // Database errors (500)
            AppError::Database(msg) => {
                error!(target: "database", error = %msg, "Database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, "E9002", "Database error")
            }

            // Internal errors (500)
            AppError::Internal(msg) => {
                error!(target: "internal", error = %msg, "Internal error occurred");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "E9001",
                    "Internal server error",
                )
            }

            // Invalid request (400)
            AppError::Invalid(msg) => (StatusCode::BAD_REQUEST, "E0006", msg.as_str()),
        };

        let body = Json(AppResponse::<()> {
            code: code.to_string(),
            message: message.to_string(),
            data: None,
            trace_id: None, // 边缘场景不需要 trace_id
        });

        (status, body).into_response()
    }
}

impl From<MultipartError> for AppError {
    fn from(e: MultipartError) -> Self {
        AppError::Validation(format!("Multipart error: {}", e))
    }
}

// ========== Helper Constructors ==========

impl AppError {
    /// Create an invalid credentials error with unified message
    /// Used to prevent username enumeration during login
    pub fn invalid_credentials() -> Self {
        Self::Invalid("Invalid username or password".to_string())
    }
}

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
