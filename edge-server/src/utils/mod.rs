//! 工具模块 - 通用工具函数和类型
//!
//! # 内容
//!
//! - [`AppError`] - 应用错误类型
//! - [`AppResponse`] - API 响应结构
//! - 日志等工具

pub mod logger;
pub mod result;
pub mod types;

// 错误类型 - 从 shared 模块导入
pub use result::AppResult;
pub use shared::error::ApiError as AppError;

/// API 响应结构
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
