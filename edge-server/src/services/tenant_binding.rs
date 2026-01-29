//! 租户绑定的凭证存储
//!
//! 将租户绑定信息存储到 workspace/cert/Credential.json
//! 而不是数据库中。
//!
//! 使用 shared::activation::SignedBinding 来存储绑定数据，
//! 包含 last_verified_at 用于时钟篡改检测。

use crate::utils::AppError;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use crab_cert::{CertMetadata, generate_hardware_id};
use serde::{Deserialize, Serialize};
use shared::activation::SignedBinding;
use std::path::Path;

fn base64_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    STANDARD.decode(s)
}

/// 验证证书对（证书 + CA）以及硬件绑定
pub fn verify_cert_pair(cert_pem: &str, ca_pem: &str) -> Result<(), AppError> {
    // 1. 验证证书链
    crab_cert::verify_chain_against_root(cert_pem, ca_pem)
        .map_err(|e| AppError::validation(format!("Certificate chain validation failed: {}", e)))?;

    // 2. 解析元数据
    let metadata = CertMetadata::from_pem(cert_pem).map_err(|e| {
        AppError::validation(format!("Failed to parse certificate metadata: {}", e))
    })?;

    // 3. 验证硬件 ID
    let current_hardware_id = generate_hardware_id();

    if let Some(cert_device_id) = metadata.device_id {
        if cert_device_id != current_hardware_id {
            return Err(AppError::validation(format!(
                "Hardware ID mismatch! Certificate bound to {}, but current machine is {}",
                cert_device_id, current_hardware_id
            )));
        }
    } else {
        // 如果证书没有 device_id，根据策略可能允许也可能拒绝。
        // 对于 Edge Server，应该强制要求绑定。
        return Err(AppError::validation(
            "Certificate missing device_id extension",
        ));
    }

    Ok(())
}

/// 凭证存储位置
pub const CREDENTIAL_FILE: &str = "credential.json";

/// 订阅状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

impl SubscriptionStatus {
    /// 订阅是否被阻止（不允许使用系统）
    ///
    /// PastDue 仍允许使用（Stripe 正在重试扣费）
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

/// 订阅计划类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanType {
    Free,
    Pro,
    Enterprise,
}

fn default_now_millis() -> i64 {
    shared::util::now_millis()
}

/// 订阅详情
///
/// 订阅信息有独立签名，防止本地篡改。
/// 签名有效期较短 (默认 7 天)，需要定期从 Auth Server 刷新。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Subscription {
    pub id: Option<String>,
    pub tenant_id: String,
    pub status: SubscriptionStatus,
    pub plan: PlanType,
    #[serde(default = "default_now_millis")]
    pub starts_at: i64,
    pub expires_at: Option<i64>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default = "default_now_millis")]
    pub last_checked_at: i64,
    /// 签名有效期 (Unix millis，超过此时间需要从 Auth Server 刷新)
    #[serde(default)]
    pub signature_valid_until: Option<i64>,
    /// Tenant CA 签名 (base64)
    /// 签名内容: "{tenant_id}|{plan}|{status}|{features_joined}|{signature_valid_until}"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl Subscription {
    /// 返回待签名的数据
    pub fn signable_data(&self) -> String {
        let features_str = self.features.join(",");
        let valid_until = self
            .signature_valid_until
            .map(|t| t.to_string())
            .unwrap_or_default();
        format!(
            "{}|{}|{:?}|{}|{}",
            self.tenant_id,
            self.plan_str(),
            self.status,
            features_str,
            valid_until
        )
    }

    fn plan_str(&self) -> &'static str {
        match self.plan {
            PlanType::Free => "free",
            PlanType::Pro => "pro",
            PlanType::Enterprise => "enterprise",
        }
    }

    /// 验证订阅签名
    pub fn verify_signature(&self, tenant_ca_cert_pem: &str) -> Result<(), AppError> {
        let sig_b64 = self
            .signature
            .as_ref()
            .ok_or_else(|| AppError::validation("Subscription is not signed"))?;

        let sig_bytes = base64_decode(sig_b64).map_err(|e| {
            AppError::validation(format!("Invalid subscription signature encoding: {}", e))
        })?;

        let data = self.signable_data();
        crab_cert::verify(tenant_ca_cert_pem, data.as_bytes(), &sig_bytes).map_err(|e| {
            AppError::validation(format!("Subscription signature verification failed: {}", e))
        })
    }

    /// 检查签名是否过期 (需要刷新)
    pub fn is_signature_expired(&self) -> bool {
        match self.signature_valid_until {
            Some(valid_until) => shared::util::now_millis() > valid_until,
            None => true, // 没有有效期 = 已过期
        }
    }

    /// 完整验证 (签名 + 有效期)
    pub fn validate(&self, tenant_ca_cert_pem: &str) -> Result<(), AppError> {
        // 1. 验证签名
        self.verify_signature(tenant_ca_cert_pem)?;

        // 2. 检查签名有效期
        if self.is_signature_expired() {
            return Err(AppError::validation(
                "Subscription signature has expired, needs refresh",
            ));
        }

        Ok(())
    }

    /// 检查是否已签名
    pub fn is_signed(&self) -> bool {
        self.signature.is_some()
    }
}

