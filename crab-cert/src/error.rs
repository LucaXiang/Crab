use thiserror::Error;

#[derive(Error, Debug)]
pub enum CertError {
    #[error("RCGen error: {0}")]
    Rcgen(#[from] rcgen::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid certificate")]
    InvalidCertificate,
    #[error("Invalid key")]
    InvalidKey,
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    #[error("TLS error: {0}")]
    Tls(#[from] rustls::Error),
}

pub type Result<T> = std::result::Result<T, CertError>;
