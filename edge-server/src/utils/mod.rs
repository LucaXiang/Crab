//! 工具模块 - 通用工具函数和类型
//!
//! # 内容
//!
//! - [`AppError`] - 应用错误类型 (from shared::error)
//! - [`ApiResponse`] - API 响应结构 (from shared::error)
//! - 日志等工具

pub mod error;
pub mod logger;
pub mod result;
pub mod types;

// Re-export error types from the error module (which re-exports from shared)
pub use error::{ApiResponse, AppError, AppResult, ErrorCategory, ErrorCode};
pub use error::{ok, ok_with_message};

/// Legacy API 响应结构
///
/// Kept for backward compatibility with existing code that uses the old format.
/// New code should use `ApiResponse` from `shared::error`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> AppResponse<T> {
    /// 创建成功响应
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// 创建错误响应
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}
