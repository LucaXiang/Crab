use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Arc;

use crate::services::credential::verify_cert_pair;
use crate::utils::AppError;

/// 证书服务 - 管理 mTLS 证书和信任链验证
///
/// # 证书文件布局
///
/// ```text
/// work_dir/certs/
/// ├── root_ca.pem      # 根证书 (用于验证 tenant_ca)
/// ├── tenant_ca.pem    # 租户 CA 证书 (用于验证客户端)
/// ├── edge_cert.pem    # 边缘服务器证书
/// └── edge_key.pem     # 边缘服务器私钥
/// ```
///
/// # 职责
///
/// - 证书保存 (`save_certificates`)
/// - TLS 配置加载 (`load_tls_config`)
/// - 证书自检 (`self_check`)
/// - 证书删除 (`delete_certificates`)
/// - Root CA 下载和验证 (`download_root_ca`, `verify_certificate_chain`)
#[derive(Clone, Debug)]
pub struct CertService {
    /// 工作目录
    work_dir: PathBuf,
}

impl CertService {
    /// 创建证书服务
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    /// 下载并保存 Root CA 证书
    pub async fn download_root_ca(&self, auth_url: &str) -> Result<String, AppError> {
        let client = reqwest::Client::new();

        let resp = client
            .get(format!("{}/pki/root_ca", auth_url))
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Failed to download root CA: {}", e)))?;

        if !resp.status().is_success() {
            return Err(AppError::internal(format!(
                "Root CA download failed: HTTP {}",
                resp.status()
            )));
        }

        // 解析JSON响应，提取PEM内容
        let json_response: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::internal(format!("Failed to parse root CA JSON: {}", e)))?;

