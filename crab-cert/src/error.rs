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
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
    #[error("TLS error: {0}")]
    Tls(String),

    // ── P12 specific errors ──
    #[error("Invalid P12/PFX file format: {0}")]
    P12InvalidFormat(String),
    #[error("Wrong P12 password or corrupted file: {0}")]
    P12WrongPassword(String),
    #[error("P12 contains no private key for signing")]
    P12MissingPrivateKey,
    #[error("P12 contains no certificate")]
    P12MissingCertificate,
    #[error("Certificate chain signature verification failed: {0}")]
    P12ChainVerifyFailed(String),
    #[error("Certificate root CA not recognized by AEAT: {0}")]
    P12UntrustedCa(String),
}

pub type Result<T> = std::result::Result<T, CertError>;
