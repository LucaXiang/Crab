//! Crab Edge Server - 分布式餐厅管理系统边缘节点
//!
//! # 架构概述
//!
//! 本模块是 Edge Server 的主入口，提供以下核心功能：
//!
//! - **消息总线** (`message`): 支持 TCP/TLS/Memory 传输的实时消息系统
//! - **数据库** (`db`): 嵌入式 SurrealDB 存储
//! - **认证** (`auth`): JWT + Argon2 认证体系
//! - **证书管理** (`services/cert`): mTLS 三层证书体系
//! - **HTTP API** (`api`): RESTful API 接口
//!
//! # 模块结构
//!
//! ```text
//! edge-server/src/
//! ├── core/          # 配置、状态、错误
//! ├── auth/          # JWT 认证、权限
//! ├── services/      # 证书、激活、消息总线
//! ├── api/           # HTTP 路由和处理器
//! ├── utils/         # 工具函数
//! ├── db/            # 数据库层
//! ├── message/       # 消息总线
//! └── orders/        # 订单事件溯源
//! ```

pub mod api;
pub mod auth;
pub mod core;
pub mod db;
pub mod message;
pub mod orders;
pub mod pricing;
pub mod shifts;
pub mod printing;
pub mod services;
pub mod utils;

// Re-export 公共类型
pub use auth::{CurrentUser, JwtService};
pub use core::{Config, Server, ServerState};
pub use message::{BusMessage, EventType};
pub use orders::{OrderStorage, OrdersManager};
pub use pricing::PriceRuleEngine;
pub use utils::{AppError, AppResult};

// Re-export unified error types from shared
pub use utils::{ApiResponse, ErrorCategory, ErrorCode};

// Re-export logger functions
pub use utils::logger::{cleanup_old_logs, init_logger, init_logger_with_file};

// Audit logging macro - 空操作 (1-3 客户端场景不需要审计)
#[macro_export]
macro_rules! audit_log {
    ($($arg:tt)*) => {};
}

// Security logging macro - 支持 tracing 格式说明符
#[macro_export]
macro_rules! security_log {
    ($level:expr, $event:expr, $($key:ident = $value:expr),*) => {
        tracing::info!(
            target: "security",
            level = $level,
            event = $event,
            $($key = $value),*
        );
    };
}

pub fn print_banner() {
    println!(
        r#"
   ______           __
  / ____/________ _/ /_
 / /   / ___/ __ `/ __ \
/ /___/ /  / /_/ / /_/ /
\____/_/   \__,_/_.___/
    ______    __
   / ____/___/ /___ ____
  / __/ / __  / __ `/ _ \
 / /___/ /_/ / /_/ /  __/
/_____/\__,_/\__, /\___/
            /____/
    "#
    );
}
