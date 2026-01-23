//! Error types for the printer library

use thiserror::Error;

/// Printer error types
#[derive(Debug, Error)]
pub enum PrintError {
    /// Network connection error
    #[error("Connection failed: {0}")]
    Connection(String),

    /// IO error during printing
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Printer is offline or unreachable
    #[error("Printer offline: {0}")]
    Offline(String),

    /// Timeout waiting for printer
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Invalid printer configuration
    #[error("Invalid config: {0}")]
    InvalidConfig(String),

    /// Windows-specific printing error
    #[cfg(windows)]
    #[error("Windows printer error: {0}")]
    WindowsPrinter(String),
}

/// Result type for printer operations
pub type PrintResult<T> = Result<T, PrintError>;
