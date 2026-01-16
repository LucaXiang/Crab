//! Unified certificate management for Crab system

use crate::error::{CertError, Result as CertResult};
use crate::{CertMetadata, generate_hardware_id, verify_chain_against_root};
use std::path::PathBuf;
use std::sync::Arc;

/// Certificate storage structure
#[derive(Debug, Clone)]
pub struct CertStorage {
    pub cert_pem: String,
    pub key_pem: String,
    pub ca_pem: String,
}

/// Certificate service for managing TLS certificates
#[derive(Debug)]
pub struct CertService {
    work_dir: PathBuf,
}

impl CertService {
    /// Create new certificate service
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    /// Store certificates from auth server
    pub async fn store_certificates(&self, storage: CertStorage) -> CertResult<()> {
        use std::fs;

        let certs_dir = self.work_dir.join("certs");
        if !certs_dir.exists() {
            fs::create_dir_all(&certs_dir).map_err(CertError::Io)?;
        }

        fs::write(certs_dir.join("tenant_ca.pem"), storage.ca_pem).map_err(CertError::Io)?;
        fs::write(certs_dir.join("client_cert.pem"), storage.cert_pem).map_err(CertError::Io)?;
        fs::write(certs_dir.join("client_key.pem"), storage.key_pem).map_err(CertError::Io)?;

        Ok(())
    }

    /// Load TLS server configuration
    pub fn load_server_config(&self) -> CertResult<Option<Arc<rustls::ServerConfig>>> {
        use std::fs;

        let certs_dir = self.work_dir.join("certs");
        let ca_path = certs_dir.join("tenant_ca.pem");
        let cert_path = certs_dir.join("client_cert.pem");
        let key_path = certs_dir.join("client_key.pem");

        if !ca_path.exists() || !cert_path.exists() || !key_path.exists() {
            return Ok(None);
        }

        // Load CA certificates
        let ca_pem = fs::read_to_string(&ca_path).map_err(CertError::Io)?;
        let ca_certs =
            crate::to_rustls_certs(&ca_pem).map_err(|_| CertError::InvalidCertificate)?;

        let mut client_auth_roots = rustls::RootCertStore::empty();
        for cert in ca_certs {
            client_auth_roots
                .add(cert)
                .map_err(|_| CertError::InvalidCertificate)?;
        }

        let client_auth =
            rustls::server::WebPkiClientVerifier::builder(Arc::new(client_auth_roots))
                .build()
                .map_err(|_| CertError::InvalidCertificate)?;

        // Load server certificate and key
        let cert_pem = fs::read_to_string(&cert_path).map_err(CertError::Io)?;
        let key_pem = fs::read_to_string(&key_path).map_err(CertError::Io)?;

        let certs = crate::to_rustls_certs(&cert_pem).map_err(|_| CertError::InvalidCertificate)?;
        let key = crate::to_rustls_key(&key_pem).map_err(|_| CertError::InvalidKey)?;

        // Build server configuration
        let config = rustls::ServerConfig::builder()
            .with_client_cert_verifier(client_auth)
            .with_single_cert(certs, key)
            .map_err(|_| CertError::InvalidCertificate)?;

        Ok(Some(Arc::new(config)))
    }

    /// Load TLS client configuration
    pub fn load_client_config(&self) -> CertResult<Option<Arc<rustls::ClientConfig>>> {
        use std::fs;

        let certs_dir = self.work_dir.join("certs");
        let ca_path = certs_dir.join("tenant_ca.pem");
        let cert_path = certs_dir.join("client_cert.pem");
        let key_path = certs_dir.join("client_key.pem");

        if !ca_path.exists() || !cert_path.exists() || !key_path.exists() {
            return Ok(None);
        }

        // Load CA certificates
        let ca_pem = fs::read_to_string(&ca_path).map_err(CertError::Io)?;
        let ca_certs =
            crate::to_rustls_certs(&ca_pem).map_err(|_| CertError::InvalidCertificate)?;

        let mut root_store = rustls::RootCertStore::empty();
        for cert in ca_certs {
            root_store
                .add(cert)
                .map_err(|_| CertError::InvalidCertificate)?;
        }

        // Create custom verifier that skips hostname verification
        let verifier = Arc::new(crate::SkipHostnameVerifier::new(root_store));

        // Load client certificate and key
        let cert_pem = fs::read_to_string(&cert_path).map_err(CertError::Io)?;
        let key_pem = fs::read_to_string(&key_path).map_err(CertError::Io)?;

        let certs = crate::to_rustls_certs(&cert_pem).map_err(|_| CertError::InvalidCertificate)?;
        let key = crate::to_rustls_key(&key_pem).map_err(|_| CertError::InvalidKey)?;

        // Build client configuration
        let config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_client_auth_cert(certs, key)
            .map_err(|_| CertError::InvalidCertificate)?;

        Ok(Some(Arc::new(config)))
    }

