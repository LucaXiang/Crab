mod adapter;
mod ca;
mod credential;
mod crypto;
mod error;
mod machine;
mod metadata;
#[cfg(feature = "p12-openssl")]
mod p12;
mod profile;
mod server;
pub mod signer;
pub mod trust;

pub use adapter::{SkipHostnameVerifier, to_identity_pem, verify_client_cert, verify_server_cert};
pub use ca::CertificateAuthority;
pub use credential::{Credential, CredentialStorage};
pub use crypto::{decrypt, encrypt, sign, to_rustls_certs, to_rustls_key, verify};
pub use error::{CertError, Result};
pub use machine::{generate_hardware_id, generate_quick_hardware_id};
pub use metadata::CertMetadata;
#[cfg(feature = "p12-openssl")]
pub use p12::{P12CertInfo, parse_p12};
pub use profile::{CaProfile, CertProfile, KeyType};
pub use server::{CertService, CertStorage};
pub use trust::{get_or_create_root_ca, verify_ca_signature, verify_chain_against_root};

/// Write a file with restrictive permissions (0o600 on Unix) suitable for secrets.
///
/// On Unix, the file is created with mode 0o600 (owner read/write only).
/// On non-Unix platforms, falls back to `std::fs::write`.
pub fn write_secret_file(
    path: impl AsRef<std::path::Path>,
    contents: impl AsRef<[u8]>,
) -> std::io::Result<()> {
    _write_secret_file(path.as_ref(), contents.as_ref())
}

#[cfg(unix)]
fn _write_secret_file(path: &std::path::Path, contents: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(contents)
}

#[cfg(not(unix))]
fn _write_secret_file(path: &std::path::Path, contents: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, contents)
}
