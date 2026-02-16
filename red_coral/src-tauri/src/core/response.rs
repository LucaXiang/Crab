//! API Response wrapper
//!
//! 统一的 API 响应格式，与前端 TypeScript ApiResponse<T> 类型对齐
//! 支持新的 unified error system (shared::error::ErrorCode)

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

use super::bridge::BridgeError;

// Re-export for convenience
pub use shared::error::ErrorCode;

/// 统一的 API 响应格式
///
/// 使用 shared::error::ErrorCode 统一错误码
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

    /// 创建错误响应
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

/// Map ClientError to the most specific ErrorCode
fn client_error_to_code(err: &crab_client::ClientError) -> ErrorCode {
    use crab_client::ClientError;
    match err {
        // Connection
        ClientError::Connection(_) => ErrorCode::BridgeConnectionFailed,
        ClientError::ConnectionClosed(_) => ErrorCode::ClientDisconnected,
        ClientError::Request(_) => ErrorCode::NetworkError,
        ClientError::Timeout(_) => ErrorCode::TimeoutError,
        // TLS / Certificate
        ClientError::Tls(_) | ClientError::Certificate(_) => ErrorCode::CertificateInvalid,
        ClientError::NoCertificates => ErrorCode::BridgeNotConnected,
        ClientError::CertificateExpired => ErrorCode::LicenseExpired,
        // Auth
        ClientError::Auth(_) => ErrorCode::InvalidCredentials,
        ClientError::Unauthorized(_) => ErrorCode::NotAuthenticated,
        ClientError::SessionExpired => ErrorCode::SessionExpired,
        ClientError::Forbidden(_) => ErrorCode::PermissionDenied,
        // Request-level
        ClientError::NotFound(_) => ErrorCode::NotFound,
        ClientError::Validation(_) => ErrorCode::ValidationFailed,
        ClientError::NotSupported(_) | ClientError::InvalidState(_) => ErrorCode::InvalidRequest,
        ClientError::Config(_) => ErrorCode::ConfigError,
        // API error (handled separately before this function is called)
        ClientError::Api { .. } => ErrorCode::InternalError,
        // Protocol / serialization
        ClientError::Serialization(_)
        | ClientError::InvalidMessage(_)
        | ClientError::InvalidResponse(_)
        | ClientError::Protocol(_) => ErrorCode::InternalError,
        ClientError::Io(_) | ClientError::Internal(_) => ErrorCode::InternalError,
    }
}

/// Map TenantError to the most specific ErrorCode
fn tenant_error_to_code(err: &super::tenant_manager::TenantError) -> ErrorCode {
    use super::tenant_manager::TenantError;
    match err {
        TenantError::NotFound(_) => ErrorCode::TenantNotFound,
        TenantError::NoTenantSelected => ErrorCode::TenantNotSelected,
        TenantError::Certificate(_) => ErrorCode::CertificateInvalid,
        TenantError::Network(_) => ErrorCode::NetworkError,
        TenantError::CredentialsInvalid(_) => ErrorCode::TenantCredentialsInvalid,
        TenantError::NoSubscription(_) => ErrorCode::TenantNoSubscription,
        TenantError::AuthServerError(_) => ErrorCode::AuthServerError,
        TenantError::AuthFailed(_) => ErrorCode::ActivationFailed,
        TenantError::DeviceLimitReached(_) => ErrorCode::DeviceLimitReached,
        TenantError::ClientLimitReached(_) => ErrorCode::ClientLimitReached,
        TenantError::OfflineNotAvailable(_) => ErrorCode::NotAuthenticated,
        TenantError::SessionCache(_) | TenantError::Io(_) => ErrorCode::InternalError,
    }
}

/// 从 BridgeError 创建错误响应
/// 会自动提取服务端返回的 error code，非 API 错误映射到最匹配的 ErrorCode
impl<T: Serialize> ApiResponse<T> {
    pub fn from_bridge_error(err: BridgeError) -> Self {
        match &err {
            BridgeError::Client(client_err) => {
                // API 错误：直接保留服务端的 error code + details
                if let crab_client::ClientError::Api {
                    code,
                    message,
                    details,
                } = client_err
                {
                    return Self {
                        code: Some(*code as u16),
                        message: message.clone(),
                        data: None,
                        details: details.as_ref().map(|d| {
                            let mut map = HashMap::new();
                            if let Some(obj) = d.as_object() {
                                for (k, v) in obj {
                                    map.insert(k.clone(), v.clone());
                                }
                            }
                            map
                        }),
                    };
                }
                // 非 API 错误：映射到正确的 ErrorCode
                let code = client_error_to_code(client_err);
                Self {
                    code: Some(code.code()),
                    message: client_err.to_string(),
                    data: None,
                    details: None,
                }
            }
            BridgeError::NotInitialized => Self {
                code: Some(ErrorCode::BridgeNotInitialized.code()),
                message: err.to_string(),
                data: None,
                details: None,
            },
            BridgeError::NotAuthenticated => Self {
                code: Some(ErrorCode::NotAuthenticated.code()),
                message: err.to_string(),
                data: None,
                details: None,
            },
            BridgeError::Config(_) => Self {
                code: Some(ErrorCode::ConfigError.code()),
                message: err.to_string(),
                data: None,
                details: None,
            },
            BridgeError::Tenant(tenant_err) => {
                let code = tenant_error_to_code(tenant_err);
                Self {
                    code: Some(code.code()),
                    message: err.to_string(),
                    data: None,
                    details: None,
                }
            }
            BridgeError::NotImplemented(_) | BridgeError::AlreadyRunning(_) => Self {
                code: Some(ErrorCode::InvalidRequest.code()),
                message: err.to_string(),
                data: None,
                details: None,
            },
            BridgeError::Server(_) | BridgeError::Io(_) => Self {
                code: Some(ErrorCode::InternalError.code()),
                message: err.to_string(),
                data: None,
                details: None,
            },
        }
    }
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

/// Role Permission entity (constructed from API response)
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct RolePermission {
    pub role_id: i64,
    pub permission: String,
}

// ============ Auth ============

/// 认证数据
#[derive(Debug, Clone, Serialize)]
pub struct AuthData {
    pub session: Option<super::EmployeeSession>,
    pub mode: super::LoginMode,
}

// ============ Tenants ============

/// 激活结果
#[derive(Debug, Clone, Serialize)]
pub struct ActivationResultData {
    pub tenant_id: String,
    /// 订阅状态 (来自 auth server)，null 表示无订阅信息
    pub subscription_status: Option<String>,
    /// Quota 信息 (设备数已满时返回)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_info: Option<shared::activation::QuotaInfo>,
}

// ============ App Config ============

/// 应用配置响应
#[derive(Debug, Clone, Serialize)]
pub struct AppConfigResponse {
    pub current_mode: Option<super::ModeType>,
    pub current_tenant: Option<String>,
    pub server_config: super::ServerModeConfig,
    pub client_config: Option<super::ClientModeConfig>,
    pub known_tenants: Vec<String>,
}
