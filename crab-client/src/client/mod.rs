//! Client module - unified client implementation.
//!
//! This module provides the `CrabClient` type with typestate-based API
//! for both Remote and Local modes.

// Core modules
mod builder;
mod common;
pub mod http;
#[cfg(feature = "in-process")]
pub mod http_oneshot;
mod local;
pub mod message;
mod remote;

// Re-export main types
pub use common::CrabClient;
pub use http::{HttpClient, NetworkHttpClient};
#[cfg(feature = "in-process")]
pub use http_oneshot::OneshotHttpClient;
pub use message::{InMemoryMessageClient, NetworkMessageClient};

// Re-export message config from parent module
pub use crate::message::MessageClientConfig;
