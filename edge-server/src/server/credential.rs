//! Credential storage for tenant binding
//!
//! Stores tenant binding information to workspace/cert/Credential.json
//! instead of database.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Credential storage location
pub const CREDENTIAL_FILE: &str = "Credential.json";

/// Credential stored for a bound tenant
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Credential {
    /// Tenant ID
    pub tenant_id: String,
    /// Server ID
    pub server_id: String,
    /// Device ID (Hardware ID from certificate)
    #[serde(default)]
    pub device_id: Option<String>,
    /// Certificate fingerprint (SHA256 of server certificate)
    pub fingerprint: String,
    /// Bound timestamp (RFC3339)
    pub bound_at: String,
}

impl Credential {
    /// Create a new credential
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
        }
    }

    /// Load credential from file
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

    /// Save credential to file
    pub fn save(&self, cert_dir: &Path) -> Result<(), std::io::Error> {
        let path = cert_dir.join(CREDENTIAL_FILE);
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Delete credential file
    pub fn delete(cert_dir: &Path) -> Result<(), std::io::Error> {
        let path = cert_dir.join(CREDENTIAL_FILE);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Check if bound
    pub fn is_bound(cert_dir: &Path) -> bool {
        let path = cert_dir.join(CREDENTIAL_FILE);
        path.exists()
    }
}

/// Verify a certificate pair (Server Cert signed by CA)
pub fn verify_cert_pair(
    server_cert_pem: &str,
    ca_cert_pem: &str,
) -> Result<Credential, CertificateChainError> {
    // 1. Verify chain using crab-cert
    crab_cert::verify_server_cert(server_cert_pem, ca_cert_pem)?;

    // 2. Extract metadata from Server Cert
    let server_meta = crab_cert::CertMetadata::from_pem(server_cert_pem)
        .map_err(CertificateChainError::VerificationFailed)?;

    // 3. Extract tenant_id
    // Priority: 1. Extension in Server Cert, 2. Extension in CA Cert, 3. CN of CA Cert
    let tenant_id = server_meta
        .tenant_id
        .or_else(|| {
            let ca_meta = crab_cert::CertMetadata::from_pem(ca_cert_pem).ok()?;
            ca_meta.tenant_id.or(ca_meta.common_name)
        })
        .ok_or_else(|| CertificateChainError::InvalidMetadata("Missing Tenant ID".to_string()))?;

    let server_id = server_meta.common_name.ok_or_else(|| {
        CertificateChainError::InvalidMetadata("Missing Common Name (Server ID)".to_string())
    })?;

    tracing::info!("✅ Certificate pair verified");
    tracing::info!("  Tenant ID: {}", tenant_id);
    tracing::info!("  Server ID: {}", server_id);
    tracing::info!(
        "  Fingerprint: {}...",
        &server_meta.fingerprint_sha256[..16]
    );

    Ok(Credential::new(
        tenant_id,
        server_id,
        server_meta.device_id,
        server_meta.fingerprint_sha256,
    ))
}

/// Certificate chain verification for startup self-check
///
/// Trust chain: ROOT_CA → Tenant CA → Server Certificate
pub fn verify_certificate_chain(cert_dir: &Path) -> Result<Credential, CertificateChainError> {
    use std::fs;

    // Load all certificates for startup verification
    let ca_root_path = cert_dir.join("ca_root.crt");
    let tenant_cert_path = cert_dir.join("tenant/tenant.crt");
    let server_cert_path = cert_dir.join("server/edge_server.crt");

    // Check files exist
    if !ca_root_path.exists() {
        return Err(CertificateChainError::MissingFile(
            "ca_root.crt".to_string(),
        ));
    }
    if !tenant_cert_path.exists() {
        return Err(CertificateChainError::MissingFile(
            "tenant/tenant.crt".to_string(),
        ));
    }
    if !server_cert_path.exists() {
        return Err(CertificateChainError::MissingFile(
            "server/edge_server.crt".to_string(),
        ));
    }

    // Read certificates
    let ca_root_pem = fs::read_to_string(&ca_root_path)
        .map_err(|e| CertificateChainError::Io(e, "ca_root.crt".to_string()))?;
    let tenant_cert_pem = fs::read_to_string(&tenant_cert_path)
        .map_err(|e| CertificateChainError::Io(e, "tenant/tenant.crt".to_string()))?;
    let server_cert_pem = fs::read_to_string(&server_cert_path)
        .map_err(|e| CertificateChainError::Io(e, "server/edge_server.crt".to_string()))?;

    // Verify: ROOT_CA (Self-signed check)
    // We check if it can verify itself
    crab_cert::verify_server_cert(&ca_root_pem, &ca_root_pem).map_err(|_| {
        CertificateChainError::ChainBroken("Root CA is not self-signed or invalid".to_string())
    })?;

    // Verify: Tenant CA issued by ROOT_CA
    crab_cert::verify_server_cert(&tenant_cert_pem, &ca_root_pem).map_err(|e| {
        CertificateChainError::ChainBroken(format!("Tenant CA verification failed: {}", e))
    })?;

    // Verify: Server Cert issued by Tenant CA
    // This also returns the Credential
    verify_cert_pair(&server_cert_pem, &tenant_cert_pem)
}

/// Certificate chain verification error
#[derive(Debug, thiserror::Error)]
pub enum CertificateChainError {
    #[error("Missing file: {0}")]
    MissingFile(String),

    #[error("IO error reading {1}: {0}")]
    Io(std::io::Error, String),

    #[error("Certificate verification failed: {0}")]
    VerificationFailed(#[from] crab_cert::CertError),

    #[error("Certificate chain is broken: {0}")]
    ChainBroken(String),

    #[error("Invalid certificate metadata: {0}")]
    InvalidMetadata(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_new() {
        let cred = Credential::new(
            "dev-tenant-001".to_string(),
            "dev-edge-server-001".to_string(),
            Some("hardware-id-123".to_string()),
            "abc123...".to_string(),
        );
        assert_eq!(cred.tenant_id, "dev-tenant-001");
        assert_eq!(cred.server_id, "dev-edge-server-001");
        assert_eq!(cred.device_id, Some("hardware-id-123".to_string()));
        assert_eq!(cred.fingerprint, "abc123...");
        assert!(!cred.bound_at.is_empty());
    }

    #[test]
    fn test_parse_server_cn() {
        // Test with device_id
        let result = parse_server_cn("tenant-001:server-001@device-123", "tenant-001");
        assert!(result.is_ok());
        let (server_id, device_id) = result.unwrap();
        assert_eq!(server_id, "server-001");
        assert_eq!(device_id, Some("device-123".to_string()));

        // Test without device_id
        let result = parse_server_cn("tenant-001:server-001", "tenant-001");
        assert!(result.is_ok());
        let (server_id, device_id) = result.unwrap();
        assert_eq!(server_id, "server-001");
        assert_eq!(device_id, None);

        // Test tenant mismatch
        let result = parse_server_cn("tenant-002:server-001@device-123", "tenant-001");
        assert!(result.is_err());
    }

    #[test]
    fn test_credential_load_not_exists() {
        let dir = PathBuf::from("/nonexistent");
        let result = Credential::load(&dir);
        assert!(result.unwrap().is_none());
    }
}
