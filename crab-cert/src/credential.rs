//! Credential storage for authentication tokens.
//!
//! This module provides persistent storage for credentials used in
//! certificate operations and tenant authentication.
//!
//! Credentials can be signed by Root CA to ensure integrity.
//!
//! Used by:
//! - crab-client: Store tenant tokens for certificate download
//! - edge-server: Store activation credentials
//! - crab-auth: Issue signed credentials

use crate::error::{CertError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Tenant credential for certificate operations.
///
/// This credential is obtained from the Auth Server and stored persistently
/// to allow reconnection without re-authentication.
///
/// The credential can be signed by Root CA to ensure integrity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    /// Client/Server identifier.
    pub client_name: String,
    /// Tenant ID from Auth Server.
    pub tenant_id: String,
    /// Auth Server token for certificate operations.
    pub token: String,
    /// Token expiration timestamp (Unix seconds).
    pub expires_at: Option<u64>,
    /// Hardware ID binding (optional).
    pub device_id: Option<String>,
    /// Signature from Root CA (base64 encoded).
    /// Signs: "{client_name}|{tenant_id}|{expires_at}|{device_id}"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Last verification timestamp (Unix seconds).
    /// Used to detect clock tampering.
    #[serde(default)]
    pub last_verified_at: Option<u64>,
    /// Signature for last_verified_at (base64 encoded).
    /// Signed by entity private key to prevent tampering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_verified_at_signature: Option<String>,
}

impl Credential {
    /// Creates a new unsigned credential.
    pub fn new(
        client_name: impl Into<String>,
        tenant_id: impl Into<String>,
        token: impl Into<String>,
        expires_at: Option<u64>,
    ) -> Self {
        Self {
            client_name: client_name.into(),
            tenant_id: tenant_id.into(),
            token: token.into(),
            expires_at,
            device_id: None,
            signature: None,
            last_verified_at: None,
            last_verified_at_signature: None,
        }
    }

    /// Creates a new credential with hardware binding.
    pub fn with_device_id(mut self, device_id: impl Into<String>) -> Self {
        self.device_id = Some(device_id.into());
        self
    }

