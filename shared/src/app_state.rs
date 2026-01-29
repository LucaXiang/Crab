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
    /// 获取恢复建议 (完整文本)
    pub fn recovery_hint(&self) -> &'static str {
        match self {
            Self::FirstTimeSetup => "输入管理员提供的凭据完成激活",
            Self::CertificateExpired { .. } => "请重新激活设备以更新证书",
            Self::CertificateExpiringSoon { .. } => "建议尽快重新激活以更新证书",
            Self::CertificateInvalid { .. } => "证书文件损坏，请重新激活",
            Self::SignatureInvalid { .. } => "安全验证失败，请重新激活",
            Self::DeviceMismatch { .. } => "如果更换了设备，请联系管理员重新激活",
            Self::ClockTampering { .. } => "请检查系统时间设置是否正确",
            Self::BindingInvalid { .. } => "设备绑定无效，请重新激活",
            Self::TokenExpired { .. } => "凭据已过期，请重新激活",
            Self::NetworkError {
                can_continue_offline: true,
                ..
            } => "可以离线继续使用，联网后将自动同步",
            Self::NetworkError {
                can_continue_offline: false,
                ..
            } => "请检查网络连接后重试",
            Self::Revoked { .. } => "请联系管理员了解详情",
        }
    }

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
// 激活进度
// =============================================================================

/// 激活步骤
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivationStep {
    Authenticating,
    DownloadingCertificates,
    VerifyingBinding,
    CheckingSubscription,
    StartingServer,
    Complete,
}

impl ActivationStep {
    pub fn message_zh(&self) -> &'static str {
        match self {
            Self::Authenticating => "正在验证凭据...",
            Self::DownloadingCertificates => "正在下载证书...",
            Self::VerifyingBinding => "正在验证设备绑定...",
            Self::CheckingSubscription => "正在检查订阅状态...",
            Self::StartingServer => "正在启动服务...",
            Self::Complete => "激活完成",
        }
    }

    pub fn step_number(&self) -> u8 {
        match self {
            Self::Authenticating => 1,
            Self::DownloadingCertificates => 2,
            Self::VerifyingBinding => 3,
            Self::CheckingSubscription => 4,
            Self::StartingServer => 5,
            Self::Complete => 6,
        }
    }

    pub const TOTAL_STEPS: u8 = 6;
}

/// 激活进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationProgress {
    pub step: ActivationStep,
    pub total_steps: u8,
    pub current_step: u8,
    pub message: String,
    pub started_at: i64,
}

impl ActivationProgress {
    pub fn new(step: ActivationStep) -> Self {
        Self {
            step,
            total_steps: ActivationStep::TOTAL_STEPS,
            current_step: step.step_number(),
            message: step.message_zh().to_string(),
            started_at: crate::util::now_millis(),
        }
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_valid_until: Option<i64>,
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
