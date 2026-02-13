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
pub mod time;
pub mod types;
pub mod validation;

// Re-export error types from the error module (which re-exports from shared)
pub use error::{ApiResponse, AppError, AppResult, ErrorCategory, ErrorCode};
pub use error::{ok, ok_with_message};

