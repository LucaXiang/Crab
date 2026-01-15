//! Unified client for Edge Server
//!
//! Provides a unified interface for both network-based HTTP calls and
//! in-process oneshot calls, similar to SurrealDB's design.
//!
//! # Examples
//!
//! ## HTTP client (network calls)
//!
//! ```ignore
//! use edge_server::CrabClient;
//!
//! let client = CrabClient::new::<Http>("http://localhost:8080");
//! let user = client.me().await?;
//! ```
//!
//! ## Oneshot client (in-process)
//!
//! ```ignore
//! use edge_server::{CrabClient, ServerState};
//!
//! let state = ServerState::initialize(&config).await;
//! state.start_background_tasks().await;
//! let client = CrabClient::new(state);
//! let user = client.me().await?;
//! ```

use async_trait::async_trait;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::sync::broadcast;

pub mod http;
pub mod message;
pub mod oneshot;

pub use http::Http;
pub use message::MessageClient;
pub use oneshot::Oneshot;

use crate::common::AppError;
use crate::message::BusMessage;

// Re-export shared response types
pub use shared::client::{ApiResponse, CurrentUserResponse, LoginResponse, UserInfo};

// ========== CrabClient Trait ==========

/// Unified client trait for Edge Server
///
/// This trait provides a consistent interface for all client implementations,
/// whether they use network calls or oneshot in-process calls.
#[async_trait]
pub trait CrabClient: Send + Sync + Clone + 'static {
    /// Make a GET request
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, AppError>;

    /// Make a POST request with JSON body
    async fn post<T: DeserializeOwned, B: Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, AppError>;

    /// Make a POST request without body
    async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> Result<T, AppError>;

    /// Login with username and password
    async fn login(&mut self, username: &str, password: &str) -> Result<LoginResponse, AppError>;

    /// Get current user information
    async fn me(&self) -> Result<CurrentUserResponse, AppError>;

    /// Logout
    async fn logout(&mut self) -> Result<(), AppError>;

    /// Subscribe to message bus notifications
    ///
    /// Returns a receiver that can be used to receive messages.
    /// Each call creates a new subscription.
    fn subscribe(&self) -> Result<broadcast::Receiver<BusMessage>, AppError>;
}

// ========== Main Client Struct ==========

/// Unified client wrapper for Edge Server
///
/// Uses a generic backend to support both HTTP and oneshot connections.
///
/// # Examples
///
/// ## HTTP client (network)
///
/// ```ignore
/// let client = ClientInner::<Http>::new("http://localhost:8080");
/// let login = client.login("admin", "admin123").await?;
/// ```
///
/// ## Oneshot client (in-process)
///
/// ```ignore
/// let state = ServerState::initialize(&config).await;
/// state.start_background_tasks().await;
/// let client = ClientInner::<Oneshot>::new(state);
/// let user = client.me().await?;
/// ```
#[derive(Debug, Clone)]
pub struct ClientInner<C: CrabClient> {
    inner: C,
}

impl<C: CrabClient> ClientInner<C> {
    /// Create a new client with the given backend
    pub fn new(inner: C) -> Self {
        Self { inner }
    }

    /// Get reference to inner client
    pub fn inner(&self) -> &C {
        &self.inner
    }
}

// Delegate trait implementations to inner client
#[async_trait]
impl<C: CrabClient> CrabClient for ClientInner<C> {
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, AppError> {
        self.inner.get(path).await
    }

    async fn post<T: DeserializeOwned, B: Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, AppError> {
        self.inner.post(path, body).await
    }

    async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> Result<T, AppError> {
        self.inner.post_empty(path).await
    }

    async fn login(&mut self, username: &str, password: &str) -> Result<LoginResponse, AppError> {
        let inner = &mut self.inner;
        inner.login(username, password).await
    }

    async fn me(&self) -> Result<CurrentUserResponse, AppError> {
        self.inner.me().await
    }

    async fn logout(&mut self) -> Result<(), AppError> {
        let inner = &mut self.inner;
        inner.logout().await
    }

    fn subscribe(&self) -> Result<broadcast::Receiver<BusMessage>, AppError> {
        self.inner.subscribe()
    }
}
