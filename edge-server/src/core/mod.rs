//! 核心模块 - 服务器配置、状态和错误定义
//!
//! # 模块结构
//!
//! - [`Config`] - 服务器配置
//! - [`ServerState`] - 服务器状态
//! - [`Server`] - HTTP 服务器
//! - [`ServerError`] - 服务器错误
//! - [`BackgroundTasks`] - 后台任务管理
//! - [`EventRouter`] - 事件路由与分发

pub mod config;
pub mod error;
pub mod event_router;
pub mod server;
pub mod state;
pub mod tasks;

pub use config::Config;
pub use error::{Result, ServerError};
pub use event_router::{EventChannels, EventRouter};
pub use server::Server;
pub use state::ServerState;
pub use tasks::{BackgroundTasks, TaskKind};
