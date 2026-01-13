use crate::crypto::verify;
use crate::error::{CertError, Result};

/// Hardcoded Root CA Certificate (PEM)
/// This is the trust anchor for the entire system.
pub const ROOT_CA_PEM: &str = include_str!("../certs/root_ca.pem");

/// Verify a CA certificate against the hardcoded Root CA.
/// This checks the signature of the `ca_cert` using the Root CA's public key.
pub fn verify_ca_signature(ca_cert_pem: &str) -> Result<()> {
    // 1. Parse CA Cert PEM to get DER
    let (_, pem) = x509_parser::pem::parse_x509_pem(ca_cert_pem.as_bytes())
        .map_err(|e| CertError::VerificationFailed(format!("PEM parse error: {}", e)))?;

    // 2. Extract TBS bytes manually from DER
    // The Certificate is a SEQUENCE of [TBS, Alg, Sig].
    // We parse the outer sequence, then extract the first element as raw bytes.
    let tbs_bytes = extract_tbs_bytes(&pem.contents)?;

    // 3. Extract Signature
    // We can use x509-parser to easily get the signature value
    let (_, cert) = x509_parser::parse_x509_certificate(&pem.contents)
        .map_err(|e| CertError::VerificationFailed(format!("X509 parse error: {}", e)))?;

    let signature = cert.signature_value.as_ref(); // returns &[u8]

    // 4. Verify
    verify(ROOT_CA_PEM, tbs_bytes, signature)?;

    Ok(())
}

fn extract_tbs_bytes(der: &[u8]) -> Result<&[u8]> {
    // Helper to read header and return (header_len, content_len)
    fn read_header(data: &[u8]) -> Result<(usize, usize)> {
        if data.len() < 2 {
            return Err(CertError::VerificationFailed("DER too short".into()));
        }
        let mut idx = 1;
        let len_byte = data[idx];
        idx += 1;

        let len = if len_byte & 0x80 == 0 {
            len_byte as usize
        } else {
            let num_bytes = (len_byte & 0x7F) as usize;
            if num_bytes > 4 {
                return Err(CertError::VerificationFailed("DER length too large".into()));
            }
            if data.len() < idx + num_bytes {
                return Err(CertError::VerificationFailed("DER truncated".into()));
            }
            let mut l = 0usize;
            for i in 0..num_bytes {
                l = (l << 8) | (data[idx + i] as usize);
            }
            idx += num_bytes;
            l
        };
        Ok((idx, len))
    }

    // 1. Parse Outer Sequence
    if der.is_empty() || der[0] != 0x30 {
        return Err(CertError::VerificationFailed(
            "Not a certificate sequence".into(),
        ));
    }
    let (outer_hdr_len, _) = read_header(der)?;
    let content = &der[outer_hdr_len..];

    // 2. First element is TBS (Sequence)
    if content.is_empty() {
        return Err(CertError::VerificationFailed(
            "Empty certificate content".into(),
        ));
    }
    // TBS should start here
    // Verify it looks like a sequence (TBS is a Sequence)
    // Note: Sometimes it might be explicit tagged? Usually X.509 TBS is a Sequence.
    // We don't strictly enforce 0x30 but usually it is.

    let (tbs_hdr_len, tbs_content_len) = read_header(content)?;
    let tbs_total_len = tbs_hdr_len + tbs_content_len;

    if content.len() < tbs_total_len {
        return Err(CertError::VerificationFailed("TBS truncated".into()));
    }

    Ok(&content[0..tbs_total_len])
}

/// Verify a certificate chain against the hardcoded Root CA.
/// This is a convenience wrapper around `adapter::verify_server_cert` using the hardcoded Root.
pub fn verify_chain_against_root(chain_pem: &str) -> Result<()> {
    crate::adapter::verify_server_cert(chain_pem, ROOT_CA_PEM)
}
