//! Crab Client - Unified client interface for Crab services.
//!
//! This crate provides a type-safe client for connecting to Edge Servers,
//! supporting both Remote (mTLS) and Local (in-process) modes.
//!
//! # Modes
//!
//! - **Remote**: Connects to a remote Edge Server using mTLS certificates.
//!   Requires setup with tenant credentials to download certificates.
//! - **Local** (requires "in-process" feature): In-process communication via
//!   Tower oneshot and broadcast channels. Zero network overhead.
//!
//! # Example (Remote Mode)
//!
//! ```no_run
//! use crab_client::CrabClient;
//!
//! # async fn example() -> Result<(), crab_client::ClientError> {
//! // Build client
//! let client = CrabClient::remote()
//!     .auth_server("https://auth.example.com")
//!     .cert_path("./certs")
//!     .client_name("pos-01")
//!     .build()?;
//!
//! // First-time setup (downloads certificates)
//! let client = client.setup("tenant", "pass", "edge:8081").await?;
//!
//! // Or reconnect using cached certificates
//! // let client = client.reconnect("edge:8081").await?;
//!
//! // Employee login
//! let client = client.login("cashier", "1234").await?;
//!
//! // Make requests
//! // let response = client.request(&msg).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Example (Local Mode - requires "in-process" feature)
//!
//! ```ignore
//! use crab_client::CrabClient;
//! use axum::Router;
//! use tokio::sync::broadcast;
//!
//! // Get router and message sender from ServerState
//! let router: Router = build_app().with_state(state);
//! let (sender, _) = broadcast::channel(1024);
//!
//! let client = CrabClient::local()
//!     .with_router(router)
//!     .with_message_sender(sender)
//!     .build()?;
//!
//! let client = client.connect().await?;
//! let client = client.login("waiter", "1234").await?;
//!
//! // Make HTTP requests (via Tower oneshot, no network)
//! let orders: Vec<Order> = client.get("/api/orders").await?;
//!
//! // Make RPC requests (via broadcast channel, no network)
//! let response = client.request(&msg).await?;
//! ```

// Core modules
mod cert;
mod client;
pub mod error;
pub mod message;
pub mod types;

// Re-export certificate types
pub use cert::{CertError, CertManager, Credential, CredentialStorage};

// Re-export client types
#[cfg(feature = "in-process")]
pub use client::OneshotHttpClient;
pub use client::{
    CrabClient, HttpClient, InMemoryMessageClient, MessageClientConfig, NetworkHttpClient,
    NetworkMessageClient,
};

// Re-export type markers
pub use types::{
    Authenticated, ClientMode, ClientState, ClientStatus, Connected, Disconnected, Local, Remote,
    SessionData,
};

// Re-export error types
pub use error::{ClientError, ClientResult, MessageError, MessageResult};

// Re-export message types
pub use message::{BusMessage, EventType};

// Re-export shared types
pub use shared::client::{ApiResponse, CurrentUserResponse, LoginRequest, LoginResponse, UserInfo};
