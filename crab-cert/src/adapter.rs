use crate::error::{CertError, Result as CertResult};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error as RustlsError, SignatureScheme};
use std::result::Result as StdResult;
use std::sync::Arc;

/// Combine cert and key into a single PEM string (Identity)
pub fn to_identity_pem(cert_pem: &str, key_pem: &str) -> String {
    format!("{}\n{}", cert_pem.trim(), key_pem.trim())
}

/// Load Root CA into a RootCertStore
pub fn load_root_store(ca_pem: &str) -> CertResult<rustls::RootCertStore> {
    let mut root_store = rustls::RootCertStore::empty();
    let mut reader = std::io::BufReader::new(ca_pem.as_bytes());
    for cert in rustls_pemfile::certs(&mut reader) {
        match cert {
            Ok(c) => {
                root_store.add(c).map_err(|e| {
                    CertError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                })?;
            }
            Err(e) => return Err(CertError::Io(e)),
        }
    }
    Ok(root_store)
}

/// Convert PEM string to Vec<CertificateDer>
pub fn to_rustls_certs(pem: &str) -> CertResult<Vec<CertificateDer<'static>>> {
    let mut reader = std::io::BufReader::new(pem.as_bytes());
    rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(CertError::Io)
}

/// A ServerCertVerifier that enforces CA signature validation but ignores hostname mismatches.
#[derive(Debug)]
pub struct SkipHostnameVerifier {
    verifier: Arc<rustls::client::WebPkiServerVerifier>,
}

impl SkipHostnameVerifier {
    pub fn new(root_store: rustls::RootCertStore) -> Self {
        let verifier = rustls::client::WebPkiServerVerifier::builder(Arc::new(root_store))
            .build()
            .unwrap();
        Self { verifier }
    }
}

