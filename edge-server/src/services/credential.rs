//! 租户绑定的凭证存储
//!
//! 将租户绑定信息存储到 workspace/cert/Credential.json
//! 而不是数据库中。

use crate::utils::AppError;
use chrono::{DateTime, Utc};
use crab_cert::{CertMetadata, generate_hardware_id};
use serde::{Deserialize, Serialize};
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
pub const CREDENTIAL_FILE: &str = "Credential.json";

/// 订阅状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    PastDue,
    Canceled,
    Unpaid,
    Trial,
}

/// 订阅计划类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanType {
    Free,
    Pro,
    Enterprise,
}

fn default_now() -> DateTime<Utc> {
    Utc::now()
}

/// 订阅详情
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Subscription {
    pub id: Option<String>,
    pub tenant_id: String,
    pub status: SubscriptionStatus,
    pub plan: PlanType,
    #[serde(default = "default_now")]
    pub starts_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default = "default_now")]
    pub last_checked_at: DateTime<Utc>,
}

impl Default for Subscription {
    fn default() -> Self {
        Self {
            id: None,
            tenant_id: "default".to_string(),
            status: SubscriptionStatus::Active,
            plan: PlanType::Free,
            starts_at: Utc::now(),
            expires_at: None,
            features: vec![],
            last_checked_at: Utc::now(),
        }
    }
}

/// 已绑定租户的存储凭证
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Credential {
    /// 租户 ID
    pub tenant_id: String,
    /// 服务器 ID
    pub server_id: String,
    /// 设备 ID (来自证书的硬件 ID)
    #[serde(default)]
    pub device_id: Option<String>,
    /// 证书指纹 (服务器证书的 SHA256)
    pub fingerprint: String,
    /// 绑定时间戳 (RFC3339)
    pub bound_at: String,
    /// 订阅信息
    #[serde(default)]
    pub subscription: Option<Subscription>,
}

impl Credential {
    /// 创建新凭证
    pub fn new(
        tenant_id: String,
        server_id: String,
        device_id: Option<String>,
        fingerprint: String,
    ) -> Self {
        Self {
            tenant_id,
            server_id,
            device_id,
            fingerprint,
            bound_at: chrono::Utc::now().to_rfc3339(),
            subscription: None,
        }
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
        // Use compact format for smaller file size and faster parsing
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
