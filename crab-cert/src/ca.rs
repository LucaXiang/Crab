use crate::error::{CertError, Result};
use crate::profile::{self, CaProfile, CertProfile};
use rand::thread_rng;
use rcgen::{CertificateParams, DistinguishedName, DnType, Issuer, KeyPair};
use rsa::RsaPrivateKey;
use rsa::pkcs8::EncodePrivateKey;
use std::fs;
use std::path::Path;
use x509_parser::pem::parse_x509_pem;

pub struct CertificateAuthority {
    pub(crate) params: CertificateParams,
    pub(crate) key_pair: KeyPair,
    pub(crate) cert_pem: String,
}

impl CertificateAuthority {
    pub fn new_root(profile: CaProfile) -> Result<Self> {
        let params = profile::create_ca_params(&profile);

        let key_pair = generate_key_pair(profile.key_type)?;
        let cert = params.self_signed(&key_pair).unwrap();

        Ok(Self {
            params,
            key_pair,
            cert_pem: cert.pem(),
        })
    }

    pub fn load(cert_pem: &str, key_pem: &str) -> Result<Self> {
        let key_pair =
            KeyPair::from_pem(key_pem).map_err(|e| CertError::VerificationFailed(e.to_string()))?;

        // We need to reconstruct params from the cert PEM for signing
        let (_, pem) = parse_x509_pem(cert_pem.as_bytes())
            .map_err(|e| CertError::VerificationFailed(e.to_string()))?;
        let (_, x509) = x509_parser::parse_x509_certificate(&pem.contents)
            .map_err(|e| CertError::VerificationFailed(e.to_string()))?;

        let mut params = CertificateParams::new(vec![]).unwrap();

        // Reconstruct DN
        let mut dn = DistinguishedName::new();
        for rdn in x509.subject().iter_rdn() {
            for attr in rdn.iter() {
                let oid = attr.attr_type();
                let val = attr.as_str().unwrap_or_default().to_string();

                if oid == &x509_parser::oid_registry::OID_X509_COMMON_NAME {
                    dn.push(DnType::CommonName, val);
                } else if oid == &x509_parser::oid_registry::OID_X509_ORGANIZATION_NAME {
                    dn.push(DnType::OrganizationName, val);
                } else if oid == &x509_parser::oid_registry::OID_X509_ORGANIZATIONAL_UNIT {
                    dn.push(DnType::OrganizationalUnitName, val);
                } else if oid == &x509_parser::oid_registry::OID_X509_COUNTRY_NAME {
                    dn.push(DnType::CountryName, val);
                }
            }
        }
        params.distinguished_name = dn;

        // Reconstruct CA status (simplified)
        if x509.is_ca() {
            params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        } else {
            params.is_ca = rcgen::IsCa::NoCa;
        }

        Ok(Self {
            params,
            key_pair,
            cert_pem: cert_pem.to_string(),
        })
    }

    /// Load a Certificate Authority from certificate and key files
    pub fn load_from_file<P: AsRef<Path>>(cert_path: P, key_path: P) -> Result<Self> {
        let cert_pem = fs::read_to_string(cert_path).map_err(|e| {
            CertError::VerificationFailed(format!("Failed to read cert file: {}", e))
        })?;
        let key_pem = fs::read_to_string(key_path).map_err(|e| {
            CertError::VerificationFailed(format!("Failed to read key file: {}", e))
        })?;
        Self::load(&cert_pem, &key_pem)
    }

    /// Save the Certificate Authority's certificate and key to files
    pub fn save<P: AsRef<Path>>(&self, dir: P, name: &str) -> Result<()> {
        let dir = dir.as_ref();
        if !dir.exists() {
            fs::create_dir_all(dir).map_err(|e| {
                CertError::VerificationFailed(format!("Failed to create directory: {}", e))
            })?;
        }

        let cert_path = dir.join(format!("{}.crt", name));
        let key_path = dir.join(format!("{}.key", name));

        fs::write(&cert_path, &self.cert_pem).map_err(|e| {
            CertError::VerificationFailed(format!("Failed to write cert file: {}", e))
        })?;

        fs::write(&key_path, self.key_pem()).map_err(|e| {
            CertError::VerificationFailed(format!("Failed to write key file: {}", e))
        })?;

        Ok(())
    }

    pub fn new_intermediate(profile: CaProfile, parent: &CertificateAuthority) -> Result<Self> {
        let params = profile::create_ca_params(&profile);
        let key_pair = generate_key_pair(profile.key_type)?;

        let parent_issuer = Issuer::new(parent.params.clone(), &parent.key_pair);
        let cert = params
            .signed_by(&key_pair, &parent_issuer)
            .map_err(|e| CertError::VerificationFailed(e.to_string()))?;

        Ok(Self {
            params,
            key_pair,
            cert_pem: cert.pem(),
        })
    }

    pub fn issue_cert(&self, profile: &CertProfile) -> Result<(String, String)> {
        let params = profile::create_cert_params(profile);
        let key_pair = generate_key_pair(profile.key_type)?;

        // Create Issuer from self
        let issuer = Issuer::new(self.params.clone(), &self.key_pair);

        let cert = params
            .signed_by(&key_pair, &issuer)
            .map_err(|e| CertError::VerificationFailed(e.to_string()))?;

        Ok((cert.pem(), key_pair.serialize_pem()))
    }

    /// Get the CA certificate PEM
    pub fn cert_pem(&self) -> &str {
        &self.cert_pem
    }

    /// Get the CA private key PEM
    pub fn key_pem(&self) -> String {
        self.key_pair.serialize_pem()
    }
}

fn generate_key_pair(key_type: profile::KeyType) -> Result<KeyPair> {
    match key_type {
        profile::KeyType::P256 => {
            KeyPair::generate().map_err(|e| CertError::VerificationFailed(e.to_string()))
        }
        profile::KeyType::Rsa2048 => {
            let mut rng = thread_rng();
            let private_key = RsaPrivateKey::new(&mut rng, 2048)
                .map_err(|e| CertError::VerificationFailed(format!("RSA gen error: {}", e)))?;
            let pem = private_key
                .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
                .map_err(|e| CertError::VerificationFailed(format!("RSA PEM error: {}", e)))?;
            KeyPair::from_pem(&pem).map_err(|e| CertError::VerificationFailed(e.to_string()))
        }
        profile::KeyType::Rsa4096 => {
            let mut rng = thread_rng();
            let private_key = RsaPrivateKey::new(&mut rng, 4096)
                .map_err(|e| CertError::VerificationFailed(format!("RSA gen error: {}", e)))?;
            let pem = private_key
                .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
                .map_err(|e| CertError::VerificationFailed(format!("RSA PEM error: {}", e)))?;
            KeyPair::from_pem(&pem).map_err(|e| CertError::VerificationFailed(e.to_string()))
        }
    }
}
