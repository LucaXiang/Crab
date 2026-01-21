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
//! - [`kitchen_printers`] - 厨打管理接口
//! - [`employees`] - 员工管理接口
//! - [`orders`] - 订单管理接口
//! - [`system_state`] - 系统状态接口

pub mod convert;

pub mod auth;
pub mod health;
pub mod role;
pub mod upload;

// Data models API
pub mod tags;
pub mod categories;
pub mod products;
pub mod attributes;
pub mod has_attribute;
pub mod zones;
pub mod tables;
pub mod price_rules;
pub mod kitchen_printers;
pub mod employees;
pub mod orders;
pub mod system_state;

// Re-export common types for handlers
pub use crate::utils::{AppResponse, AppResult};
