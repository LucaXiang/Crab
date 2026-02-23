//! Shared types for Crab framework
//!
//! Common types used across multiple crates including HTTP types,
//! error types, response structures, and utility types.

pub mod activation;
pub mod app_state;
pub mod client;
pub mod cloud;
pub mod error;
pub mod message;
pub mod models;
pub mod order;
pub mod request;
pub mod types;
pub mod util;

/// Git commit hash (short), embedded at compile time by build.rs
pub const GIT_HASH: &str = env!("GIT_HASH");

/// Default auth server URL (production, public HTTPS via Caddy)
pub const DEFAULT_AUTH_SERVER_URL: &str = "https://auth.redcoral.app";

/// Default cloud sync URL (production, mTLS direct connection on port 8443)
pub const DEFAULT_CLOUD_SYNC_URL: &str = "https://sync.redcoral.app:8443";

// Re-exports
pub use serde::{Deserialize, Serialize};

// Message bus re-exports (for convenient access)
pub use message::{BusMessage, EventType};

// Unified error system re-exports
pub use error::{ApiResponse as UnifiedApiResponse, AppError, AppResult, ErrorCategory, ErrorCode};

// App state re-exports
pub use app_state::{
    ActivationRequiredReason, CertificateHealth, ClockDirection, ComponentsHealth, DatabaseHealth,
    DeviceInfo, HealthLevel, HealthStatus, NetworkHealth, SubscriptionBlockedInfo,
    SubscriptionHealth,
};
