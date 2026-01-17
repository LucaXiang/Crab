//! API 路由模块
//!
//! # 结构
//!
//! - [`health`] - 健康检查和指标
//! - [`auth`] - 认证相关接口
//! - [`role`] - 角色管理接口
//! - [`upload`] - 文件上传接口

pub mod auth;
pub mod health;
pub mod role;
pub mod upload;

// Re-export common types for handlers
pub use crate::utils::{AppResponse, AppResult};
