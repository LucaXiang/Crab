use crate::error::Result;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct CertMetadata {
    pub common_name: Option<String>,
    pub tenant_id: Option<String>,
    pub device_id: Option<String>,
    pub client_name: Option<String>,
    pub serial_number: String,
    pub fingerprint_sha256: String,
    pub not_after: time::OffsetDateTime,
}

impl CertMetadata {
    pub fn from_pem(pem: &str) -> Result<Self> {
        let (_, pem) = x509_parser::pem::parse_x509_pem(pem.as_bytes()).map_err(|e| {
            crate::error::CertError::VerificationFailed(format!("PEM parse error: {}", e))
        })?;

        Self::from_der(&pem.contents)
    }

    pub fn from_der(der: &[u8]) -> Result<Self> {
        // Calculate SHA256 fingerprint
        let mut hasher = Sha256::new();
        hasher.update(der);
        let fingerprint_sha256 = hex::encode(hasher.finalize());

        let (_, x509) = x509_parser::parse_x509_certificate(der).map_err(|e| {
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
        let mut client_name = None;

        // Serial Number (Hex)
        let serial_number = x509.tbs_certificate.serial.to_str_radix(16);

        // Not After
        let not_after = x509.validity().not_after.to_datetime();

        // Parse extensions
        for ext in x509.extensions() {
            let oid = &ext.oid;
            // Compare with our internal OIDs
            // OID_TENANT_ID: 1.3.6.1.4.1.99999.1
            // OID_DEVICE_ID: 1.3.6.1.4.1.99999.2
            // OID_CLIENT_NAME: 1.3.6.1.4.1.99999.5

            if oid.to_id_string() == "1.3.6.1.4.1.99999.1" {
                tenant_id = decode_der_utf8_string(ext.value);
            } else if oid.to_id_string() == "1.3.6.1.4.1.99999.2" {
                device_id = decode_der_utf8_string(ext.value);
            } else if oid.to_id_string() == "1.3.6.1.4.1.99999.5" {
                client_name = decode_der_utf8_string(ext.value);
            }
        }

        Ok(Self {
            common_name,
            tenant_id,
            device_id,
            client_name,
            serial_number,
            fingerprint_sha256,
            not_after,
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

fn decode_der_utf8_string(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }

    // Try parsing as DER UTF8String (Tag 0x0C)
    if bytes[0] == 0x0C {
        let mut idx = 1;
        if idx < bytes.len() {
            let len_byte = bytes[idx];
            idx += 1;

            let len = if len_byte & 0x80 == 0 {
                len_byte as usize
            } else {
                let num_bytes = (len_byte & 0x7F) as usize;
                if idx + num_bytes > bytes.len() {
                    // Not enough bytes for length
                    return None;
                }
                let mut l = 0usize;
                for i in 0..num_bytes {
                    l = (l << 8) | (bytes[idx + i] as usize);
                }
                idx += num_bytes;
                l
            };

            if let Some(Ok(s)) = bytes
                .get(idx..idx + len)
                .map(|slice| String::from_utf8(slice.to_vec()))
            {
                return Some(s);
            }
        }
    }

    // Return None if not a valid DER UTF8String
    None
}
