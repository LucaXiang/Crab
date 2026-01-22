//! API 路由模块
//!
//! # 结构
//!
//! - [`health`] - 健康检查和指标
//! - [`auth`] - 认证相关接口
//! - [`role`] - 角色管理接口
//! - [`upload`] - 文件上传接口
//! - [`tags`] - 标签管理接口
//! - [`categories`] - 分类管理接口
//! - [`products`] - 商品管理接口
//! - [`attributes`] - 属性管理接口
//! - [`zones`] - 区域管理接口
//! - [`tables`] - 桌台管理接口
//! - [`price_rules`] - 价格规则管理接口
//! - [`employees`] - 员工管理接口
//! - [`orders`] - 订单管理接口
//! - [`system_state`] - 系统状态接口

pub mod convert;

pub mod auth;
pub mod health;
pub mod role;
pub mod upload;

// Data models API
pub mod attributes;
pub mod categories;
pub mod employees;
pub mod has_attribute;
pub mod orders;
pub mod price_rules;
pub mod print_destinations;
pub mod products;
pub mod sync;
pub mod system_state;
pub mod tables;
pub mod tags;
pub mod zones;

// Re-export common types for handlers
pub use crate::utils::{AppResponse, AppResult};