impl Default for Subscription {
    fn default() -> Self {
        let now = shared::util::now_millis();
        Self {
            id: None,
            tenant_id: "default".to_string(),
            status: SubscriptionStatus::Inactive,
            plan: PlanType::Free,
            starts_at: now,
            expires_at: None,
            features: vec![],
            last_checked_at: now,
            signature_valid_until: None,
            signature: None,
        }
    }
}

/// 已绑定租户的存储凭证
///
/// 核心绑定数据使用 SignedBinding (来自 shared)，
/// 由 Tenant CA 签名保护，包含 last_verified_at 用于时钟篡改检测。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TenantBinding {
    /// 签名的绑定数据 (来自 Auth Server)
    pub binding: SignedBinding,
    /// 订阅信息 (独立签名)
    #[serde(default)]
    pub subscription: Option<Subscription>,
}

impl TenantBinding {
    /// 从 SignedBinding 创建 (推荐)
    pub fn from_signed(binding: SignedBinding) -> Self {
        Self {
            binding,
            subscription: None,
        }
    }

    /// 检测时钟篡改 (委托给 SignedBinding)
    pub fn check_clock_tampering(&self) -> Result<(), AppError> {
        self.binding
            .check_clock_tampering()
            .map_err(AppError::validation)
    }

    /// 验证签名 (委托给 SignedBinding)
    pub fn verify_signature(&self, tenant_ca_cert_pem: &str) -> Result<(), AppError> {
        self.binding
            .verify_signature(tenant_ca_cert_pem)
            .map_err(AppError::validation)
    }

    /// 验证硬件绑定 (委托给 SignedBinding)
    pub fn verify_device(&self) -> Result<(), AppError> {
        self.binding.verify_device().map_err(AppError::validation)
    }

    /// 完整验证 (签名 + 硬件 + 时钟)
    pub fn validate(&self, tenant_ca_cert_pem: &str) -> Result<(), AppError> {
        self.binding
            .validate(tenant_ca_cert_pem)
            .map_err(AppError::validation)
    }

    /// 检查是否已签名
    pub fn is_signed(&self) -> bool {
        !self.binding.signature.is_empty()
    }

    /// 更新 binding (从 Auth Server 获取刷新后的 binding)
    pub fn update_binding(&mut self, new_binding: SignedBinding) {
        self.binding = new_binding;
    }

    /// 获取内部 binding 引用 (用于发送刷新请求)
    pub fn get_binding(&self) -> &SignedBinding {
        &self.binding
    }

    /// 从文件加载凭证
    pub fn load(cert_dir: &Path) -> Result<Option<Self>, std::io::Error> {
        let path = cert_dir.join(CREDENTIAL_FILE);
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            .map(Some)
    }

    /// 保存凭证到文件
    pub fn save(&self, cert_dir: &Path) -> Result<(), std::io::Error> {
        let path = cert_dir.join(CREDENTIAL_FILE);
        let content = serde_json::to_string(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 删除凭证文件
    pub fn delete(cert_dir: &Path) -> Result<(), std::io::Error> {
        let path = cert_dir.join(CREDENTIAL_FILE);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// 检查是否已绑定
    pub fn is_bound(cert_dir: &Path) -> bool {
        let path = cert_dir.join(CREDENTIAL_FILE);
        path.exists()
    }
}
