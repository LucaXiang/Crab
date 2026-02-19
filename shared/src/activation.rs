//! 统一的激活响应结构
//!
//! Auth Server 一次性返回所有激活所需数据，包括:
//! - 证书链 (Root CA, Tenant CA, Entity Cert, Private Key)
//! - 签名的绑定数据 (防篡改)
//! - 订阅信息
//!
//! 使用者:
//! - crab-auth: 生成并签名激活响应
//! - edge-server: 验证并保存服务器绑定
//! - crab-client: 验证并保存客户端凭证

use crate::error::ErrorCode;
use serde::{Deserialize, Serialize};

/// 统一的激活响应
///
/// Auth Server 返回此结构，包含完整的激活数据。
/// 所有敏感数据都由 Tenant CA 签名保护。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationResponse {
    /// 是否成功
    pub success: bool,
    /// 错误信息 (失败时，开发调试用)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// 结构化错误码 (失败时，客户端用于 i18n 查表)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error_code: Option<ErrorCode>,
    /// 激活数据 (成功时)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ActivationData>,
    /// Quota 信息 (设备数已满时返回)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub quota_info: Option<QuotaInfo>,
}

/// 设备 Quota 信息
///
/// 当激活请求因设备数已满被拒绝时返回，
/// 包含当前已激活设备列表，供前端展示替换选项。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaInfo {
    /// 计划允许的最大设备数量 (server 或 client 取决于上下文)
    pub max_slots: u32,
    /// 当前活跃设备数
    pub active_count: u32,
    /// 当前已激活设备列表
    pub active_devices: Vec<ActiveDevice>,
}

/// 已激活设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveDevice {
    pub entity_id: String,
    pub device_id: String,
    pub activated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_refreshed_at: Option<i64>,
}

/// 完整的激活数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationData {
    // === 身份信息 ===
    /// 实体 ID (Server ID 或 Client Name)
    pub entity_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 设备 ID (硬件绑定)
    pub device_id: String,

    // === 证书链 ===
    /// Root CA 证书 (PEM)
    pub root_ca_cert: String,
    /// Tenant CA 证书 (PEM)
    pub tenant_ca_cert: String,
    /// 实体证书 (PEM)
    pub entity_cert: String,
    /// 实体私钥 (PEM)
    pub entity_key: String,

    // === 签名的绑定数据 ===
    /// 绑定数据 (JSON 字符串，已签名)
    pub binding: SignedBinding,

    // === 订阅信息 ===
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription: Option<SubscriptionInfo>,
}

/// 签名的绑定数据
///
/// 关键数据由 Tenant CA 私钥签名，防止篡改。
/// 包含 last_verified_at 用于时钟篡改检测。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedBinding {
    /// 实体 ID
    pub entity_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 设备 ID
    pub device_id: String,
    /// 证书指纹 (SHA256)
    pub fingerprint: String,
    /// 绑定时间 (Unix millis)
    pub bound_at: i64,
    /// 实体类型
    pub entity_type: EntityType,
    /// 最后验证时间 (Unix millis) - 用于时钟篡改检测
    #[serde(default)]
    pub last_verified_at: i64,
    /// Tenant CA 签名 (base64)
    /// 签名内容: "{entity_id}|{tenant_id}|{device_id}|{fingerprint}|{bound_at}|{entity_type}|{last_verified_at}"
    pub signature: String,
}

impl SignedBinding {
    /// 最大允许的时钟回拨时间 (1 小时)
    pub const MAX_CLOCK_BACKWARD_SECS: i64 = 3600;
    /// 最大允许的时钟前进时间 (30 天)
    pub const MAX_CLOCK_FORWARD_SECS: i64 = 30 * 24 * 3600;

    /// 创建新的绑定数据 (未签名)
    pub fn new(
        entity_id: impl Into<String>,
        tenant_id: impl Into<String>,
        device_id: impl Into<String>,
        fingerprint: impl Into<String>,
        entity_type: EntityType,
    ) -> Self {
        let now = crate::util::now_millis();
        Self {
            entity_id: entity_id.into(),
            tenant_id: tenant_id.into(),
            device_id: device_id.into(),
            fingerprint: fingerprint.into(),
            bound_at: now,
            entity_type,
            last_verified_at: now,
            signature: String::new(),
        }
    }

