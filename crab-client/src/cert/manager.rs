// crab-client/src/cert/manager.rs
// 证书管理器 - 处理凭证申请、验证和存储

use crate::cert::{Credential, CredentialStorage};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CertError {
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Certificate expired")]
    Expired,

    #[error("Certificate not found")]
    NotFound,

    #[error("Invalid certificate: {0}")]
    Invalid(String),
}

/// 证书管理器
#[derive(Debug)]
pub struct CertManager {
    credential_storage: CredentialStorage,
    cert_path: PathBuf,
    client_name: String,
}

impl CertManager {
    /// 创建证书管理器
    pub fn new(base_path: impl Into<PathBuf>, client_name: &str) -> Self {
        let cert_path = base_path.into().join(client_name);
        let credential_storage = CredentialStorage::new(&cert_path, client_name);
        Self {
            credential_storage,
            cert_path,
            client_name: client_name.to_string(),
        }
    }

    /// 加载或请求凭证
    pub async fn load_or_login(
        &self,
        auth_url: &str,
        username: &str,
        password: &str,
    ) -> Result<Credential, CertError> {
        // 检查本地凭证
        if let Some(cred) = self.credential_storage.load() {
            tracing::info!("Using cached credential for {}", cred.client_name);
            return Ok(cred);
        }

        // 请求新凭证
        tracing::info!("Requesting credential from {}", auth_url);
        self.login(auth_url, username, password).await
    }

    /// 登录获取凭证
    pub async fn login(
        &self,
        auth_url: &str,
        username: &str,
        password: &str,
    ) -> Result<Credential, CertError> {
        let client = reqwest::Client::new();

        let response = client
            .post(format!("{}/api/auth/login", auth_url))
            .json(&serde_json::json!({
                "username": username,
                "password": password
            }))
            .send()
            .await
            .map_err(|e| CertError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CertError::Network(
                response.text().await.unwrap_or_else(|_| "Unknown error".to_string())
            ));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| CertError::Network(e.to_string()))?;

        let token = data.get("token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CertError::Invalid("No token in response".to_string()))?
            .to_string();

        let tenant_id = data.get("tenant_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let credential = Credential::new(
            self.client_name.clone(),
            token,
            None,
            tenant_id,
        );

        // 保存凭证
        self.credential_storage.save(&credential)
            .map_err(|e| CertError::Storage(e.to_string()))?;

        tracing::info!("Credential saved to {:?}", self.credential_storage.path());

        Ok(credential)
    }

    /// 获取凭证路径
    pub fn credential_path(&self) -> &Path {
        self.credential_storage.path()
    }

    /// 检查凭证是否存在
    pub fn has_credential(&self) -> bool {
        self.credential_storage.exists()
    }

    /// 删除凭证 (登出)
    pub fn logout(&self) -> std::io::Result<()> {
        self.credential_storage.delete()
    }
}
