use crate::error::Result;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct CertMetadata {
    pub common_name: Option<String>,
    pub tenant_id: Option<String>,
    pub device_id: Option<String>,
    pub hardware_id: Option<String>,
    pub serial_number: String,
    pub fingerprint_sha256: String,
}

impl CertMetadata {
    pub fn from_pem(pem: &str) -> Result<Self> {
        let (_, pem) = x509_parser::pem::parse_x509_pem(pem.as_bytes()).map_err(|e| {
            crate::error::CertError::VerificationFailed(format!("PEM parse error: {}", e))
        })?;

        // Calculate SHA256 fingerprint
        let mut hasher = Sha256::new();
        hasher.update(&pem.contents);
        let fingerprint_sha256 = hex::encode(hasher.finalize());

        let (_, x509) = x509_parser::parse_x509_certificate(&pem.contents).map_err(|e| {
            crate::error::CertError::VerificationFailed(format!("X509 parse error: {}", e))
        })?;

        let mut common_name = None;
        for rdn in x509.subject().iter_rdn() {
            for attr in rdn.iter() {
                if attr.attr_type() == &x509_parser::oid_registry::OID_X509_COMMON_NAME {
                    common_name = attr.as_str().ok().map(|s| s.to_string());
                }
            }
        }

        let mut tenant_id = None;
        let mut device_id = None;
        let mut hardware_id = None;

        // Serial Number (Hex)
        let serial_number = x509.tbs_certificate.serial.to_str_radix(16);

        // Parse extensions
        for ext in x509.extensions() {
            let oid = &ext.oid;
            // Compare with our internal OIDs
            // OID_TENANT_ID: 1.3.6.1.4.1.99999.1
            // OID_DEVICE_ID: 1.3.6.1.4.1.99999.2
            // OID_HARDWARE_ID: 1.3.6.1.4.1.99999.4

            if oid.to_id_string() == "1.3.6.1.4.1.99999.1" {
                tenant_id = String::from_utf8(ext.value.to_vec()).ok();
            } else if oid.to_id_string() == "1.3.6.1.4.1.99999.2" {
                device_id = String::from_utf8(ext.value.to_vec()).ok();
            } else if oid.to_id_string() == "1.3.6.1.4.1.99999.4" {
                hardware_id = String::from_utf8(ext.value.to_vec()).ok();
            }
        }

        Ok(Self {
            common_name,
            tenant_id,
            device_id,
            hardware_id,
            serial_number,
            fingerprint_sha256,
        })
    }

    /// Load metadata from a PEM certificate file
    pub fn from_pem_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let pem = fs::read_to_string(path).map_err(|e| {
            crate::error::CertError::VerificationFailed(format!("Failed to read cert file: {}", e))
        })?;
        Self::from_pem(&pem)
    }

    /// Verify if the fingerprint matches the expected SHA256 hex string (case-insensitive)
    pub fn verify_fingerprint(&self, expected_sha256_hex: &str) -> bool {
        self.fingerprint_sha256
            .eq_ignore_ascii_case(expected_sha256_hex)
    }
}
