//! Core module for RedCoral POS
//!
//! 包含核心组件:
//! - TenantManager: 多租户证书和会话管理
//! - SessionCache: 员工会话缓存（支持离线登录）
//! - ClientBridge: 统一的客户端桥接层
//! - ApiResponse: 统一的 API 响应格式

pub mod bridge;
pub mod image_cache;
pub mod response;
pub mod session_cache;
pub mod tenant_manager;

pub use bridge::{
    AppConfig, AppState, BridgeError, ClientBridge, ClientModeConfig, ModeInfo, ModeType,
    ServerModeConfig,
};
pub use response::{
    ApiResponse, AppConfigResponse, AttributeData, AttributeListData, AuthData, CategoryData,
    CategoryListData, DeleteData, EmployeeListData, OrderEventListData, OrderSnapshotListData,
    PriceRuleListData, PrintDestinationData, PrintDestinationListData, ProductAttributeListData,
    ProductData, ProductFullData, ProductListData, Role, RoleListData, RolePermission,
    RolePermissionListData, TableListData, TagListData, TenantListData, ZoneListData,
};
pub use session_cache::{EmployeeSession, LoginMode, SessionCache, SessionCacheError};
pub use tenant_manager::{TenantError, TenantInfo, TenantManager};
