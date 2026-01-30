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

pub mod auth;
pub mod health;
pub mod role;
pub mod upload;

// Data models API
pub mod attributes;
pub mod categories;
pub mod employees;
pub mod has_attribute;
pub mod kitchen_orders;
pub mod orders;
pub mod price_rules;
pub mod print_config;
pub mod print_destinations;
pub mod products;
pub mod store_info;
pub mod label_template;
pub mod sync;
pub mod system_state;
pub mod tables;
pub mod tags;
pub mod zones;

// Operations (班次与日结)
pub mod shifts;
pub mod daily_reports;

// Analytics (数据统计)
pub mod statistics;

// Archive (归档验证)
pub mod archive_verify;

// Audit (审计日志)
pub mod audit_log;

// System Issues (系统问题)
pub mod system_issues;

// Re-export common types for handlers
pub use crate::utils::AppResult;
