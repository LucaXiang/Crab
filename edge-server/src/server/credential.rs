//! Credential storage for tenant binding
//!
//! Stores tenant binding information to workspace/cert/Credential.json
//! instead of database.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use x509_cert::der::DecodePem;

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
    pub fn load(cert_dir: &PathBuf) -> Result<Option<Self>, std::io::Error> {
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
    pub fn save(&self, cert_dir: &PathBuf) -> Result<(), std::io::Error> {
        let path = cert_dir.join(CREDENTIAL_FILE);
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Delete credential file
    pub fn delete(cert_dir: &PathBuf) -> Result<(), std::io::Error> {
        let path = cert_dir.join(CREDENTIAL_FILE);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Check if bound
    pub fn is_bound(cert_dir: &PathBuf) -> bool {
        let path = cert_dir.join(CREDENTIAL_FILE);
        path.exists()
    }
}

/// Certificate chain verification for startup self-check
///
/// Trust chain: ROOT_CA → Tenant CA → Server Certificate
/// Uses ROOT_CA to verify Tenant CA is legitimate (company-issued).
/// This is done at startup only.
/// mTLS runtime only uses Tenant CA for isolation.
pub fn verify_certificate_chain(
    cert_dir: &PathBuf,
) -> Result<Credential, CertificateChainError> {
    use std::fs;

    // Load all certificates for startup verification
    let ca_root_path = cert_dir.join("ca_root.crt");
    let tenant_cert_path = cert_dir.join("tenant/tenant.crt");
    let server_cert_path = cert_dir.join("server/edge_server.crt");

    // Check files exist
    if !ca_root_path.exists() {
        return Err(CertificateChainError::MissingFile("ca_root.crt".to_string()));
    }
    if !tenant_cert_path.exists() {
        return Err(CertificateChainError::MissingFile("tenant/tenant.crt".to_string()));
    }
    if !server_cert_path.exists() {
        return Err(CertificateChainError::MissingFile("server/edge_server.crt".to_string()));
    }

    // Read and parse certificates
    let ca_root_pem = fs::read_to_string(&ca_root_path)
        .map_err(|e| CertificateChainError::Io(e, "ca_root.crt".to_string()))?;
    let tenant_cert_pem = fs::read_to_string(&tenant_cert_path)
        .map_err(|e| CertificateChainError::Io(e, "tenant/tenant.crt".to_string()))?;
    let server_cert_pem = fs::read_to_string(&server_cert_path)
        .map_err(|e| CertificateChainError::Io(e, "server/edge_server.crt".to_string()))?;

    // Parse using x509-cert
    let ca_root = x509_cert::Certificate::from_pem(ca_root_pem.as_bytes())
        .map_err(CertificateChainError::ParseX509)?;
    let tenant_cert = x509_cert::Certificate::from_pem(tenant_cert_pem.as_bytes())
        .map_err(CertificateChainError::ParseX509)?;
    let server_cert = x509_cert::Certificate::from_pem(server_cert_pem.as_bytes())
        .map_err(CertificateChainError::ParseX509)?;

    // Verify: ROOT_CA is self-signed
    let ca_root_subject = &ca_root.tbs_certificate.subject;
    let ca_root_issuer = &ca_root.tbs_certificate.issuer;
    if ca_root_issuer != ca_root_subject {
        return Err(CertificateChainError::ChainBroken(
            "Root CA is not self-signed".to_string(),
        ));
    }

    // Verify: Tenant CA issued by ROOT_CA
    let tenant_issuer = &tenant_cert.tbs_certificate.issuer;
    if tenant_issuer != ca_root_subject {
        return Err(CertificateChainError::ChainBroken(
            "Tenant CA is not issued by ROOT_CA".to_string(),
        ));
    }

    // Verify: Server cert issued by Tenant CA
    let server_issuer = &server_cert.tbs_certificate.issuer;
    let tenant_subject = &tenant_cert.tbs_certificate.subject;
    if server_issuer != tenant_subject {
        return Err(CertificateChainError::ChainBroken(
            "Server certificate is not issued by Tenant CA".to_string(),
        ));
    }

    // Calculate server certificate fingerprint
    let fingerprint = calculate_fingerprint_from_pem(&server_cert_pem);

    // Extract tenant_id, server_id, and device_id from certificate CN
    let tenant_cn = extract_cn(&tenant_cert.tbs_certificate.subject)
        .ok_or_else(|| CertificateChainError::MissingCn("tenant_cert".to_string()))?;
    let server_cn = extract_cn(&server_cert.tbs_certificate.subject)
        .ok_or_else(|| CertificateChainError::MissingCn("server_cert".to_string()))?;

    // tenant_id should be CN without "-CA" suffix
    let tenant_id = if tenant_cn.ends_with("-CA") {
        tenant_cn[..tenant_cn.len() - 3].to_string()
    } else {
        tenant_cn.clone()
    };

    // Parse server CN to extract server_id and device_id
    // Expected format: "tenant_id:server_id@device_id" or "tenant_id:server_id"
    let (server_id, device_id) = parse_server_cn(&server_cn, &tenant_id)?;

    tracing::info!("✅ Certificate chain verified (startup self-check)");
    tracing::info!("  Tenant ID: {}", tenant_id);
    tracing::info!("  Server ID: {}", server_id);
    if let Some(ref did) = device_id {
        tracing::info!("  Device ID: {}", did);
    }
    tracing::info!("  Fingerprint: {}...", &fingerprint[..16]);

    Ok(Credential::new(tenant_id, server_id, device_id, fingerprint))
}

/// Calculate SHA256 fingerprint from PEM certificate
fn calculate_fingerprint_from_pem(pem: &str) -> String {
    use sha2::Digest;

    // Convert PEM to DER
    let der = convert_pem_to_der(pem.as_bytes());
    let der = match der {
        Ok(d) => d,
        Err(_) => return "invalid".to_string(),
    };

    let result = sha2::Sha256::digest(&der);
    hex::encode(result)
}

/// Convert PEM to DER
fn convert_pem_to_der(pem: &[u8]) -> Result<Vec<u8>, String> {
    let pem_str = std::str::from_utf8(pem).map_err(|_| "Invalid PEM encoding".to_string())?;

    let start = pem_str
        .find("-----BEGIN CERTIFICATE-----")
        .ok_or_else(|| "No PEM header found".to_string())?;
    let end = pem_str.find("-----END CERTIFICATE-----").ok_or_else(|| "No PEM footer found".to_string())?;

    let base64_content = &pem_str[start + 27..end]
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<String>();

    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_content)
        .map_err(|e| format!("Failed to decode base64: {}", e))
}

/// Extract Common Name from Name
fn extract_cn(name: &x509_cert::name::Name) -> Option<String> {
    use const_oid::ObjectIdentifier;
    use x509_cert::der::asn1::Utf8StringRef;

    let common_name_oid = ObjectIdentifier::from_arcs([2u32, 5, 4, 3]).ok()?;

    for rdn in name.0.iter() {
        for attr in rdn.0.iter() {
            if attr.oid == common_name_oid {
                let any = x509_cert::der::Any::from(&attr.value);
                if let Ok(utf8_ref) = any.decode_as::<Utf8StringRef>() {
                    return Some(utf8_ref.to_string());
                }
            }
        }
    }
    None
}

/// Parse server certificate CN to extract server_id and device_id
///
/// Expected CN format: "tenant_id:server_id@device_id" or "tenant_id:server_id"
///
/// # Returns
///
/// Returns (server_id, Option<device_id>)
fn parse_server_cn(
    server_cn: &str,
    expected_tenant_id: &str,
) -> Result<(String, Option<String>), CertificateChainError> {
    // Split by '@' to separate device_id
    let parts: Vec<&str> = server_cn.split('@').collect();

    let (prefix, device_id) = match parts.len() {
        1 => (parts[0], None),
        2 => (parts[0], Some(parts[1].to_string())),
        _ => {
            return Err(CertificateChainError::ChainBroken(format!(
                "Invalid server CN format: {}",
                server_cn
            )));
        }
    };

    // Split prefix by ':' to get tenant_id and server_id
    let prefix_parts: Vec<&str> = prefix.split(':').collect();

    // Accept both formats:
    // 1. Simple: "server_id" (e.g., "dev-edge-server-001")
    // 2. Full: "tenant_id:server_id" (e.g., "dev-tenant-001:dev-edge-server-001")
    let (tenant_id, server_id) = if prefix_parts.len() == 2 {
        // Full format
        let tenant_id = prefix_parts[0];
        let server_id = prefix_parts[1];

        // Verify tenant_id matches
        if tenant_id != expected_tenant_id {
            return Err(CertificateChainError::ChainBroken(format!(
                "Tenant ID mismatch in server CN: expected {}, found {}",
                expected_tenant_id, tenant_id
            )));
        }

        (tenant_id.to_string(), server_id.to_string())
    } else if prefix_parts.len() == 1 {
        // Simple format - accept without tenant validation
        (expected_tenant_id.to_string(), prefix_parts[0].to_string())
    } else {
        return Err(CertificateChainError::ChainBroken(format!(
            "Invalid server CN prefix format: {}",
            prefix
        )));
    };

    Ok((server_id, device_id))
}

/// Certificate chain verification error
#[derive(Debug, thiserror::Error)]
pub enum CertificateChainError {
    #[error("Missing file: {0}")]
    MissingFile(String),

    #[error("IO error reading {1}: {0}")]
    Io(std::io::Error, String),

    #[error("Failed to parse X509 certificate: {0}")]
    ParseX509(#[from] x509_cert::der::Error),

    #[error("Certificate chain is broken: {0}")]
    ChainBroken(String),

    #[error("Missing Common Name in {0}")]
    MissingCn(String),
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
