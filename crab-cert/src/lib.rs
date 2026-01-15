mod adapter;
mod ca;
mod crypto;
mod error;
mod machine;
mod metadata;
mod profile;
pub mod signer; // Export signer module
pub mod trust; // Export trust module

pub use adapter::{SkipHostnameVerifier, to_identity_pem, verify_client_cert, verify_server_cert};
pub use ca::CertificateAuthority;
pub use crypto::{decrypt, encrypt, sign, to_rustls_certs, to_rustls_key, verify}; // Export helpers
pub use error::{CertError, Result};
pub use machine::generate_hardware_id;
pub use metadata::CertMetadata;
pub use profile::{CaProfile, CertProfile, KeyType};
pub use trust::{get_or_create_root_ca, verify_ca_signature, verify_chain_against_root};
