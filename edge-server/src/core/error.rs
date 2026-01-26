use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Resource not found")]
    NotFound,

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match &self {
            ServerError::NotFound => (StatusCode::NOT_FOUND, "not_found", self.to_string()),
            ServerError::Validation(msg) => {
                (StatusCode::BAD_REQUEST, "validation_error", msg.clone())
            }
            ServerError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "unauthorized", self.to_string())
            }
            ServerError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", self.to_string()),
            ServerError::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg.clone()),
            ServerError::Internal(err) => {
                // 记录内部错误但不暴露详细信息
                tracing::error!(error = ?err, "Internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "An internal error occurred".to_string(),
                )
            }
        };

        let body = ErrorResponse {
            error: error_type.to_string(),
            message,
            details: None,
        };

        (status, Json(body)).into_response()
    }
}

/// 处理器的 Result 类型别名
pub type Result<T> = std::result::Result<T, ServerError>;
