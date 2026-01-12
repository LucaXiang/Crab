//! Shared types for Crab framework
//!
//! Common types used across multiple crates including HTTP types,
//! error types, response structures, and utility types.

pub mod response;
pub mod error;
pub mod request;
pub mod types;
pub mod client;
pub mod message;

// Re-exports
pub use http;
pub use axum::{body, Json};
pub use serde::{Deserialize, Serialize};

// Message bus re-exports (for convenient access)
pub use message::{BusMessage, EventType};