    /// Validate certificates with hardware binding
    pub async fn validate_certificates(&self) -> CertResult<()> {
        let (cert_pem, ca_pem) = self.read_certificates()?;

        // Verify certificate chain
        verify_chain_against_root(&cert_pem, &ca_pem)
            .map_err(|e| CertError::VerificationFailed(e.to_string()))?;

        // Check hardware binding
        self.validate_hardware_binding(&cert_pem)?;

        Ok(())
    }

    /// Validate hardware ID binding
    fn validate_hardware_binding(&self, cert_pem: &str) -> CertResult<()> {
        let metadata = CertMetadata::from_pem(cert_pem)
            .map_err(|e| CertError::VerificationFailed(e.to_string()))?;

        let current_hardware_id = generate_hardware_id();

        if let Some(cert_device_id) = metadata.device_id {
            if cert_device_id != current_hardware_id {
                return Err(CertError::VerificationFailed(format!(
                    "Hardware ID mismatch: certificate={}, current={}",
                    cert_device_id, current_hardware_id
                )));
            }
        } else {
            return Err(CertError::VerificationFailed(
                "Certificate missing device_id extension".to_string(),
            ));
        }

        Ok(())
    }

    /// Delete all certificates
    pub async fn cleanup_certificates(&self) -> CertResult<()> {
        let certs_dir = self.work_dir.join("certs");
        if certs_dir.exists() {
            std::fs::remove_dir_all(&certs_dir).map_err(CertError::Io)?;
        }
        Ok(())
    }

    /// Check if certificates exist
    pub fn has_certificates(&self) -> bool {
        let certs_dir = self.work_dir.join("certs");
        certs_dir.exists()
    }

    fn read_certificates(&self) -> CertResult<(String, String)> {
        use std::fs;
        let certs_dir = self.work_dir.join("certs");
        let cert_path = certs_dir.join("client_cert.pem");
        let ca_path = certs_dir.join("tenant_ca.pem");

        if !cert_path.exists() || !ca_path.exists() {
            return Err(CertError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Certificate files not found",
            )));
        }

        let cert_pem = fs::read_to_string(&cert_path).map_err(CertError::Io)?;
        let ca_pem = fs::read_to_string(&ca_path).map_err(CertError::Io)?;

        Ok((cert_pem, ca_pem))
    }
}

/// Credential information
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CredentialInfo {
    pub tenant_id: String,
    pub device_id: Option<String>,
    pub serial_number: String,
}

/// Credential manager
#[allow(dead_code)]
#[derive(Debug)]
pub struct CredentialManager {
    cert_service: CertService,
}

impl CredentialManager {
    /// Create new credential manager
    #[allow(dead_code)]
    pub fn new(work_dir: PathBuf) -> Self {
        Self {
            cert_service: CertService::new(work_dir),
        }
    }

    /// Get certificate service
    #[allow(dead_code)]
    pub fn cert_service(&self) -> &CertService {
        &self.cert_service
    }

    /// Get credential information
    #[allow(dead_code)]
    pub async fn get_credential_info(&self) -> CertResult<Option<CredentialInfo>> {
        if !self.cert_service.has_certificates() {
            return Ok(None);
        }

        let (cert_pem, _) = self.cert_service.read_certificates()?;

        let metadata = CertMetadata::from_pem(&cert_pem)
            .map_err(|e| CertError::VerificationFailed(e.to_string()))?;

        Ok(Some(CredentialInfo {
            tenant_id: metadata.tenant_id.unwrap_or_default(),
            device_id: metadata.device_id,
            serial_number: metadata.serial_number,
        }))
    }

    /// Check if system is ready (has valid certificates)
    #[allow(dead_code)]
    pub async fn is_ready(&self) -> bool {
        self.cert_service.has_certificates()
            && self.cert_service.validate_certificates().await.is_ok()
    }
}