    /// 返回待签名的数据
    pub fn signable_data(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}",
            self.entity_id,
            self.tenant_id,
            self.device_id,
            self.fingerprint,
            self.bound_at,
            self.entity_type.as_str(),
            self.last_verified_at
        )
    }

    /// 更新 last_verified_at (需要重新签名)
    pub fn refresh(mut self) -> Self {
        self.last_verified_at = crate::util::now_millis();
        self.signature = String::new(); // 清除旧签名，需要重新签名
        self
    }

    /// 检测时钟篡改
    pub fn check_clock_tampering(&self) -> Result<(), String> {
        if self.last_verified_at == 0 {
            return Ok(()); // 未设置时跳过检查
        }

        let now_ms = crate::util::now_millis();
        let diff_secs = (now_ms - self.last_verified_at) / 1000;

        // 时钟回拨检测
        if diff_secs < -Self::MAX_CLOCK_BACKWARD_SECS {
            return Err(format!(
                "Clock tampering detected: time moved backward by {} seconds",
                -diff_secs
            ));
        }

        // 时钟大幅前进检测
        if diff_secs > Self::MAX_CLOCK_FORWARD_SECS {
            return Err(format!(
                "Clock tampering detected: time jumped forward by {} days",
                diff_secs / 86400
            ));
        }

        Ok(())
    }

    /// 使用 Tenant CA 私钥签名
    pub fn sign(mut self, tenant_ca_key_pem: &str) -> Result<Self, String> {
        let data = self.signable_data();
        let sig_bytes = crab_cert::sign(tenant_ca_key_pem, data.as_bytes())
            .map_err(|e| format!("Failed to sign binding: {}", e))?;
        self.signature = base64_encode(&sig_bytes);
        Ok(self)
    }

    /// 验证签名
    pub fn verify_signature(&self, tenant_ca_cert_pem: &str) -> Result<(), String> {
        if self.signature.is_empty() {
            return Err("Binding is not signed".into());
        }

        let sig_bytes = base64_decode(&self.signature)
            .map_err(|e| format!("Invalid signature encoding: {}", e))?;

        let data = self.signable_data();
        crab_cert::verify(tenant_ca_cert_pem, data.as_bytes(), &sig_bytes)
            .map_err(|e| format!("Signature verification failed: {}", e))
    }

    /// 验证硬件绑定
    pub fn verify_device(&self) -> Result<(), String> {
        let current_device_id = crab_cert::generate_hardware_id();
        if self.device_id != current_device_id {
            return Err(format!(
                "Device ID mismatch: expected {}, got {}",
                self.device_id, current_device_id
            ));
        }
        Ok(())
    }

    /// 完整验证 (签名 + 硬件 + 时钟)
    pub fn validate(&self, tenant_ca_cert_pem: &str) -> Result<(), String> {
        self.verify_signature(tenant_ca_cert_pem)?;
        self.verify_device()?;
        self.check_clock_tampering()?;
        Ok(())
    }
}

/// 实体类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    /// Edge Server
    Server,
    /// Client (POS, KDS, etc.)
    Client,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityType::Server => "server",
            EntityType::Client => "client",
        }
    }
}

/// P12 电子签名证书状态
///
/// 由 crab-auth 在激活/订阅刷新时查询 p12_certificates 表后返回。
/// edge-server 不直接访问 P12 文件，仅通过此元数据判断状态。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct P12Info {
    /// 是否已上传 P12 证书
    pub has_p12: bool,
    /// 证书指纹 (SHA256)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    /// 证书主体 (CN)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// 证书过期时间 (Unix millis)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
}

/// 订阅信息
///
/// 订阅信息有独立签名，有效期较短 (默认 7 天)。
/// 签名过期后需要从 Auth Server 刷新。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubscriptionInfo {
    /// 租户 ID
    pub tenant_id: String,
    /// 订阅 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// 订阅状态
    pub status: SubscriptionStatus,
    /// 计划类型
    pub plan: PlanType,
    /// 开始时间 (Unix millis)
    pub starts_at: i64,
    /// 过期时间 (Unix millis)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    /// 启用的功能
    #[serde(default)]
    pub features: Vec<String>,
    /// Plan 允许的最大门店数，0 = 无限
    #[serde(default)]
    pub max_stores: u32,
    /// 每个 edge-server 允许的最大 Client 数，0 = 无限
    #[serde(default)]
    pub max_clients: u32,
    /// P12 电子签名证书状态 (由 crab-auth 返回)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub p12: Option<P12Info>,
    /// 签名有效期 (Unix millis，超过此时间需要刷新)
    pub signature_valid_until: i64,
    /// Tenant CA 签名 (base64)
    /// 签名内容: "{tenant_id}|{plan}|{status}|{features}|{max_stores}|{signature_valid_until}"
    pub signature: String,
    /// 最后检查时间 (Unix millis，本地记录)
    #[serde(default)]
    pub last_checked_at: i64,
}

impl SubscriptionInfo {
    /// 签名过期宽限期 (3 天)
    ///
    /// 签名有效期 7 天 + 宽限期 3 天 = 最多 10 天离线容忍。
    /// 超过此限制必须联网刷新，否则阻止使用。
    pub const SIGNATURE_GRACE_PERIOD_MS: i64 = 3 * 24 * 60 * 60 * 1000;

    /// 检查签名是否陈旧 (过期 + 宽限期也已过)
    pub fn is_signature_stale(&self) -> bool {
        crate::util::now_millis() > self.signature_valid_until + Self::SIGNATURE_GRACE_PERIOD_MS
    }

    /// 检查是否已签名
    pub fn is_signed(&self) -> bool {
        !self.signature.is_empty()
    }

