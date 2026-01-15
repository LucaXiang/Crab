//! 服务器中间件模块
//!
//! 包含服务器的所有 HTTP 中间件

pub mod logging;
pub use logging::logging_middleware;
