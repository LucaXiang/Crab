use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Arc;

use crate::common::AppError;
use crate::server::credential::verify_cert_pair;

#[derive(Clone, Debug)]
pub struct CertService {
    work_dir: PathBuf,
}

impl CertService {
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    pub async fn save_certificates(
        &self,
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

        fs::write(certs_dir.join("tenant_ca.pem"), tenant_ca_pem)
            .map_err(|e| AppError::internal(format!("Failed to write tenant CA: {}", e)))?;
        fs::write(certs_dir.join("edge_cert.pem"), edge_cert_pem)
            .map_err(|e| AppError::internal(format!("Failed to write edge cert: {}", e)))?;
        fs::write(certs_dir.join("edge_key.pem"), edge_key_pem)
            .map_err(|e| AppError::internal(format!("Failed to write edge key: {}", e)))?;

        tracing::info!("📜 Certificates saved to {:?}", certs_dir);
        Ok(())
    }

    pub fn load_tls_config(&self) -> Result<Option<Arc<rustls::ServerConfig>>, AppError> {
        use std::fs;

        let certs_dir = self.work_dir.join("certs");
        let tenant_ca_path = certs_dir.join("tenant_ca.pem");
        let edge_cert_path = certs_dir.join("edge_cert.pem");
        let edge_key_path = certs_dir.join("edge_key.pem");

        if !tenant_ca_path.exists() || !edge_cert_path.exists() || !edge_key_path.exists() {
            return Ok(None);
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