        // 提取root_ca_cert字段
        let root_ca_pem = json_response
            .get("root_ca_cert")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::internal("Response missing root_ca_cert field".to_string()))?;

        // 验证 Root CA 格式
        if !root_ca_pem.contains("BEGIN CERTIFICATE") {
            return Err(AppError::validation(
                "Invalid root CA format in JSON response",
            ));
        }

        // 保存 Root CA
        self.save_root_ca(root_ca_pem).await?;

        tracing::info!("✅ Root CA downloaded and saved successfully");
        Ok(root_ca_pem.to_string())
    }

    /// 验证证书链 (Root CA -> Tenant CA -> Edge Cert)
    pub async fn verify_certificate_chain(
        &self,
        root_ca_pem: &str,
        tenant_ca_pem: &str,
        edge_cert_pem: &str,
    ) -> Result<(), AppError> {
        // 1. 验证 Tenant CA 是否被 Root CA 签发
        crab_cert::verify_chain_against_root(tenant_ca_pem, root_ca_pem)
            .map_err(|e| AppError::validation(format!("Tenant CA validation failed: {}", e)))?;

        // 2. 验证 Edge Cert 是否被 Tenant CA 签发
        crab_cert::verify_chain_against_root(edge_cert_pem, tenant_ca_pem)
            .map_err(|e| AppError::validation(format!("Edge cert validation failed: {}", e)))?;

        tracing::info!(
            "✅ Certificate chain verification passed: Root CA -> Tenant CA -> Edge Cert"
        );
        Ok(())
    }

    /// 保存 Root CA 证书
    pub async fn save_root_ca(&self, root_ca_pem: &str) -> Result<(), AppError> {
        use std::fs;

        let certs_dir = self.work_dir.join("certs");
        if !certs_dir.exists() {
            fs::create_dir_all(&certs_dir)
                .map_err(|e| AppError::internal(format!("Failed to create certs dir: {}", e)))?;
        }

        fs::write(certs_dir.join("root_ca.pem"), root_ca_pem)
            .map_err(|e| AppError::internal(format!("Failed to write root CA: {}", e)))?;

        Ok(())
    }

    /// 保存证书文件 (PEM 格式)
    ///
    /// 保存到 `work_dir/certs/` 目录
    pub async fn save_certificates(
        &self,
        root_ca_pem: &str,
        tenant_ca_pem: &str,
        edge_cert_pem: &str,
        edge_key_pem: &str,
    ) -> Result<(), AppError> {
        use std::fs;

        let certs_dir = self.work_dir.join("certs");
        if !certs_dir.exists() {
            fs::create_dir_all(&certs_dir)
                .map_err(|e| AppError::internal(format!("Failed to create certs dir: {}", e)))?;
        }

        // 保存所有证书文件
        fs::write(certs_dir.join("root_ca.pem"), root_ca_pem)
            .map_err(|e| AppError::internal(format!("Failed to write root CA: {}", e)))?;
        fs::write(certs_dir.join("tenant_ca.pem"), tenant_ca_pem)
            .map_err(|e| AppError::internal(format!("Failed to write tenant CA: {}", e)))?;
        fs::write(certs_dir.join("edge_cert.pem"), edge_cert_pem)
            .map_err(|e| AppError::internal(format!("Failed to write edge cert: {}", e)))?;
        fs::write(certs_dir.join("edge_key.pem"), edge_key_pem)
            .map_err(|e| AppError::internal(format!("Failed to write edge key: {}", e)))?;

        tracing::info!("📜 Certificates saved to {:?}", certs_dir);
        Ok(())
    }

    /// 加载 mTLS 配置
    ///
    /// # 返回
    ///
    /// - `Ok(Some(config))` - 证书存在，加载成功
    /// - `Ok(None)` - 证书文件不存在
    /// - `Err(...)` - 加载失败
    pub fn load_tls_config(&self) -> Result<Option<Arc<rustls::ServerConfig>>, AppError> {
        use std::fs;

        let certs_dir = self.work_dir.join("certs");
        let tenant_ca_path = certs_dir.join("tenant_ca.pem");
        let edge_cert_path = certs_dir.join("edge_cert.pem");
        let edge_key_path = certs_dir.join("edge_key.pem");

        // 检查必需的证书文件
        if !tenant_ca_path.exists() || !edge_cert_path.exists() || !edge_key_path.exists() {
            return Ok(None);
        }

        // 可选：检查 root_ca 是否存在
        let root_ca_path = certs_dir.join("root_ca.pem");
        let has_root_ca = root_ca_path.exists();

        // 如果存在 root_ca，验证证书链完整性
        if has_root_ca {
            tracing::info!("Verifying certificate chain with Root CA...");
            let root_ca_pem = fs::read_to_string(&root_ca_path)
                .map_err(|e| AppError::internal(format!("Failed to read root CA: {}", e)))?;
            let tenant_ca_pem = fs::read_to_string(&tenant_ca_path)
                .map_err(|e| AppError::internal(format!("Failed to read tenant CA: {}", e)))?;
            let edge_cert_pem = fs::read_to_string(&edge_cert_path)
                .map_err(|e| AppError::internal(format!("Failed to read edge cert: {}", e)))?;

            // 验证证书链
            match crab_cert::verify_chain_against_root(&tenant_ca_pem, &root_ca_pem) {
                Ok(_) => tracing::info!("✅ Root CA -> Tenant CA verification passed"),
                Err(e) => {
                    tracing::warn!("⚠️ Root CA -> Tenant CA verification failed: {}", e);
                    return Err(AppError::validation(
                        "Certificate chain validation failed: Root CA verification failed"
                            .to_string(),
                    ));
                }
            }

            match crab_cert::verify_chain_against_root(&edge_cert_pem, &tenant_ca_pem) {
                Ok(_) => tracing::info!("✅ Tenant CA -> Edge Cert verification passed"),
                Err(e) => {
                    tracing::warn!("⚠️ Tenant CA -> Edge Cert verification failed: {}", e);
                    return Err(AppError::validation(
                        "Certificate chain validation failed: Edge cert verification failed"
                            .to_string(),
                    ));
                }
            }
        } else {
            tracing::warn!("⚠️ Root CA not found - certificate chain cannot be fully verified");
        }

        tracing::info!("🔒 Loading mTLS certificates from {:?}", certs_dir);

        // 1. Load CA certs for client verification
        let ca_pem = fs::read_to_string(&tenant_ca_path)
            .map_err(|e| AppError::internal(format!("Failed to read tenant CA: {}", e)))?;
        let ca_certs = crab_cert::to_rustls_certs(&ca_pem)
            .map_err(|e| AppError::internal(format!("Failed to parse tenant CA: {}", e)))?;

        let mut client_auth_roots = rustls::RootCertStore::empty();
        for cert in ca_certs {
            client_auth_roots.add(cert).map_err(|e| {
                AppError::internal(format!("Failed to add CA cert to store: {}", e))
            })?;
        }

        let client_auth =
            rustls::server::WebPkiClientVerifier::builder(Arc::new(client_auth_roots))
                .build()
                .map_err(|e| {
                    AppError::internal(format!("Failed to build client verifier: {}", e))
                })?;

        // 2. Load server cert and key
        let cert_pem = fs::read_to_string(&edge_cert_path)
            .map_err(|e| AppError::internal(format!("Failed to read edge cert: {}", e)))?;
        let key_pem = fs::read_to_string(&edge_key_path)
            .map_err(|e| AppError::internal(format!("Failed to read edge key: {}", e)))?;

        let certs = crab_cert::to_rustls_certs(&cert_pem)
            .map_err(|e| AppError::internal(format!("Failed to parse edge cert: {}", e)))?;
        let key = crab_cert::to_rustls_key(&key_pem)
            .map_err(|e| AppError::internal(format!("Failed to parse edge key: {}", e)))?;

        // 3. Build ServerConfig
        let config = rustls::ServerConfig::builder()
            .with_client_cert_verifier(client_auth)
            .with_single_cert(certs, key)
            .map_err(|e| AppError::internal(format!("Failed to build server config: {}", e)))?;

        Ok(Some(Arc::new(config)))
    }

    pub fn delete_certificates(&self) -> Result<(), AppError> {
        let certs_dir = self.work_dir.join("certs");
        if certs_dir.exists() {
            tracing::info!("🗑️ Removing invalid certificates from {:?}", certs_dir);
            std::fs::remove_dir_all(&certs_dir)
                .map_err(|e| AppError::internal(format!("Failed to delete certs dir: {}", e)))?;
        }
        Ok(())
    }

    pub fn get_fingerprint(pem_content: &str) -> String {
        match crab_cert::to_rustls_certs(pem_content) {
            Ok(certs) => {
                if let Some(cert) = certs.first() {
                    let mut hasher = Sha256::new();
                    hasher.update(cert.as_ref());
                    let result = hasher.finalize();
                    hex::encode(result)
                } else {
                    "Unknown (No Certs Found)".to_string()
                }
            }
            Err(_) => "Unknown (Parse Error)".to_string(),
        }
    }

    pub fn truncate_fingerprint(fp: &str) -> String {
        if fp.len() > 40 {
            format!("{}...", &fp[0..40])
        } else {
            fp.to_string()
        }
    }

    /// 执行开机自检
    ///
    /// 验证项目：
    /// 1. 证书文件存在性
    /// 2. 证书链有效性 (签名、过期时间)
    /// 3. 硬件 ID 绑定 (防止证书被拷贝到其他机器)
    pub async fn self_check(&self) -> Result<(), AppError> {
        tracing::info!("🔍 Running CertService self-check...");
        let (cert_pem, ca_pem) = self.read_certs()?;

        // verify_cert_pair 包含完整的校验逻辑：
        // 1. Chain validity
        // 2. Metadata presence
        // 3. Hardware ID match
        verify_cert_pair(&cert_pem, &ca_pem)
            .map_err(|e| AppError::validation(format!("Self-check failed: {}", e)))?;

        tracing::info!("✅ CertService self-check passed: Hardware ID and Chain verified.");
        Ok(())
    }

    /// 清理证书链文件
    ///
    /// 当自检失败时调用，删除旧的证书文件以等待重新激活
    pub async fn cleanup_certificates(&self) -> Result<(), AppError> {
        tracing::warn!("🧹 Cleaning up certificate files after self-check failure...");

        let certs_dir = self.work_dir.join("certs");
        let edge_cert_path = certs_dir.join("edge_cert.pem");
        let tenant_ca_path = certs_dir.join("tenant_ca.pem");

        // 删除证书文件
        if edge_cert_path.exists() {
            std::fs::remove_file(&edge_cert_path)
                .map_err(|e| AppError::internal(format!("Failed to remove edge cert: {}", e)))?;
            tracing::info!("Removed edge certificate file");
        }

        if tenant_ca_path.exists() {
            std::fs::remove_file(&tenant_ca_path)
                .map_err(|e| AppError::internal(format!("Failed to remove tenant CA: {}", e)))?;
            tracing::info!("Removed tenant CA certificate file");
        }

        tracing::warn!("✅ Certificate cleanup completed. Server will wait for reactivation.");
        Ok(())
    }

    fn read_certs(&self) -> Result<(String, String), AppError> {
        use std::fs;
        let certs_dir = self.work_dir.join("certs");
        let edge_cert_path = certs_dir.join("edge_cert.pem");
        let tenant_ca_path = certs_dir.join("tenant_ca.pem");

        if !edge_cert_path.exists() || !tenant_ca_path.exists() {
            return Err(AppError::internal("Certificates not found"));
        }

        let cert_pem = fs::read_to_string(&edge_cert_path)
            .map_err(|e| AppError::internal(format!("Failed to read edge cert: {}", e)))?;
        let ca_pem = fs::read_to_string(&tenant_ca_path)
            .map_err(|e| AppError::internal(format!("Failed to read tenant CA: {}", e)))?;

        Ok((cert_pem, ca_pem))
    }
}
