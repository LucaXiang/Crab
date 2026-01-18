//! Core module for RedCoral POS
//!
//! 包含核心组件:
//! - TenantManager: 多租户证书和会话管理
//! - SessionCache: 员工会话缓存（支持离线登录）
//! - ClientBridge: 统一的客户端桥接层
//! - ConnectionMonitor: 连接状态监控和自动重连

pub mod client_bridge;
pub mod connection_monitor;
pub mod session_cache;
pub mod tenant_manager;

pub use client_bridge::{
    AppConfig, BridgeError, ClientBridge, ClientModeConfig, ModeInfo, ModeType,
    ServerModeConfig,
};
pub use connection_monitor::ConnectionMonitor;
pub use session_cache::{EmployeeSession, LoginMode, SessionCache, SessionCacheError};
pub use tenant_manager::{TenantError, TenantInfo, TenantManager};
