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

use serde::{Deserialize, Serialize};

/// 统一的激活响应
///
/// Auth Server 返回此结构，包含完整的激活数据。
/// 所有敏感数据都由 Tenant CA 签名保护。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationResponse {
    /// 是否成功
    pub success: bool,
    /// 错误信息 (失败时)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// 激活数据 (成功时)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ActivationData>,
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignedBinding {
    /// 实体 ID
    pub entity_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 设备 ID
    pub device_id: String,
    /// 证书指纹 (SHA256)
    pub fingerprint: String,
    /// 绑定时间 (RFC3339)
    pub bound_at: String,
    /// 实体类型
    pub entity_type: EntityType,
    /// 最后验证时间 (RFC3339) - 用于时钟篡改检测
    #[serde(default)]
    pub last_verified_at: String,
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
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            entity_id: entity_id.into(),
            tenant_id: tenant_id.into(),
            device_id: device_id.into(),
            fingerprint: fingerprint.into(),
            bound_at: now.clone(),
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
        self.last_verified_at = chrono::Utc::now().to_rfc3339();
        self.signature = String::new(); // 清除旧签名，需要重新签名
        self
    }

    /// 检测时钟篡改
    pub fn check_clock_tampering(&self) -> Result<(), String> {
        if self.last_verified_at.is_empty() {
            return Ok(()); // 未设置时跳过检查
        }

        let last_verified = chrono::DateTime::parse_from_rfc3339(&self.last_verified_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .map_err(|e| format!("Failed to parse last_verified_at: {}", e))?;

        let now = chrono::Utc::now();
        let diff_secs = (now - last_verified).num_seconds();

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

/// 订阅信息
///
/// 订阅信息有独立签名，有效期较短 (默认 7 天)。
/// 签名过期后需要从 Auth Server 刷新。
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// 开始时间 (RFC3339)
    pub starts_at: String,
    /// 过期时间 (RFC3339)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// 启用的功能
    #[serde(default)]
    pub features: Vec<String>,
    /// 签名有效期 (RFC3339，超过此时间需要刷新)
    pub signature_valid_until: String,
    /// Tenant CA 签名 (base64)
    /// 签名内容: "{tenant_id}|{plan}|{status}|{features}|{signature_valid_until}"
    pub signature: String,
}

impl SubscriptionInfo {
    /// 返回待签名的数据
    pub fn signable_data(&self) -> String {
        let features_str = self.features.join(",");
        format!(
            "{}|{}|{}|{}|{}",
            self.tenant_id,
            self.plan.as_str(),
            self.status.as_str(),
            features_str,
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
        use chrono::{DateTime, Utc};
        match DateTime::parse_from_rfc3339(&self.signature_valid_until) {
            Ok(valid_until) => Utc::now() > valid_until,
            Err(_) => true, // 解析失败视为过期
        }
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
            SubscriptionStatus::Active => "active",
            SubscriptionStatus::Trial => "trial",
            SubscriptionStatus::PastDue => "past_due",
            SubscriptionStatus::Canceled => "canceled",
            SubscriptionStatus::Unpaid => "unpaid",
        }
    }
}

impl PlanType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlanType::Free => "free",
            PlanType::Pro => "pro",
            PlanType::Enterprise => "enterprise",
        }
    }
}

/// 订阅状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    Trial,
    PastDue,
    Canceled,
    Unpaid,
}

/// 计划类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanType {
    Free,
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