impl ServerCertVerifier for SkipHostnameVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>, // Ignore the target server name (e.g., IP address)
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> StdResult<ServerCertVerified, RustlsError> {
        // We delegate to WebPkiServerVerifier but we trick it by providing a dummy server name
        // that matches what is in the certificate. This ensures the chain is valid and signed
        // by our trusted CA, but ignores whether we are connecting to "localhost" or "192.168.x.x".

        // 1. Parse the certificate to extract ANY valid name (SAN or CN)
        let cert = x509_parser::parse_x509_certificate(end_entity.as_ref())
            .map_err(|_| RustlsError::InvalidCertificate(rustls::CertificateError::BadEncoding))?
            .1;

        let mut valid_name = None;

        // Try to find a valid name in SANs
        if let Some(sans) = cert.subject_alternative_name().ok().flatten() {
            for entry in sans.value.general_names.iter() {
                match entry {
                    x509_parser::extensions::GeneralName::DNSName(name) => {
                        if let Ok(sn) = ServerName::try_from(*name) {
                            valid_name = Some(sn.to_owned());
                            break;
                        }
                    }
                    x509_parser::extensions::GeneralName::IPAddress(ip) => {
                        // Handle IP addresses in SAN
                        let ip_addr = match ip.len() {
                            4 => Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(
                                ip[0], ip[1], ip[2], ip[3],
                            ))),
                            16 => {
                                let octets: [u8; 16] = (*ip).try_into().unwrap();
                                Some(std::net::IpAddr::V6(std::net::Ipv6Addr::from(octets)))
                            }
                            _ => None,
                        };

                        if let Some(ip) = ip_addr {
                            let sn = ServerName::from(ip);
                            valid_name = Some(sn.to_owned());
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }

        // Fallback to CN
        if valid_name.is_none() {
            for rdn in cert.subject().iter_rdn() {
                for attr in rdn.iter() {
                    if attr.attr_type() == &x509_parser::oid_registry::OID_X509_COMMON_NAME
                        && let Some(sn) = attr
                            .as_str()
                            .ok()
                            .and_then(|s| ServerName::try_from(s).ok())
                    {
                        valid_name = Some(sn.to_owned());
                    }
                }
                if valid_name.is_some() {
                    break;
                }
            }
        }

        // If we still can't find a name, we can't verify it against WebPKI rules anyway
        let name_to_verify = valid_name.ok_or_else(|| {
            RustlsError::General(
                "No valid hostname found in certificate for verification".to_string(),
            )
        })?;

        // 2. Delegate to WebPkiServerVerifier with the name EXTRACTED FROM THE CERT itself
        self.verifier.verify_server_cert(
            end_entity,
            intermediates,
            &name_to_verify,
            ocsp_response,
            now,
        )
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> StdResult<HandshakeSignatureValid, RustlsError> {
        self.verifier.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> StdResult<HandshakeSignatureValid, RustlsError> {
        self.verifier.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.verifier.supported_verify_schemes()
    }
}

/// Verify a server certificate against a CA root, ignoring hostname mismatch
pub fn verify_server_cert(cert_pem: &str, ca_pem: &str) -> CertResult<()> {
    let root_store = load_root_store(ca_pem)?;
    let verifier = SkipHostnameVerifier::new(root_store);

    let certs = to_rustls_certs(cert_pem)?;
    if certs.is_empty() {
        return Err(CertError::VerificationFailed(
            "No server certificate found".into(),
        ));
    }

    // Dummy server name, will be ignored/replaced by SkipHostnameVerifier
    let server_name = ServerName::try_from("example.com").unwrap();

    verifier
        .verify_server_cert(
            &certs[0],
            &certs[1..],
            &server_name,
            &[] as &[u8],
            UnixTime::now(),
        )
        .map_err(|e| CertError::VerificationFailed(e.to_string()))
        .map(|_| ())?;

    Ok(())
}

/// Verify a client certificate against a CA root
pub fn verify_client_cert(cert_pem: &str, ca_pem: &str) -> CertResult<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CaProfile, CertProfile, CertificateAuthority, KeyType};

    #[test]
    fn test_skip_hostname_verifier() {
        // Install crypto provider for tests
        let _ = rustls::crypto::ring::default_provider().install_default();

        // 1. Create Root CA
        let root_profile = CaProfile {
            common_name: "Test Root CA".to_string(),
            organization: "Test Org".to_string(),
            validity_days: 1,
            key_type: KeyType::P256,
            ..Default::default()
        };
        let root_ca = CertificateAuthority::new_root(root_profile).unwrap();

        let mut root_store = rustls::RootCertStore::empty();
        for cert in to_rustls_certs(root_ca.cert_pem()).unwrap() {
            root_store.add(cert).unwrap();
        }

        // 2. Create Server Cert with a specific name "valid.com"
        // Corrected arguments: common_name, sans, tenant_id, device_id
        let server_profile =
            CertProfile::new_server("valid.com", vec![], None, "device-123".to_string());
        let (server_cert_pem, _) = root_ca.issue_cert(&server_profile).unwrap();
        let server_certs = to_rustls_certs(&server_cert_pem).unwrap();

        // 3. Setup Verifier
        let verifier = SkipHostnameVerifier::new(root_store);

        // 4. Verify against a WRONG name "wrong.com" -> Should PASS
        let wrong_name = ServerName::try_from("wrong.com").unwrap();

        let result =
            verifier.verify_server_cert(&server_certs[0], &[], &wrong_name, &[], UnixTime::now());

        assert!(result.is_ok(), "Should pass even if hostname mismatch");

        // 5. Verify invalid signature -> Should FAIL
        // Create another random CA
        let other_ca = CertificateAuthority::new_root(CaProfile::default()).unwrap();
        let (fake_cert_pem, _) = other_ca.issue_cert(&server_profile).unwrap();
        let fake_certs = to_rustls_certs(&fake_cert_pem).unwrap();

        let result_fake =
            verifier.verify_server_cert(&fake_certs[0], &[], &wrong_name, &[], UnixTime::now());

        assert!(result_fake.is_err(), "Should fail if signature is invalid");
    }
}
