//! Crab Edge Server - 分布式餐厅管理系统边缘节点
//!
//! # 架构概述
//!
//! 本模块是 Edge Server 的主入口，提供以下核心功能：
//!
//! - **消息总线** (`message`): 支持 TCP/TLS/Memory 传输的实时消息系统
//! - **数据库** (`db`): 嵌入式 SQLite 存储
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
//! ├── orders/        # 订单事件溯源 (核心引擎)
//! ├── archiving/     # 归档系统 (SQLite + 哈希链验证)
//! ├── order_money/   # 金额计算 (rust_decimal)
//! └── order_sync     # 重连同步协议
//! ```

pub mod api;
pub mod archiving;
pub mod audit;
pub mod auth;
pub mod cloud_sync;
pub mod core;
pub mod db;
pub mod marketing;
pub mod message;
pub mod order_money;
pub mod order_sync;
pub mod orders;
pub mod pricing;
pub mod printing;
pub mod services;
pub mod shifts;
pub mod utils;

// Re-export 公共类型
pub use auth::{CurrentUser, JwtService};
pub use core::{Config, Server, ServerState};
pub use message::{BusMessage, EventType};
pub use orders::{OrderStorage, OrdersManager};
pub use utils::{AppError, AppResult};

// Re-export unified error types from shared
pub use utils::{ApiResponse, ErrorCategory, ErrorCode};

// Re-export logger functions
pub use utils::logger::{cleanup_old_logs, init_logger, init_logger_with_file};

/// 审计日志宏 — 异步记录到 AuditService
///
/// # 用法
///
/// ```ignore
/// audit_log!(
///     state.audit_service,
///     AuditAction::LoginSuccess,
///     "auth", "1",
///     operator_id = Some(1),
///     operator_name = Some("张三".into()),
///     details = serde_json::json!({"ip": "127.0.0.1"})
/// );
/// ```
#[macro_export]
macro_rules! audit_log {
    // 完整版（含 target）
    ($service:expr, $action:expr, $res_type:expr, $res_id:expr,
     operator_id = $op_id:expr, operator_name = $op_name:expr, details = $details:expr, target = $target:expr) => {
        $service
            .log_with_target(
                $action, $res_type, $res_id, $op_id, $op_name, $details, $target,
            )
            .await;
    };
    // 标准版（无 target）
    ($service:expr, $action:expr, $res_type:expr, $res_id:expr,
     operator_id = $op_id:expr, operator_name = $op_name:expr, details = $details:expr) => {
        $service
            .log($action, $res_type, $res_id, $op_id, $op_name, $details)
            .await;
    };
    ($service:expr, $action:expr, $res_type:expr, $res_id:expr, details = $details:expr) => {
        $service
            .log($action, $res_type, $res_id, None, None, $details)
            .await;
    };
    ($service:expr, $action:expr, $res_type:expr, $res_id:expr) => {
        $service
            .log(
                $action,
                $res_type,
                $res_id,
                None,
                None,
                serde_json::json!({}),
            )
            .await;
    };
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