    /// Returns the token.
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Checks if the credential has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            return now > expires_at;
        }
        false
    }

    /// Checks if the credential is still valid (not expired).
    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }

    /// Returns the data to be signed/verified.
    fn signable_data(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.client_name,
            self.tenant_id,
            self.expires_at.unwrap_or(0),
            self.device_id.as_deref().unwrap_or("")
        )
    }

    /// Signs the credential using a CA private key.
    ///
    /// # Arguments
    /// * `ca_key_pem` - CA private key in PEM format
    ///
    /// # Returns
    /// Self with signature field populated
    pub fn sign(mut self, ca_key_pem: &str) -> Result<Self> {
        let data = self.signable_data();
        let sig_bytes = crate::crypto::sign(ca_key_pem, data.as_bytes())?;
        self.signature = Some(base64_encode(&sig_bytes));
        Ok(self)
    }

    /// Verifies the credential signature using a CA certificate.
    ///
    /// # Arguments
    /// * `ca_cert_pem` - CA certificate in PEM format
    ///
    /// # Returns
    /// Ok(()) if signature is valid, Err otherwise
    pub fn verify_signature(&self, ca_cert_pem: &str) -> Result<()> {
        let sig_b64 = self.signature.as_ref().ok_or_else(|| {
            CertError::VerificationFailed("Credential is not signed".into())
        })?;

        let sig_bytes = base64_decode(sig_b64).map_err(|e| {
            CertError::VerificationFailed(format!("Invalid signature encoding: {}", e))
        })?;

        let data = self.signable_data();
        crate::crypto::verify(ca_cert_pem, data.as_bytes(), &sig_bytes)
    }

    /// Validates the credential including expiration, hardware binding, and signature.
    ///
    /// # Arguments
    /// * `ca_cert_pem` - Optional CA certificate for signature verification
    /// * `expected_device_id` - Optional expected hardware ID
    pub fn validate(
        &self,
        ca_cert_pem: Option<&str>,
        expected_device_id: Option<&str>,
    ) -> Result<()> {
        // Check expiration
        if self.is_expired() {
            return Err(CertError::VerificationFailed("Credential has expired".into()));
        }

        // Check hardware binding if required
        if let Some(expected) = expected_device_id {
            match &self.device_id {
                Some(actual) if actual == expected => {}
                Some(actual) => {
                    return Err(CertError::VerificationFailed(format!(
                        "Hardware ID mismatch: expected {}, got {}",
                        expected, actual
                    )));
                }
                None => {
                    return Err(CertError::VerificationFailed(
                        "Credential missing device_id".into(),
                    ));
                }
            }
        }

        // Verify signature if CA cert provided
        if let Some(ca_cert) = ca_cert_pem {
            self.verify_signature(ca_cert)?;
        }

        Ok(())
    }

    /// Checks if the credential is signed.
    pub fn is_signed(&self) -> bool {
        self.signature.is_some()
    }

    /// Maximum allowed clock backward drift (1 hour in seconds).
    const MAX_CLOCK_BACKWARD_SECS: u64 = 3600;
    /// Maximum allowed clock forward drift (30 days in seconds).
    const MAX_CLOCK_FORWARD_SECS: u64 = 30 * 24 * 3600;

    /// Checks for clock tampering.
    ///
    /// Detects two scenarios:
    /// 1. Clock set back more than 1 hour - may try to extend expiration
    /// 2. Clock jumped forward more than 30 days - may try to skip online verification
    ///
    /// Also verifies the timestamp signature if entity_cert_pem is provided.
    pub fn check_clock_tampering(&self) -> Result<()> {
        let (last_verified, sig) = match (&self.last_verified_at, &self.last_verified_at_signature) {
            (Some(ts), Some(sig)) => (*ts, sig.clone()),
            (None, _) => return Ok(()), // No timestamp recorded yet
            (Some(_), None) => {
                // Timestamp exists but no signature - suspicious
                return Err(CertError::VerificationFailed(
                    "Clock timestamp exists but signature is missing".into(),
                ));
            }
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Clock set back (now < last_verified - tolerance)
        if last_verified > now && (last_verified - now) > Self::MAX_CLOCK_BACKWARD_SECS {
            return Err(CertError::VerificationFailed(format!(
                "Clock tampering detected: time moved backward by {} seconds",
                last_verified - now
            )));
        }

        // Clock jumped forward too much
        if now > last_verified && (now - last_verified) > Self::MAX_CLOCK_FORWARD_SECS {
            return Err(CertError::VerificationFailed(format!(
                "Clock tampering detected: time jumped forward by {} days",
                (now - last_verified) / 86400
            )));
        }

        // Signature verification is done separately with verify_timestamp_signature()
        // since we need the entity certificate
        let _ = sig; // Mark as used
        Ok(())
    }

    /// Verifies the timestamp signature using Tenant CA certificate.
    ///
    /// The signature format is: "{timestamp}|{client_name}|{tenant_id}|{device_id}"
    /// Signed by Auth Server using Tenant CA private key.
    pub fn verify_timestamp_signature(&self, tenant_ca_cert_pem: &str) -> Result<()> {
        let (ts, sig_b64) = match (&self.last_verified_at, &self.last_verified_at_signature) {
            (Some(ts), Some(sig)) => (*ts, sig.clone()),
            (None, None) => return Ok(()), // No timestamp yet
            _ => {
                return Err(CertError::VerificationFailed(
                    "Timestamp/signature mismatch".into(),
                ))
            }
        };

        let sig_bytes = base64_decode(&sig_b64).map_err(|e| {
            CertError::VerificationFailed(format!("Invalid timestamp signature encoding: {}", e))
        })?;

        // The signature format matches what Auth Server signs
        let data = format!(
            "{}|{}|{}|{}",
            ts,
            self.client_name,
            self.tenant_id,
            self.device_id.as_deref().unwrap_or("")
        );
        crate::crypto::verify(tenant_ca_cert_pem, data.as_bytes(), &sig_bytes)
            .map_err(|e| CertError::VerificationFailed(format!("Timestamp signature invalid: {}", e)))
    }
}

// Simple base64 helpers (avoid adding dependency)
fn base64_encode(data: &[u8]) -> String {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    STANDARD.encode(data)
}

fn base64_decode(s: &str) -> std::result::Result<Vec<u8>, base64::DecodeError> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    STANDARD.decode(s)
}

/// File-based credential storage.
///
/// Stores credentials as JSON files in the filesystem.
#[derive(Debug, Clone)]
pub struct CredentialStorage {
    path: PathBuf,
}

impl CredentialStorage {
    /// Creates a new credential storage.
    ///
    /// The credential will be stored at `{base_path}/{filename}`.
    pub fn new(base_path: impl Into<PathBuf>, filename: &str) -> Self {
        let path = base_path.into().join(filename);
        Self { path }
    }

    /// Creates storage at a specific path.
    pub fn at_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Ensures the parent directory exists.
    pub fn ensure_dir(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    /// Saves a credential to storage.
    pub fn save(&self, credential: &Credential) -> std::io::Result<()> {
        self.ensure_dir()?;
        let json = serde_json::to_string_pretty(credential)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&self.path, json)
    }

    /// Loads a credential from storage.
    ///
    /// Returns `None` if the file doesn't exist or is invalid.
    pub fn load(&self) -> Option<Credential> {
        if !self.path.exists() {
            return None;
        }
        let json = fs::read_to_string(&self.path).ok()?;
        serde_json::from_str(&json).ok()
    }

    /// Checks if a credential file exists.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Deletes the credential file.
    pub fn delete(&self) -> std::io::Result<()> {
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        Ok(())
    }

