//! Shared types for Crab framework
//!
//! Common types used across multiple crates including HTTP types,
//! error types, response structures, and utility types.

pub mod activation;
pub mod client;
pub mod error;
pub mod intent;
pub mod message;
pub mod models;
pub mod order;
pub mod request;
pub mod response;
pub mod types;

// Re-exports
pub use axum::{Json, body};
pub use http;
pub use serde::{Deserialize, Serialize};

// Message bus re-exports (for convenient access)
pub use message::{BusMessage, EventType};

// Intent re-exports (for DataIntent dispatch pattern)
pub use intent::{CrudAction, DataIntent, DataResult, QueryRequest, PaginatedResponse};
