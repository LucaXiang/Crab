//! API Response wrapper
//!
//! 统一的 API 响应格式，与前端 TypeScript ApiResponse<T> 类型对齐
//! 支持新的 unified error system (shared::error::ErrorCode)

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

// Re-export for convenience
pub use shared::error::ErrorCode;

/// 统一的 API 响应格式
///
/// Compatible with unified error system while maintaining backward compatibility
#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T: Serialize> {
    /// 错误码 (null or 0 = success)
    /// Now using numeric codes from shared::error::ErrorCode
    pub code: Option<u16>,
    /// 消息
    pub message: String,
    /// 数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Context details for i18n
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, Value>>,
}

impl<T: Serialize> ApiResponse<T> {
    /// 创建成功响应
    pub fn success(data: T) -> Self {
        Self {
            code: Some(0),
            message: "success".to_string(),
            data: Some(data),
            details: None,
        }
    }

    /// 创建成功响应（带自定义消息）
    #[allow(dead_code)]
    pub fn success_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            code: Some(0),
            message: message.into(),
            data: Some(data),
            details: None,
        }
    }

    /// Create error response with string code (legacy, for backward compatibility)
    ///
    /// NOTE: This method is kept for backward compatibility with existing code.
    /// Prefer using `error_with_code()` for new code.
    #[deprecated(since = "0.2.0", note = "Use error_with_code() with ErrorCode instead")]
    #[allow(deprecated)]
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        let code_str = code.into();
        // Try parse as number, fallback to 1 (Unknown)
        let code_num = code_str.parse::<u16>().unwrap_or(1);
        Self {
            code: Some(code_num),
            message: message.into(),
            data: None,
            details: None,
        }
    }

    /// Create error response with ErrorCode (new unified error system)
    ///
    /// This is the preferred method for creating error responses.
    /// Uses numeric error codes from `shared::error::ErrorCode`.
    pub fn error_with_code(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: Some(code.code()),
            message: message.into(),
            data: None,
            details: None,
        }
    }
}

impl ApiResponse<()> {
    /// 创建无数据的成功响应
    pub fn ok() -> Self {
        Self {
            code: Some(0),
            message: "success".to_string(),
            data: None,
            details: None,
        }
    }
}

/// 从 Result 转换为 ApiResponse
impl<T: Serialize> From<Result<T, String>> for ApiResponse<T> {
    fn from(result: Result<T, String>) -> Self {
        match result {
            Ok(data) => ApiResponse::success(data),
            Err(e) => Self {
                code: Some(ErrorCode::Unknown.code()),
                message: e,
                data: None,
                details: None,
            },
        }
    }
}

// ============ 列表数据包装 (与前端类型对齐) ============

/// Tags 列表
#[derive(Debug, Clone, Serialize)]
pub struct TagListData {
    pub tags: Vec<shared::models::Tag>,
}

/// Categories 列表
#[derive(Debug, Clone, Serialize)]
pub struct CategoryListData {
    pub categories: Vec<shared::models::Category>,
}

/// 单个 Category
#[derive(Debug, Clone, Serialize)]
pub struct CategoryData {
    pub category: shared::models::Category,
}

/// Products 列表
#[derive(Debug, Clone, Serialize)]
pub struct ProductListData {
    pub products: Vec<shared::models::Product>,
}

/// 单个 Product
#[derive(Debug, Clone, Serialize)]
pub struct ProductData {
    pub product: shared::models::Product,
}

/// 完整 Product (含 specs, attributes, tags)
#[derive(Debug, Clone, Serialize)]
pub struct ProductFullData {
    pub product: shared::models::ProductFull,
}

/// Attributes 列表
#[derive(Debug, Clone, Serialize)]
pub struct AttributeListData {
    pub templates: Vec<shared::models::Attribute>,
}

/// 单个 Attribute (template)
#[derive(Debug, Clone, Serialize)]
pub struct AttributeData {
    pub template: shared::models::Attribute,
}

/// 删除响应
#[derive(Debug, Clone, Serialize)]
pub struct DeleteData {
    pub deleted: bool,
}

impl DeleteData {
    pub fn success() -> Self {
        Self { deleted: true }
    }
}

/// Product Attributes 列表
#[derive(Debug, Clone, Serialize)]
pub struct ProductAttributeListData {
    pub attributes: Vec<shared::models::AttributeBinding>,
}

// ============ Print Destinations ============

/// Print Destinations 列表
#[derive(Debug, Clone, Serialize)]
pub struct PrintDestinationListData {
    pub print_destinations: Vec<shared::models::PrintDestination>,
}

/// 单个 PrintDestination
#[derive(Debug, Clone, Serialize)]
pub struct PrintDestinationData {
    pub print_destination: shared::models::PrintDestination,
}

// ============ Zones & Tables ============

/// Zones 列表
#[derive(Debug, Clone, Serialize)]
pub struct ZoneListData {
    pub zones: Vec<shared::models::Zone>,
}

/// Tables 列表
#[derive(Debug, Clone, Serialize)]
pub struct TableListData {
    pub tables: Vec<shared::models::DiningTable>,
}

// ============ Roles ============

/// Role entity (Local definition until available in shared)
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct Role {
    pub id: Option<String>,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub is_active: bool,
}

/// Role Permission entity
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct RolePermission {
    pub role_id: String,
    pub permission: String,
}

/// Roles 列表
#[derive(Debug, Clone, Serialize)]
pub struct RoleListData {
    pub roles: Vec<Role>,
}

/// Role Permissions 列表
#[derive(Debug, Clone, Serialize)]
pub struct RolePermissionListData {
    pub permissions: Vec<RolePermission>,
}

// ============ Auth ============

/// 认证数据
#[derive(Debug, Clone, Serialize)]
pub struct AuthData {
    pub session: Option<super::EmployeeSession>,
    pub mode: super::LoginMode,
}

// ============ Orders ============

/// Orders 列表
#[derive(Debug, Clone, Serialize)]
pub struct OrderListData {
    pub orders: Vec<shared::models::Order>,
}

// ============ System ============

/// Employees 列表
#[derive(Debug, Clone, Serialize)]
pub struct EmployeeListData {
    pub employees: Vec<shared::models::Employee>,
}

/// Price Rules 列表
#[derive(Debug, Clone, Serialize)]
pub struct PriceRuleListData {
    pub rules: Vec<shared::models::PriceRule>,
}

/// Order Snapshots 列表
#[derive(Debug, Clone, Serialize)]
pub struct OrderSnapshotListData {
    pub snapshots: Vec<shared::order::OrderSnapshot>,
}

/// Order Events 列表
#[derive(Debug, Clone, Serialize)]
pub struct OrderEventListData {
    pub events: Vec<shared::order::OrderEvent>,
}

/// Order List Response with pagination
#[derive(Debug, Clone, Serialize)]
pub struct FetchOrderListResponse {
    pub orders: Vec<shared::models::Order>,
    pub total: i64,
    pub page: i32,
}

// ============ Tenants ============

/// Tenants 列表
#[derive(Debug, Clone, Serialize)]
pub struct TenantListData {
    pub tenants: Vec<super::TenantInfo>,
}

// ============ App Config ============

/// 应用配置响应
#[derive(Debug, Clone, Serialize)]
pub struct AppConfigResponse {
    pub current_mode: super::ModeType,
    pub current_tenant: Option<String>,
    pub server_config: super::ServerModeConfig,
    pub client_config: Option<super::ClientModeConfig>,
    pub known_tenants: Vec<String>,
}
