//! 核心模块 - 服务器配置、状态和错误定义
//!
//! # 模块结构
//!
//! - [`Config`] - 服务器配置
//! - [`ServerState`] - 服务器状态
//! - [`Server`] - HTTP 服务器
//! - [`ServerError`] - 服务器错误

pub mod config;
pub mod error;
pub mod server;
pub mod state;

pub use config::Config;
pub use error::{Result, ServerError};
pub use server::Server;
pub use state::ServerState;
