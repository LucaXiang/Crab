//! Core module for RedCoral POS
//!
//! 包含核心组件:
//! - TenantManager: 多租户证书和会话管理
//! - SessionCache: 员工会话缓存（支持离线登录）
//! - ClientBridge: 统一的客户端桥接层
//! - ApiResponse: 统一的 API 响应格式
//! - ErrorCodes: 标准化错误码

pub mod client_bridge;
pub mod error_codes;
pub mod response;
pub mod session_cache;
pub mod tenant_manager;

pub use client_bridge::{
    AppConfig, AppState, BridgeError, ClientBridge, ClientModeConfig, ModeInfo, ModeType,
    ServerModeConfig,
};
pub use response::{
    ApiResponse, AttributeData, AttributeListData, CategoryData, CategoryListData, DeleteData,
    PrinterData, PrinterListData, ProductAttributeListData, ProductData, ProductListData,
    SpecListData, TagListData,
};
pub use session_cache::{EmployeeSession, LoginMode, SessionCache, SessionCacheError};
pub use tenant_manager::{TenantError, TenantInfo, TenantManager};
