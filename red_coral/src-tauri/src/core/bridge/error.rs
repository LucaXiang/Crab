//! Bridge error types

use thiserror::Error;

use super::super::tenant_manager::TenantError;

/// ClientBridge 错误类型
#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("Not initialized")]
    NotInitialized,

    #[error("Not authenticated")]
    NotAuthenticated,

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Already running in {0} mode")]
    AlreadyRunning(String),

    #[error("Tenant error: {0}")]
    Tenant(#[from] TenantError),

    #[error("Client error: {0}")]
    Client(#[from] crab_client::ClientError),

    #[error("Server error: {0}")]
    Server(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
