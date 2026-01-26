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
        expired_at: String,
        days_overdue: i64,
    },

    /// 证书即将过期 (警告)
    CertificateExpiringSoon {
        expires_at: String,
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
        last_verified_at: String,
    },

    /// Binding 无效
    BindingInvalid { error: String },

    /// Token 过期
    TokenExpired { expired_at: String },

    /// 网络错误
    NetworkError {
        error: String,
        can_continue_offline: bool,
    },

    /// 已被吊销
    Revoked { revoked_at: String, reason: String },
}

impl ActivationRequiredReason {
    /// 获取恢复建议的 code (前端负责 i18n)
    pub fn recovery_hint_code(&self) -> &'static str {
        match self {
            Self::FirstTimeSetup => "hint.first_time_setup",
            Self::CertificateExpired { .. } => "hint.certificate_expired",
            Self::CertificateExpiringSoon { .. } => "hint.certificate_expiring_soon",
            Self::CertificateInvalid { .. } => "hint.certificate_invalid",
            Self::SignatureInvalid { .. } => "hint.signature_invalid",
            Self::DeviceMismatch { .. } => "hint.device_mismatch",
            Self::ClockTampering { .. } => "hint.clock_tampering",
            Self::BindingInvalid { .. } => "hint.binding_invalid",
            Self::TokenExpired { .. } => "hint.token_expired",
            Self::NetworkError { can_continue_offline: true, .. } => "hint.network_error_offline_ok",
            Self::NetworkError { can_continue_offline: false, .. } => "hint.network_error",
            Self::Revoked { .. } => "hint.revoked",
        }
    }

    /// 是否可以自动恢复
    pub fn can_auto_recover(&self) -> bool {
        matches!(
            self,
            Self::CertificateExpiringSoon { .. }
                | Self::NetworkError { can_continue_offline: true, .. }
        )
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expired_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period_days: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period_ends_at: Option<String>,
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
    pub started_at: String,
}

impl ActivationProgress {
    pub fn new(step: ActivationStep) -> Self {
        Self {
            step,
            total_steps: ActivationStep::TOTAL_STEPS,
            current_step: step.step_number(),
            message: step.message_zh().to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
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
    pub expires_at: Option<String>,
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
    pub signature_valid_until: Option<String>,
    pub needs_refresh: bool,
}

/// 网络健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkHealth {
    pub status: HealthLevel,
    pub auth_server_reachable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_connected_at: Option<String>,
}

/// 数据库健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub status: HealthLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_write_at: Option<String>,
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
    pub checked_at: String,
    pub device_info: DeviceInfo,
}
