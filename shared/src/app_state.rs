//! 应用状态类型定义
//!
//! 统一 Server/Client 模式的应用状态，供前端路由守卫使用。

use serde::{Deserialize, Serialize};

use crate::activation::{PlanType, SubscriptionStatus};

// =============================================================================
// 激活失败原因
// =============================================================================

/// 时钟偏移方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockDirection {
    /// 时钟回拨
    Backward,
    /// 时钟前跳
    Forward,
}

/// 需要激活的原因
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "code", content = "details")]
pub enum ActivationRequiredReason {
    /// 首次激活
    FirstTimeSetup,

    /// 证书过期
    CertificateExpired {
        expired_at: i64,
        days_overdue: i64,
    },

    /// 证书即将过期 (警告)
    CertificateExpiringSoon {
        expires_at: i64,
        days_remaining: i64,
    },

    /// 证书无效
    CertificateInvalid { error: String },

    /// 签名验证失败
    SignatureInvalid { component: String, error: String },

    /// 硬件 ID 不匹配
    DeviceMismatch { expected: String, actual: String },

    /// 时钟篡改
    ClockTampering {
        direction: ClockDirection,
        drift_seconds: i64,
        last_verified_at: i64,
    },

    /// Binding 无效
    BindingInvalid { error: String },

    /// Token 过期
    TokenExpired { expired_at: i64 },

    /// 网络错误
    NetworkError {
        error: String,
        can_continue_offline: bool,
    },

    /// 已被吊销
    Revoked { revoked_at: i64, reason: String },
}

impl ActivationRequiredReason {
    /// 是否可以自动恢复
    pub fn can_auto_recover(&self) -> bool {
        matches!(
            self,
            Self::CertificateExpiringSoon { .. }
                | Self::NetworkError {
                    can_continue_offline: true,
                    ..
                }
        )
    }

    /// 获取恢复建议代码 (用于前端 i18n)
    pub fn recovery_hint_code(&self) -> &'static str {
        match self {
            Self::FirstTimeSetup => "first_time_setup",
            Self::CertificateExpired { .. } => "certificate_expired",
            Self::CertificateExpiringSoon { .. } => "certificate_expiring_soon",
            Self::CertificateInvalid { .. } => "certificate_invalid",
            Self::SignatureInvalid { .. } => "signature_invalid",
            Self::DeviceMismatch { .. } => "device_mismatch",
            Self::ClockTampering { .. } => "clock_tampering",
            Self::BindingInvalid { .. } => "binding_invalid",
            Self::TokenExpired { .. } => "token_expired",
            Self::NetworkError {
                can_continue_offline: true,
                ..
            } => "network_error_offline_ok",
            Self::NetworkError {
                can_continue_offline: false,
                ..
            } => "network_error",
            Self::Revoked { .. } => "revoked",
        }
    }
}

// =============================================================================
// 订阅阻止信息
// =============================================================================

/// 订阅阻止详细信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionBlockedInfo {
    pub status: SubscriptionStatus,
    pub plan: PlanType,
    /// Plan 允许的最大门店数，0 = 无限
    #[serde(default)]
    pub max_stores: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expired_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period_days: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period_ends_at: Option<i64>,
    pub in_grace_period: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renewal_url: Option<String>,
    pub user_message: String,
}

// =============================================================================
// 健康检查
// =============================================================================

/// 健康级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthLevel {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// 证书健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateHealth {
    pub status: HealthLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_remaining: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
}

/// 订阅健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionHealth {
    pub status: HealthLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_status: Option<String>,
    /// 签名有效期截止时间 (Unix millis)，0 = 未知
    #[serde(default)]
    pub signature_valid_until: i64,
    pub needs_refresh: bool,
}

/// 网络健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkHealth {
    pub status: HealthLevel,
    pub auth_server_reachable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_connected_at: Option<i64>,
}

/// 数据库健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub status: HealthLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_write_at: Option<i64>,
}

/// 组件健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentsHealth {
    pub certificate: CertificateHealth,
    pub subscription: SubscriptionHealth,
    pub network: NetworkHealth,
    pub database: DatabaseHealth,
}

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// 健康检查响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub overall: HealthLevel,
    pub components: ComponentsHealth,
    pub checked_at: i64,
    pub device_info: DeviceInfo,
}