    /// 返回待签名的数据
    pub fn signable_data(&self) -> String {
        let features_str = self.features.join(",");
        format!(
            "{}|{}|{}|{}|{}|{}|{}",
            self.tenant_id,
            self.plan.as_str(),
            self.status.as_str(),
            features_str,
            self.max_stores,
            self.max_clients,
            self.signature_valid_until
        )
    }

    /// 使用 Tenant CA 私钥签名
    pub fn sign(mut self, tenant_ca_key_pem: &str) -> Result<Self, String> {
        let data = self.signable_data();
        let sig_bytes = crab_cert::sign(tenant_ca_key_pem, data.as_bytes())
            .map_err(|e| format!("Failed to sign subscription: {}", e))?;
        self.signature = base64_encode(&sig_bytes);
        Ok(self)
    }

    /// 验证签名
    pub fn verify_signature(&self, tenant_ca_cert_pem: &str) -> Result<(), String> {
        if self.signature.is_empty() {
            return Err("Subscription is not signed".into());
        }

        let sig_bytes = base64_decode(&self.signature)
            .map_err(|e| format!("Invalid subscription signature encoding: {}", e))?;

        let data = self.signable_data();
        crab_cert::verify(tenant_ca_cert_pem, data.as_bytes(), &sig_bytes)
            .map_err(|e| format!("Subscription signature verification failed: {}", e))
    }

    /// 检查签名是否过期
    pub fn is_signature_expired(&self) -> bool {
        crate::util::now_millis() > self.signature_valid_until
    }

    /// 完整验证 (签名 + 有效期)
    pub fn validate(&self, tenant_ca_cert_pem: &str) -> Result<(), String> {
        self.verify_signature(tenant_ca_cert_pem)?;
        if self.is_signature_expired() {
            return Err("Subscription signature has expired, needs refresh".into());
        }
        Ok(())
    }
}

impl SubscriptionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubscriptionStatus::Inactive => "inactive",
            SubscriptionStatus::Active => "active",
            SubscriptionStatus::PastDue => "past_due",
            SubscriptionStatus::Expired => "expired",
            SubscriptionStatus::Canceled => "canceled",
            SubscriptionStatus::Unpaid => "unpaid",
        }
    }

    /// 是否处于阻止激活的状态
    pub fn is_blocked(&self) -> bool {
        matches!(
            self,
            SubscriptionStatus::Inactive
                | SubscriptionStatus::Expired
                | SubscriptionStatus::Canceled
                | SubscriptionStatus::Unpaid
        )
    }
}

impl PlanType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlanType::Basic => "basic",
            PlanType::Pro => "pro",
            PlanType::Enterprise => "enterprise",
        }
    }

    /// 返回该计划允许的最大门店数量
    /// 0 表示无限制
    pub fn max_stores(&self) -> usize {
        match self {
            PlanType::Basic => 1,
            PlanType::Pro => 3,
            PlanType::Enterprise => 0, // 无限
        }
    }
}

/// 订阅状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    /// 注册未付费/合同未签
    Inactive,
    /// 合同有效，付费正常
    Active,
    /// 扣费失败，Stripe 重试中
    PastDue,
    /// 合同到期未续约
    Expired,
    /// 主动终止/重试全败
    Canceled,
    /// 长期欠费
    Unpaid,
}

/// 计划类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanType {
    Basic,
    Pro,
    Enterprise,
}

// === Base64 helpers ===

fn base64_encode(data: &[u8]) -> String {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    STANDARD.encode(data)
}

fn base64_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    STANDARD.decode(s)
}

// === Tenant Verify Types (身份验证，不签发证书) ===

/// 租户验证响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantVerifyResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error_code: Option<ErrorCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<TenantVerifyData>,
}

/// 租户验证数据 (仅身份验证，不签发证书)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantVerifyData {
    pub tenant_id: String,
    /// JWT access token (短期，1 小时)
    pub token: String,
    /// Refresh token (长期，30 天，活跃时自动续期)
    pub refresh_token: String,
    pub subscription_status: SubscriptionStatus,
    pub plan: PlanType,
    /// 剩余可用 Server 配额
    pub server_slots_remaining: i32,
    /// 剩余可用 Client 配额
    pub client_slots_remaining: i32,
    /// 当前设备是否已有 Server 激活
    pub has_active_server: bool,
    /// 当前设备是否已有 Client 激活
    pub has_active_client: bool,
}

/// Token 刷新请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshRequest {
    pub refresh_token: String,
}

/// Token 刷新响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error_code: Option<ErrorCode>,
    /// 新的 access token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// 新的 refresh token (轮转)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

// === Deactivate Types (注销证书，释放配额) ===

/// 注销响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeactivateResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error_code: Option<ErrorCode>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_type_serialization() {
        assert_eq!(EntityType::Server.as_str(), "server");
        assert_eq!(EntityType::Client.as_str(), "client");
    }

    #[test]
    fn test_signable_data() {
        let binding = SignedBinding::new(
            "server-001",
            "tenant-123",
            "hw-abc",
            "fingerprint-xyz",
            EntityType::Server,
        );
        let data = binding.signable_data();
        assert!(data.contains("server-001"));
        assert!(data.contains("tenant-123"));
        assert!(data.contains("server"));
    }
}
