use crate::CertificateAuthority;
use crate::error::{CertError, Result};
use std::path::Path;

/// Load or create the Root CA.
///
/// This is used by the Auth Server to manage the system trust anchor.
/// If the Root CA exists in the given directory, it is loaded.
/// Otherwise, a new Root CA is generated and saved.
pub fn get_or_create_root_ca(dir: &Path) -> Result<CertificateAuthority> {
    // Try to load
    let cert_path = dir.join("root_ca.crt");
    let key_path = dir.join("root_ca.key");

    if cert_path.exists() && key_path.exists() {
        return CertificateAuthority::load_from_file(&cert_path, &key_path);
    }

    // Generate new
    let profile = crate::CaProfile::root("Crab Root CA");
    let ca = CertificateAuthority::new_root(profile)?;
    ca.save(dir, "root_ca")?;

    Ok(ca)
}

/// Verify a CA certificate against a Root CA.
///
/// This checks the signature of the `ca_cert` using the Root CA's public key.
pub fn verify_ca_signature(ca_cert_pem: &str, root_ca_pem: &str) -> Result<()> {
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
    crate::crypto::verify(root_ca_pem, tbs_bytes, signature)?;

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

/// Verify a certificate chain against a Root CA.
/// Walks the chain: cert[0] signed by cert[1], ..., cert[N-1] signed by root.
/// Uses signature verification only, no hostname checking.
pub fn verify_chain_against_root(chain_pem: &str, root_ca_pem: &str) -> Result<()> {
    let pems: Vec<::pem::Pem> = ::pem::parse_many(chain_pem)
        .map_err(|e| CertError::VerificationFailed(format!("PEM parse error: {}", e)))?;

    let cert_pems: Vec<String> = pems
        .into_iter()
        .filter(|p| p.tag() == "CERTIFICATE")
        .map(|p| ::pem::encode(&p))
        .collect();

    if cert_pems.is_empty() {
        return Err(CertError::VerificationFailed(
            "No certificates found in chain".into(),
        ));
    }

    // Verify each cert against its issuer; last cert verified against root
    for i in 0..cert_pems.len() {
        let issuer_pem = if i + 1 < cert_pems.len() {
            &cert_pems[i + 1]
        } else {
            root_ca_pem
        };
        verify_ca_signature(&cert_pems[i], issuer_pem)?;
    }

    Ok(())
}
