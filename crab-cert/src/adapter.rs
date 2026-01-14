use crate::crypto::to_rustls_certs;
use crate::error::{CertError, Result as CertResult};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error as RustlsError, SignatureScheme};
use std::result::Result as StdResult;
use std::sync::Arc;

/// Combine cert and key into a single PEM buffer suitable for reqwest::Identity::from_pem
pub fn to_identity_pem(cert_pem: &str, key_pem: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(cert_pem.as_bytes());
    if !cert_pem.ends_with('\n') {
        buf.push(b'\n');
    }
    buf.extend_from_slice(key_pem.as_bytes());
    if !key_pem.ends_with('\n') {
        buf.push(b'\n');
    }
    buf
}

fn load_root_store(ca_pem: &str) -> CertResult<rustls::RootCertStore> {
    let mut root_store = rustls::RootCertStore::empty();
    let ca_certs = to_rustls_certs(ca_pem)?;
    for cert in ca_certs {
        root_store
            .add(cert)
            .map_err(|e| CertError::VerificationFailed(e.to_string()))?;
    }
    Ok(root_store)
}

/// A ServerCertVerifier that enforces CA signature validation but ignores hostname mismatches.
/// This is crucial for local mTLS where IPs are dynamic and DNS is unreliable.
#[derive(Debug)]
pub struct SkipHostnameVerifier {
    verifier: Arc<rustls::client::WebPkiServerVerifier>,
}

impl SkipHostnameVerifier {
    pub fn new(root_store: rustls::RootCertStore) -> Self {
        let verifier = rustls::client::WebPkiServerVerifier::builder(Arc::new(root_store))
            .build()
            .unwrap(); // Should not fail with valid root store
        Self { verifier }
    }
}

impl ServerCertVerifier for SkipHostnameVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>, // Ignore this
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> StdResult<ServerCertVerified, RustlsError> {
        // We delegate to WebPkiServerVerifier but we trick it by providing a dummy server name
        // that matches what we don't care about, OR we just check the signature chain manually.
        // Actually, WebPkiServerVerifier *always* checks hostname.
        // So we cannot simply delegate if we want to skip hostname check.
        // However, rustls::client::WebPkiServerVerifier does NOT expose a method to only check chain.

        // Wait, if we want to skip hostname verification but keep chain verification,
        // we should use `verify_tls12_signature` and `verify_tls13_signature`?
        // No, those are for handshake signatures.

        // Correct approach: Use `WebPkiServerVerifier::verify_server_cert` but with a parsed
        // SAN from the cert itself? No, that defeats the purpose if the cert has a random name.

        // We have to implement the chain verification logic ourselves or find a way to use
        // `rustls::client::verify_server_cert_signed_by_trust_anchor`.

        // Ideally we use `rustls-platform-verifier` or similar, but here we want strict CA pinning.
        // Let's look at how `rustls` does it.

        // Ideally we should use `rustls::client::WebPkiServerVerifier` but since it enforces hostname,
        // we might need to construct a `ServerName` that matches the certificate's CN/SAN.
        // But the caller passed in `_server_name` which is the target IP/Domain.

        // HACK: We can try to parse the `end_entity` certificate, extract the first DNSName or IPAddress,
        // and pass THAT to the inner verifier.
        // This effectively makes the hostname check always pass (as long as the cert is valid for *some* name).

        let cert = x509_parser::parse_x509_certificate(end_entity.as_ref())
            .map_err(|_| RustlsError::InvalidCertificate(rustls::CertificateError::BadEncoding))?
            .1;

        // Try to find a valid name in the cert to satisfy the verifier
        let mut valid_name = None;

        // Check SANs
        if let Some(sans) = cert.subject_alternative_name().ok().flatten() {
            for entry in sans.value.general_names.iter() {
                match entry {
                    x509_parser::extensions::GeneralName::DNSName(name) => {
                        if let Ok(sn) = ServerName::try_from(*name) {
                            valid_name = Some(sn);
                            break;
                        }
                    }
                    x509_parser::extensions::GeneralName::IPAddress(ip) => {
                        let ip_addr = match ip.len() {
                            4 => std::net::IpAddr::V4(std::net::Ipv4Addr::new(
                                ip[0], ip[1], ip[2], ip[3],
                            )),
                            16 => {
                                let b: [u8; 16] = (*ip).try_into().unwrap();
                                std::net::IpAddr::V6(std::net::Ipv6Addr::from(b))
                            }
                            _ => continue,
                        };
                        let sn = ServerName::from(ip_addr);
                        valid_name = Some(sn.to_owned());
                        break;
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
                        && let Ok(s) = attr.as_str()
                        && let Ok(sn) = ServerName::try_from(s)
                    {
                        valid_name = Some(sn);
                    }
                }
            }
        }

        let name_to_verify = valid_name.unwrap_or_else(|| {
            // If we really can't find a name, we might as well use the one passed in,
            // it will likely fail hostname check but we have no choice.
            _server_name.to_owned()
        });

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
