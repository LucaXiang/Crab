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
#[derive(Debug, Clone)]
pub struct CertManager {
    credential_storage: CredentialStorage,
    client_name: String,
}

impl CertManager {
    /// 创建证书管理器
    pub fn new(base_path: impl Into<PathBuf>, client_name: &str) -> Self {
        let cert_path = base_path.into().join(client_name);
        let credential_storage = CredentialStorage::new(&cert_path, "credential.json");
        Self {
            credential_storage,
            client_name: client_name.to_string(),
        }
    }

    /// 获取客户端名称
    pub fn client_name(&self) -> &str {
        &self.client_name
    }

    /// 加载缓存的凭证（不登录）
    pub fn load_credential(&self) -> Result<Credential, CertError> {
        self.credential_storage.load()
            .ok_or_else(|| CertError::NotFound)
    }

    /// 保存凭证
    pub fn save_credential(&self, credential: &Credential) -> Result<(), CertError> {
        self.credential_storage.save(credential)
            .map_err(|e| CertError::Storage(e.to_string()))
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

    /// 获取证书存储路径
    pub fn cert_path(&self) -> PathBuf {
        self.credential_storage.path().parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.credential_storage.path().to_path_buf())
    }

    /// 保存证书到文件
    pub fn save_certificates(
        &self,
        cert_pem: &str,
        key_pem: &str,
        ca_cert_pem: &str,
    ) -> Result<(), CertError> {
        let cert_dir = self.cert_path();

        // 确保目录存在
        std::fs::create_dir_all(&cert_dir)
            .map_err(|e| CertError::Storage(e.to_string()))?;

        // 保存客户端证书
        let cert_path = cert_dir.join("entity.crt");
        std::fs::write(&cert_path, cert_pem)
            .map_err(|e| CertError::Storage(e.to_string()))?;

        // 保存客户端私钥
        let key_path = cert_dir.join("entity.key");
        std::fs::write(&key_path, key_pem)
            .map_err(|e| CertError::Storage(e.to_string()))?;

        // 保存 CA 证书
        let ca_path = cert_dir.join("tenant_ca.crt");
        std::fs::write(&ca_path, ca_cert_pem)
            .map_err(|e| CertError::Storage(e.to_string()))?;

        tracing::info!("Certificates saved to {:?}", cert_dir);
        Ok(())
    }

    /// 检查本地证书是否存在
    pub fn has_local_certificates(&self) -> bool {
        let cert_dir = self.cert_path();
        cert_dir.join("entity.crt").exists()
            && cert_dir.join("entity.key").exists()
            && cert_dir.join("tenant_ca.crt").exists()
    }

    /// 加载本地证书
    pub fn load_local_certificates(&self) -> Result<(String, String, String), CertError> {
        let cert_dir = self.cert_path();

        let cert_pem = std::fs::read_to_string(cert_dir.join("entity.crt"))
            .map_err(|e| CertError::Storage(e.to_string()))?;
        let key_pem = std::fs::read_to_string(cert_dir.join("entity.key"))
            .map_err(|e| CertError::Storage(e.to_string()))?;
        let ca_cert_pem = std::fs::read_to_string(cert_dir.join("tenant_ca.crt"))
            .map_err(|e| CertError::Storage(e.to_string()))?;

        Ok((cert_pem, key_pem, ca_cert_pem))
    }

    /// 获取或请求证书
    ///
    /// 如果本地有缓存证书则直接返回，否则从 Auth Server 请求新证书
    pub async fn get_or_request_certificates(
        &self,
        auth_url: &str,
        token: &str,
        tenant_id: &str,
    ) -> Result<(String, String, String), CertError> {
        // 检查本地证书
        if self.has_local_certificates() {
            tracing::info!("Using local certificates");
            return self.load_local_certificates();
        }

        // 请求新证书
        tracing::info!("Requesting certificates from {}", auth_url);
        self.request_certificates(auth_url, token, tenant_id).await
    }

    /// 从 Auth Server 请求证书
    pub async fn request_certificates(
        &self,
        auth_url: &str,
        token: &str,
        tenant_id: &str,
    ) -> Result<(String, String, String), CertError> {
        let client = reqwest::Client::new();
        let device_id = crab_cert::generate_hardware_id();

        let response = client
            .post(format!("{}/api/cert/issue", auth_url))
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({
                "tenant_id": tenant_id,
                "common_name": self.client_name,
                "is_server": false,
                "device_id": device_id
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

        let cert_pem = data.get("cert")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CertError::Invalid("No cert in response".to_string()))?
            .to_string();
        let key_pem = data.get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CertError::Invalid("No key in response".to_string()))?
            .to_string();
        let ca_cert_pem = data.get("tenant_ca_cert")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CertError::Invalid("No tenant_ca_cert in response".to_string()))?
            .to_string();

        // 保存证书
        self.save_certificates(&cert_pem, &key_pem, &ca_cert_pem)?;

        tracing::info!("Certificates requested and saved");
        Ok((cert_pem, key_pem, ca_cert_pem))
    }
}