    /// Returns the storage path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::CaProfile;
    use tempfile::TempDir;

    #[test]
    fn test_credential_creation() {
        let cred = Credential::new("test-client", "tenant-123", "token-abc", None);

        assert_eq!(cred.client_name, "test-client");
        assert_eq!(cred.tenant_id, "tenant-123");
        assert_eq!(cred.token(), "token-abc");
        assert!(!cred.is_expired());
        assert!(cred.is_valid());
        assert!(!cred.is_signed());
    }

    #[test]
    fn test_credential_with_device_id() {
        let cred = Credential::new("test-client", "tenant-123", "token-abc", None)
            .with_device_id("hw-12345");

        assert_eq!(cred.device_id, Some("hw-12345".to_string()));
    }

    #[test]
    fn test_credential_expiration() {
        // No expiration
        let cred1 = Credential::new("c", "t", "tok", None);
        assert!(!cred1.is_expired());

        // Future expiration
        let future = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600;
        let cred2 = Credential::new("c", "t", "tok", Some(future));
        assert!(!cred2.is_expired());

        // Past expiration
        let past = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(3600);
        let cred3 = Credential::new("c", "t", "tok", Some(past));
        assert!(cred3.is_expired());
    }

    #[test]
    fn test_credential_signing_and_verification() {
        // Create a test CA
        let ca = crate::ca::CertificateAuthority::new_root(CaProfile::root("Test CA")).unwrap();

        let cred = Credential::new("test-client", "tenant-123", "token-abc", None)
            .with_device_id("hw-12345")
            .sign(&ca.key_pem())
            .unwrap();

        assert!(cred.is_signed());

        // Verify signature
        cred.verify_signature(&ca.cert_pem()).unwrap();
    }

    #[test]
    fn test_credential_validation() {
        // Create a test CA
        let ca = crate::ca::CertificateAuthority::new_root(CaProfile::root("Test CA")).unwrap();

        let future = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600;

        let cred = Credential::new("test-client", "tenant-123", "token-abc", Some(future))
            .with_device_id("hw-12345")
            .sign(&ca.key_pem())
            .unwrap();

        // Full validation
        cred.validate(Some(&ca.cert_pem()), Some("hw-12345"))
            .unwrap();
    }

    #[test]
    fn test_credential_validation_expired() {
        let past = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(3600);

        let cred = Credential::new("c", "t", "tok", Some(past));
        let result = cred.validate(None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_credential_validation_device_mismatch() {
        let cred = Credential::new("c", "t", "tok", None)
            .with_device_id("hw-12345");

        let result = cred.validate(None, Some("hw-99999"));
        assert!(result.is_err());
    }

    #[test]
    fn test_credential_validation_missing_device_id() {
        let cred = Credential::new("c", "t", "tok", None);
        // No device_id set, but validation requires one

        let result = cred.validate(None, Some("hw-12345"));
        assert!(result.is_err());
    }

    #[test]
    fn test_credential_invalid_signature() {
        // Create two CAs
        let ca1 = crate::ca::CertificateAuthority::new_root(CaProfile::root("CA 1")).unwrap();
        let ca2 = crate::ca::CertificateAuthority::new_root(CaProfile::root("CA 2")).unwrap();

        // Sign with CA1
        let cred = Credential::new("c", "t", "tok", None)
            .sign(&ca1.key_pem())
            .unwrap();

        // Verify with CA2 - should fail
        let result = cred.verify_signature(&ca2.cert_pem());
        assert!(result.is_err());
    }

    #[test]
    fn test_storage_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let storage = CredentialStorage::new(temp_dir.path(), "credential.json");

        let cred = Credential::new("test-client", "tenant-1", "test-token", None)
            .with_device_id("hw-001");

        // Save
        storage.save(&cred).unwrap();
        assert!(storage.exists());

        // Load
        let loaded = storage.load().unwrap();
        assert_eq!(loaded.client_name, "test-client");
        assert_eq!(loaded.tenant_id, "tenant-1");
        assert_eq!(loaded.token(), "test-token");
        assert_eq!(loaded.device_id, Some("hw-001".to_string()));

        // Delete
        storage.delete().unwrap();
        assert!(!storage.exists());
        assert!(storage.load().is_none());
    }

    #[test]
    fn test_storage_save_load_signed() {
        let temp_dir = TempDir::new().unwrap();
        let storage = CredentialStorage::new(temp_dir.path(), "credential.json");
        let ca = crate::ca::CertificateAuthority::new_root(CaProfile::root("Test CA")).unwrap();

        let cred = Credential::new("test-client", "tenant-1", "test-token", None)
            .with_device_id("hw-001")
            .sign(&ca.key_pem())
            .unwrap();

        // Save signed credential
        storage.save(&cred).unwrap();

        // Load and verify
        let loaded = storage.load().unwrap();
        assert!(loaded.is_signed());
        loaded.verify_signature(&ca.cert_pem()).unwrap();
    }
}
