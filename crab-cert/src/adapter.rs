use crate::error::{CertError, Result};
use rustls::client::danger::ServerCertVerifier;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use std::sync::Arc;

pub fn to_rustls_certs(cert_pem: &str) -> Result<Vec<CertificateDer<'static>>> {
    let mut certs = Vec::new();
    // Use x509-parser's PEM parser or just simple splitting?
    // x509-parser parse_x509_pem returns one cert.
    // We might have a chain.

    // Simple approach using pem crate
    let pems = pem::parse_many(cert_pem)
        .map_err(|e| CertError::VerificationFailed(format!("PEM parse error: {}", e)))?;

    for p in pems {
        if p.tag() == "CERTIFICATE" {
            certs.push(CertificateDer::from(p.into_contents()));
        }
    }

    if certs.is_empty() {
        return Err(CertError::VerificationFailed(
            "No certificates found in PEM".into(),
        ));
    }

    Ok(certs)
}

pub fn to_rustls_key(key_pem: &str) -> Result<PrivateKeyDer<'static>> {
    let pems = pem::parse_many(key_pem)
        .map_err(|e| CertError::VerificationFailed(format!("PEM parse error: {}", e)))?;

    for p in pems {
        if p.tag() == "PRIVATE KEY" {
            // PKCS#8
            return Ok(PrivateKeyDer::Pkcs8(p.contents().to_vec().into()));
        } else if p.tag() == "RSA PRIVATE KEY" {
            // PKCS#1
            return Ok(PrivateKeyDer::Pkcs1(p.contents().to_vec().into()));
        } else if p.tag() == "EC PRIVATE KEY" {
            // SEC1
            return Ok(PrivateKeyDer::Sec1(p.contents().to_vec().into()));
        }
    }

    Err(CertError::VerificationFailed(
        "No supported private key found in PEM".into(),
    ))
}

/// Combine cert and key into a single PEM buffer suitable for reqwest::Identity::from_pem
pub fn to_identity_pem(cert_pem: &str, key_pem: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(cert_pem.as_bytes());
    buf.push(b'\n');
    buf.extend_from_slice(key_pem.as_bytes());
    buf
}

fn load_root_store(ca_pem: &str) -> Result<rustls::RootCertStore> {
    let mut root_store = rustls::RootCertStore::empty();
    let ca_certs = to_rustls_certs(ca_pem)?;
    for cert in ca_certs {
        root_store
            .add(cert)
            .map_err(|e| CertError::VerificationFailed(e.to_string()))?;
    }
    Ok(root_store)
}

/// Verify a server certificate against a CA root
pub fn verify_server_cert(cert_pem: &str, ca_pem: &str, domain: &str) -> Result<()> {
    let root_store = load_root_store(ca_pem)?;
    let verifier = rustls::client::WebPkiServerVerifier::builder(Arc::new(root_store))
        .build()
        .map_err(|e| CertError::VerificationFailed(e.to_string()))?;

    let certs = to_rustls_certs(cert_pem)?;
    if certs.is_empty() {
        return Err(CertError::VerificationFailed(
            "No server certificate found".into(),
        ));
    }

    let server_name = ServerName::try_from(domain)
        .map_err(|_| CertError::VerificationFailed("Invalid domain name".into()))?;

    verifier
        .verify_server_cert(
            &certs[0],
            &certs[1..],
            &server_name,
            &[] as &[u8],
            UnixTime::now(),
        )
        .map_err(|e| CertError::VerificationFailed(e.to_string()))?;

    Ok(())
}

/// Verify a client certificate against a CA root
pub fn verify_client_cert(cert_pem: &str, ca_pem: &str) -> Result<()> {
    let root_store = load_root_store(ca_pem)?;
    let verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(root_store))
        .build()
        .map_err(|e| CertError::VerificationFailed(e.to_string()))?;

    let certs = to_rustls_certs(cert_pem)?;
    if certs.is_empty() {
        return Err(CertError::VerificationFailed(
            "No client certificate found".into(),
        ));
    }

    verifier
        .verify_client_cert(&certs[0], &certs[1..], UnixTime::now())
        .map_err(|e| CertError::VerificationFailed(e.to_string()))?;

    Ok(())
}
