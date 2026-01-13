use crate::error::{CertError, Result};
use ::pem::Pem;
use ring::{rand as ring_rand, signature};
use rsa::pkcs1::DecodeRsaPublicKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use x509_parser::pem::parse_x509_pem;

pub fn to_rustls_certs(cert_pem: &str) -> Result<Vec<CertificateDer<'static>>> {
    let mut certs = Vec::new();
    let pems: Vec<Pem> = ::pem::parse_many(cert_pem)
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
    let pems: Vec<Pem> = ::pem::parse_many(key_pem)
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

fn decode_pem(pem_str: &str, expected_tag: &str) -> Result<Vec<u8>> {
    let pems: Vec<Pem> = ::pem::parse_many(pem_str)
        .map_err(|e| CertError::VerificationFailed(format!("PEM parse error: {}", e)))?;

    for p in pems {
        if p.tag() == expected_tag {
            return Ok(p.into_contents());
        }
    }

    Err(CertError::VerificationFailed(format!(
        "No {} found in PEM",
        expected_tag
    )))
}

/// Sign data using a private key (supports ECDSA P-256 and RSA)
pub fn sign(priv_key_pem: &str, data: &[u8]) -> Result<Vec<u8>> {
    let der = decode_pem(priv_key_pem, "PRIVATE KEY")?;

    // Try ECDSA P-256 first
    let rng = ring_rand::SystemRandom::new();
    if let Ok(key_pair) =
        signature::EcdsaKeyPair::from_pkcs8(&signature::ECDSA_P256_SHA256_ASN1_SIGNING, &der, &rng)
    {
        let sig = key_pair
            .sign(&rng, data)
            .map_err(|e| CertError::VerificationFailed(format!("Signing failed: {}", e)))?;
        return Ok(sig.as_ref().to_vec());
    }

    // Try RSA
    if let Ok(key_pair) = signature::RsaKeyPair::from_pkcs8(&der) {
        let mut sig = vec![0; key_pair.public().modulus_len()];
        key_pair
            .sign(&signature::RSA_PKCS1_SHA256, &rng, data, &mut sig)
            .map_err(|e| CertError::VerificationFailed(format!("Signing failed: {}", e)))?;
        return Ok(sig);
    }

    Err(CertError::VerificationFailed(
        "Unsupported or invalid private key format".into(),
    ))
}

/// Verify signature using a certificate (supports ECDSA P-256 and RSA)
pub fn verify(cert_pem: &str, data: &[u8], sig: &[u8]) -> Result<()> {
    // Parse Cert to get Public Key
    let (_, pem) = parse_x509_pem(cert_pem.as_bytes())
        .map_err(|e| CertError::VerificationFailed(format!("PEM parse error: {}", e)))?;
    let (_, x509) = x509_parser::parse_x509_certificate(&pem.contents)
        .map_err(|e| CertError::VerificationFailed(format!("X509 parse error: {}", e)))?;

    let spki = x509.tbs_certificate.subject_pki;
    let key_bytes = spki.subject_public_key.data;
    let oid = spki.algorithm.algorithm.to_id_string();

    let peer_public_key = if oid == "1.2.840.10045.2.1" {
        // ECDSA P-256
        signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_ASN1, key_bytes)
    } else if oid == "1.2.840.113549.1.1.1" {
        // RSA
        signature::UnparsedPublicKey::new(&signature::RSA_PKCS1_2048_8192_SHA256, key_bytes)
    } else {
        return Err(CertError::VerificationFailed(format!(
            "Unsupported algorithm OID: {}",
            oid
        )));
    };

    peer_public_key
        .verify(data, sig)
        .map_err(|_| CertError::VerificationFailed("Signature verification failed".into()))
}

/// Encrypt data using the Public Key from a certificate (RSA only)
pub fn encrypt(cert_pem: &str, data: &[u8]) -> Result<Vec<u8>> {
    // Parse Cert to get Public Key
    let (_, pem) = parse_x509_pem(cert_pem.as_bytes())
        .map_err(|e| CertError::VerificationFailed(format!("PEM parse error: {}", e)))?;
    let (_, x509) = x509_parser::parse_x509_certificate(&pem.contents)
        .map_err(|e| CertError::VerificationFailed(format!("X509 parse error: {}", e)))?;

    let spki = x509.tbs_certificate.subject_pki;
    let oid = spki.algorithm.algorithm.to_id_string();

    if oid != "1.2.840.113549.1.1.1" {
        return Err(CertError::VerificationFailed(
            "Encryption is only supported for RSA keys".into(),
        ));
    }

    // Parse RSA Public Key (PKCS#1)
    let public_key = RsaPublicKey::from_pkcs1_der(&spki.subject_public_key.data)
        .map_err(|e| CertError::VerificationFailed(format!("Invalid RSA public key: {}", e)))?;

    // Use PKCS1v15 padding
    let mut rng = rand::thread_rng();
    let enc_data = public_key
        .encrypt(&mut rng, Pkcs1v15Encrypt, data)
        .map_err(|e| CertError::VerificationFailed(format!("Encryption failed: {}", e)))?;

    Ok(enc_data)
}

/// Decrypt data using a Private Key (RSA only)
pub fn decrypt(priv_key_pem: &str, ciphertext: &[u8]) -> Result<Vec<u8>> {
    // Parse PKCS#8 Private Key
    let private_key = RsaPrivateKey::from_pkcs8_pem(priv_key_pem)
        .map_err(|e| CertError::VerificationFailed(format!("Invalid RSA private key: {}", e)))?;

    let data = private_key
        .decrypt(Pkcs1v15Encrypt, ciphertext)
        .map_err(|e| CertError::VerificationFailed(format!("Decryption failed: {}", e)))?;

    Ok(data)
}
