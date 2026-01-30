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
        self.credential_storage.load().ok_or(CertError::NotFound)
    }

    /// 保存凭证
    pub fn save_credential(&self, credential: &Credential) -> Result<(), CertError> {
        self.credential_storage
            .save(credential)
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
                response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string()),
            ));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| CertError::Network(e.to_string()))?;

        let token = data
            .get("token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CertError::Invalid("No token in response".to_string()))?
            .to_string();

        let tenant_id = data
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let credential = Credential::new(self.client_name.clone(), tenant_id, token, None);

        // 保存凭证
        self.credential_storage
            .save(&credential)
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
        self.credential_storage
            .path()
            .parent()
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
        std::fs::create_dir_all(&cert_dir).map_err(|e| CertError::Storage(e.to_string()))?;

        // 保存客户端证书
        let cert_path = cert_dir.join("entity.crt");
        std::fs::write(&cert_path, cert_pem).map_err(|e| CertError::Storage(e.to_string()))?;

        // 保存客户端私钥
        let key_path = cert_dir.join("entity.key");
        std::fs::write(&key_path, key_pem).map_err(|e| CertError::Storage(e.to_string()))?;

        // 保存 CA 证书
        let ca_path = cert_dir.join("tenant_ca.crt");
        std::fs::write(&ca_path, ca_cert_pem).map_err(|e| CertError::Storage(e.to_string()))?;

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
                response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string()),
            ));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| CertError::Network(e.to_string()))?;

        let cert_pem = data
            .get("cert")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CertError::Invalid("No cert in response".to_string()))?
            .to_string();
        let key_pem = data
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CertError::Invalid("No key in response".to_string()))?
            .to_string();
        let ca_cert_pem = data
            .get("tenant_ca_cert")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CertError::Invalid("No tenant_ca_cert in response".to_string()))?
            .to_string();

        // 保存证书
        self.save_certificates(&cert_pem, &key_pem, &ca_cert_pem)?;

        // 从 API 响应中获取签名和过期时间
        let credential_signature = data
            .get("credential_signature")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let credential_expires_at = data.get("credential_expires_at").and_then(|v| v.as_u64());

        // 更新 credential 的 device_id、signature 和 expires_at
        if let Ok(mut credential) = self.load_credential() {
            credential.device_id = Some(device_id);
            credential.signature = credential_signature;
            credential.expires_at = credential_expires_at;
            if let Err(e) = self.save_credential(&credential) {
                tracing::warn!("Failed to update credential: {}", e);
            }
        }

        tracing::info!("Certificates requested and saved");
        Ok((cert_pem, key_pem, ca_cert_pem))
    }

    /// 执行自检 - 验证本地证书和凭证的完整性
    ///
    /// 验证项目：
    /// 1. 证书文件存在性
    /// 2. 证书链有效性 (entity cert signed by tenant_ca)
    /// 3. 证书过期检查
    /// 4. 硬件 ID 绑定 (防止证书被拷贝到其他机器)
    /// 5. Credential 签名验证 (如果已签名)
    ///
    /// # Returns
    /// - `Ok(())` if all checks pass
    /// - `Err(CertError)` with specific error if any check fails
    pub fn self_check(&self) -> Result<(), CertError> {
        tracing::info!("🔍 Running CertManager self-check...");

        // Step 1: 检查证书文件是否存在
        if !self.has_local_certificates() {
            return Err(CertError::NotFound);
        }

        // Step 2: 加载证书
        let (cert_pem, _key_pem, ca_cert_pem) = self.load_local_certificates()?;

        // Step 3: 验证证书链
        crab_cert::verify_chain_against_root(&cert_pem, &ca_cert_pem).map_err(|e| {
            CertError::Invalid(format!("Certificate chain verification failed: {}", e))
        })?;
        tracing::info!("  ✅ Certificate chain verified.");

        // Step 4: 解析证书元数据并验证
        let metadata = crab_cert::CertMetadata::from_pem(&cert_pem)
            .map_err(|e| CertError::Invalid(format!("Failed to parse certificate: {}", e)))?;

        // Step 5: 检查证书过期
        let now = time::OffsetDateTime::now_utc();
        if metadata.not_after < now {
            return Err(CertError::Expired);
        }

        // 提前 7 天警告
        let warn_threshold = now + time::Duration::days(7);
        if metadata.not_after < warn_threshold {
            let days_left = (metadata.not_after - now).whole_days();
            tracing::warn!(
                "  ⚠️ Certificate will expire in {} days (at {})",
                days_left,
                metadata.not_after
            );
        } else {
            tracing::info!(
                "  ✅ Certificate validity OK (expires: {}).",
                metadata.not_after
            );
        }

        // Step 6: 验证硬件 ID 绑定
        let current_device_id = crab_cert::generate_hardware_id();
        if let Some(cert_device_id) = &metadata.device_id {
            if cert_device_id != &current_device_id {
                return Err(CertError::Invalid(format!(
                    "Hardware ID mismatch: cert bound to {}, current machine is {}",
                    cert_device_id, current_device_id
                )));
            }
            tracing::info!("  ✅ Hardware ID binding verified.");
        } else {
            tracing::warn!("  ⚠️ Certificate has no device_id binding (less secure).");
        }

        // Step 7: 验证 Credential 签名和时钟 (如果存在)
        if let Ok(credential) = self.load_credential() {
            // Step 7a: 时钟篡改检测
            credential
                .check_clock_tampering()
                .map_err(|e| CertError::Invalid(e.to_string()))?;

            // Step 7b: 验证时间戳签名 (使用 Tenant CA 证书)
            credential
                .verify_timestamp_signature(&ca_cert_pem)
                .map_err(|e| CertError::Invalid(e.to_string()))?;
            tracing::info!("  ✅ Clock integrity and timestamp signature verified.");

            // Step 7c: 验证凭证签名
            if credential.is_signed() {
                // 使用 tenant_ca 验证凭证签名
                credential.verify_signature(&ca_cert_pem).map_err(|e| {
                    CertError::Invalid(format!("Credential signature invalid: {}", e))
                })?;

                // 验证设备绑定
                if let Some(cred_device_id) = &credential.device_id
                    && cred_device_id != &current_device_id
                {
                    return Err(CertError::Invalid(format!(
                        "Credential device ID mismatch: {} vs {}",
                        cred_device_id, current_device_id
                    )));
                }
                tracing::info!("  ✅ Credential signature and device binding verified.");
            } else {
                return Err(CertError::Invalid(
                    "Credential is not signed. Please re-activate to obtain a signed credential."
                        .to_string(),
                ));
            }

            // 检查凭证过期
            if credential.is_expired() {
                tracing::warn!("  ⚠️ Credential token has expired (needs refresh).");
            }
        }

        tracing::info!("✅ CertManager self-check passed.");
        Ok(())
    }

    /// 清理所有本地数据 (证书 + 凭证)
    ///
    /// 当自检失败时调用，准备重新激活
    pub fn cleanup(&self) -> Result<(), CertError> {
        let cert_dir = self.cert_path();

        // 删除证书文件
        let files = ["entity.crt", "entity.key", "tenant_ca.crt"];
        for file in &files {
            let path = cert_dir.join(file);
            if path.exists() {
                std::fs::remove_file(&path).map_err(|e| CertError::Storage(e.to_string()))?;
            }
        }

        // 删除凭证
        let _ = self.logout();

        tracing::info!("🧹 Cleanup completed. Ready for reactivation.");
        Ok(())
    }

    /// 验证并加载证书 (带自检)
    ///
    /// 与 `load_local_certificates` 不同，此方法会先进行自检
    pub fn load_verified_certificates(&self) -> Result<(String, String, String), CertError> {
        self.self_check()?;
        self.load_local_certificates()
    }

    /// 刷新时间戳 (从 Auth Server 获取签名)
    ///
    /// 在自检成功后调用。向 Auth Server 请求刷新凭证时间戳。
    /// Auth Server 使用 Tenant CA 签名，防止本地伪造。
    pub async fn refresh_credential_timestamp(&self, auth_url: &str) -> Result<(), CertError> {
        let credential = self.load_credential()?;

        // 构造刷新请求
        let request = serde_json::json!({
            "client_name": credential.client_name,
            "tenant_id": credential.tenant_id,
            "device_id": credential.device_id,
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/api/credential/refresh", auth_url))
            .json(&request)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| CertError::Network(format!("Failed to refresh credential: {}", e)))?;

        if !resp.status().is_success() {
            return Err(CertError::Network(format!(
                "Auth Server returned error: {}",
                resp.status()
            )));
        }

        // 解析响应
        #[derive(serde::Deserialize)]
        struct RefreshResponse {
            success: bool,
            error: Option<String>,
            last_verified_at: Option<u64>,
            last_verified_at_signature: Option<String>,
        }

        let data: RefreshResponse = resp
            .json()
            .await
            .map_err(|e| CertError::Network(format!("Invalid response: {}", e)))?;

        if !data.success {
            return Err(CertError::Network(
                data.error.unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        // 更新凭证
        let mut updated = credential;
        updated.last_verified_at = data.last_verified_at;
        updated.last_verified_at_signature = data.last_verified_at_signature;

        self.save_credential(&updated)?;
        tracing::debug!("Credential timestamp refreshed from Auth Server");
        Ok(())
    }

    /// 完整自检流程 (自检 + 刷新时间戳)
    pub async fn full_self_check(&self, auth_url: &str) -> Result<(), CertError> {
        self.self_check()?;
        // 自检通过后刷新时间戳
        if let Err(e) = self.refresh_credential_timestamp(auth_url).await {
            tracing::warn!("Failed to refresh timestamp (offline?): {}", e);
            // 离线时不阻止启动，使用缓存的时间戳
        }
        Ok(())
    }

    /// 创建支持 mTLS 的 HTTP 客户端
    ///
    /// 使用本地证书创建 reqwest 客户端，用于访问需要 mTLS 认证的 HTTPS 服务。
    /// 使用 `SkipHostnameVerifier` 跳过主机名验证（适用于 IP 地址访问）。
    ///
    /// # Returns
    /// - `Ok(reqwest::Client)` - 配置好的 mTLS 客户端
    /// - `Err(CertError)` - 证书加载失败或配置错误
    pub fn build_mtls_http_client(&self) -> Result<reqwest::Client, CertError> {
        use std::sync::Arc;

        // 加载证书
        let (cert_pem, key_pem, ca_cert_pem) = self.load_local_certificates()?;

        // 解析客户端证书
        let client_certs = crab_cert::to_rustls_certs(&cert_pem)
            .map_err(|e| CertError::Invalid(format!("Failed to parse client cert: {}", e)))?;

        // 解析客户端私钥
        let client_key = crab_cert::to_rustls_key(&key_pem)
            .map_err(|e| CertError::Invalid(format!("Failed to parse client key: {}", e)))?;

        // 解析 CA 证书并创建 RootCertStore
        let ca_certs = crab_cert::to_rustls_certs(&ca_cert_pem)
            .map_err(|e| CertError::Invalid(format!("Failed to parse CA cert: {}", e)))?;

        let mut root_store = rustls::RootCertStore::empty();
        for cert in ca_certs {
            root_store.add(cert).map_err(|e| {
                CertError::Invalid(format!("Failed to add CA cert to store: {}", e))
            })?;
        }

        // 创建 SkipHostnameVerifier (跳过主机名验证)
        let verifier = Arc::new(crab_cert::SkipHostnameVerifier::new(root_store));

        // 创建 rustls ClientConfig
        let config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_client_auth_cert(client_certs, client_key)
            .map_err(|e| CertError::Invalid(format!("Failed to build TLS config: {}", e)))?;

        // 创建 reqwest 客户端
        let client = reqwest::Client::builder()
            .use_preconfigured_tls(config)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| CertError::Network(format!("Failed to build HTTP client: {}", e)))?;

        Ok(client)
    }
}
