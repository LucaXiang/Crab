//! Shared types for Crab framework
//!
//! Common types used across multiple crates including HTTP types,
//! error types, response structures, and utility types.

pub mod activation;
pub mod app_state;
pub mod client;
pub mod error;
pub mod message;
pub mod models;
pub mod order;
pub mod request;
pub mod types;
pub mod util;

// Re-exports
pub use axum::{Json, body};
pub use http;
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
