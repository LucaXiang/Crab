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
//! ```
//! edge-server/src/
//! ├── core/          # 配置、状态、错误
//! ├── auth/          # JWT 认证、权限
//! ├── services/      # 证书、激活、消息总线
//! ├── api/           # HTTP 路由和处理器
//! ├── utils/         # 工具函数
//! ├── db/            # 数据库层
//! ├── client/        # 客户端 SDK
//! └── message/       # 消息总线
//! ```

pub mod api;
pub mod auth;
pub mod client;
pub mod core;
pub mod db;
pub mod message;
pub mod services;
pub mod utils;

// Re-export 公共类型
pub use auth::{CurrentUser, JwtService};
pub use client::{ClientInner, CrabClient, MessageClient, Oneshot};
pub use client::{CurrentUserResponse, LoginResponse, UserInfo};
pub use core::{Config, Server, ServerState};
pub use message::{BusMessage, EventType};
pub use utils::AppError as AppErrorType;
pub use utils::{AppResponse, AppResult};

// 为向后兼容性提供 AppError 别名
pub use utils::AppError;

// Re-export logger functions
pub use utils::logger::{cleanup_old_logs, init_logger, init_logger_with_file};

// API response helper macro
#[macro_export]
macro_rules! ok {
    ($data:expr) => {
        axum::Json($crate::AppResponse {
            success: true,
            data: Some($data),
            error: None,
        })
    };
    () => {
        axum::Json($crate::AppResponse {
            success: true,
            data: None,
            error: None,
        })
    };
}

// Audit logging macro - 3参数版本: category, action, target_id
#[macro_export]
macro_rules! audit_log {
    ($category:expr, $action:expr, $target_id:expr) => {
        tracing::info!(
            target: "audit",
            category = $category,
            action = $action,
            target_id = $target_id,
            "Audit log"
        );
    };
    ($category:expr, $action:expr, $target_id:expr, $description:expr) => {
        tracing::info!(
            target: "audit",
            category = $category,
            action = $action,
            target_id = $target_id,
            description = $description,
            "Audit log"
        );
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

pub fn setup_environment() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Get work directory from env or use current directory
    let work_dir = std::env::var("WORK_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    // Create work directory if it doesn't exist
    if !work_dir.exists() {
        std::fs::create_dir_all(&work_dir)?;
        println!("Created work directory: {}", work_dir.display());
    }

    // Change to work directory so relative paths work
    std::env::set_current_dir(&work_dir)?;

    // Create logs directory
    let log_dir = work_dir.join("logs");
    if !log_dir.exists() {
        std::fs::create_dir_all(&log_dir)?;
    }

    // Create certs directory
    let certs_dir = work_dir.join("certs");
    if !certs_dir.exists() {
        std::fs::create_dir_all(&certs_dir)?;
        println!("Created certs directory: {}", certs_dir.display());
    }

    // Initialize logging
    let json_format = std::env::var("LOG_JSON")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);

    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

    init_logger_with_file(
        Some(&log_level),
        Some(json_format),
        Some(log_dir.to_str().unwrap_or("logs")),
    );

    tracing::info!(
        "Environment initialized. WorkDir: {}, LogLevel: {}",
        work_dir.display(),
        log_level
    );

    Ok(())
}
