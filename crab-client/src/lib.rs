//! Crab Client - HTTP client for Edge Server
//!
//! Provides network-based HTTP calls to the Edge Server API.

pub mod config;
pub mod error;
pub mod http;
pub mod message;

pub use config::ClientConfig;
pub use error::{ClientError, ClientResult};
pub use http::HttpClient;

// Re-export shared types for convenience
pub use shared::client::{ApiResponse, CurrentUserResponse, LoginResponse, UserInfo};

// Message types and clients
pub use message::{BusMessage, EventType, MessageClient, MessageError};
