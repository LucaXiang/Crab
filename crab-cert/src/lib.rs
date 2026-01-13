mod adapter;
mod ca;
mod crypto;
mod error;
mod machine;
mod metadata;
mod profile;

pub use ca::CertificateAuthority;
pub use crypto::{decrypt, encrypt, sign, verify};
pub use error::{CertError, Result};
pub use machine::generate_hardware_id;
pub use metadata::CertMetadata;
pub use profile::{CaProfile, CertProfile, KeyType};
// We don't export adapter functions at top level to keep it clean, or maybe we do?
// User said "simple and intuitive".
// Maybe `CertificateAuthority` should have methods `to_rustls_config`?
// But `CertificateAuthority` is for CA.
// For client certs, we get (cert_pem, key_pem).
// So `adapter` functions are useful utilities.
pub use adapter::{
    to_identity_pem, to_rustls_certs, to_rustls_key, verify_client_cert, verify_server_cert,
};
