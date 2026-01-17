// crab-client/src/cert/mod.rs
// 证书和凭证管理模块

pub mod manager;

// Re-export Credential and CredentialStorage from crab-cert
pub use crab_cert::{Credential, CredentialStorage};
pub use manager::{CertError, CertManager};
