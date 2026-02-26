//! 租户绑定的凭证存储
//!
//! 将租户绑定信息存储到 workspace/cert/Credential.json
//! 而不是数据库中。
//!
//! 使用 shared::activation::SignedBinding 来存储绑定数据，
//! 包含 last_verified_at 用于时钟篡改检测。

use crate::utils::AppError;
use crab_cert::{CertMetadata, generate_hardware_id};
use serde::{Deserialize, Serialize};
use shared::activation::{SignedBinding, SubscriptionInfo};
use std::path::Path;

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

/// 已绑定租户的存储凭证
///
/// 核心绑定数据使用 SignedBinding (来自 shared)，
/// 由 Tenant CA 签名保护，包含 last_verified_at 用于时钟篡改检测。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantBinding {
    /// 签名的绑定数据 (来自 Auth Server)
    pub binding: SignedBinding,
    /// 订阅信息 (独立签名，直接使用 shared 统一类型)
    #[serde(default)]
    pub subscription: Option<SubscriptionInfo>,
    /// 门店编号 (per-tenant 递增，Cloud 激活时分配)
    #[serde(default = "default_store_number")]
    pub store_number: u32,
}

fn default_store_number() -> u32 {
    1
}

impl TenantBinding {
    /// 从 SignedBinding 创建（无订阅数据）
    pub fn from_signed(binding: SignedBinding, store_number: u32) -> Self {
        Self {
            binding,
            subscription: None,
            store_number,
        }
    }

    /// 从激活响应创建（包含订阅数据）
    pub fn from_activation(
        binding: SignedBinding,
        subscription: Option<SubscriptionInfo>,
        store_number: u32,
    ) -> Self {
        Self {
            binding,
            subscription,
            store_number,
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
