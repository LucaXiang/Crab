//! Common client definitions and shared functionality.
//!
//! This module defines the core `CrabClient` struct and functionality
//! shared across all modes and states.

use crate::cert::CertManager;
use crate::error::{ClientError, ClientResult};
#[cfg(feature = "in-process")]
use crate::types::Local;
use crate::types::{
    Authenticated, ClientMode, ClientState, ClientStatus, Disconnected, Remote, SessionData,
    StateMarker,
};

#[cfg(feature = "in-process")]
use super::builder::LocalClientBuilder;
use super::builder::{ClientConfig, RemoteClientBuilder};
use super::http::NetworkHttpClient;
#[cfg(feature = "in-process")]
use super::http_oneshot::OneshotHttpClient;
#[cfg(feature = "in-process")]
use super::message::InMemoryMessageClient;
use super::message::NetworkMessageClient;

// ============================================================================
// Core CrabClient Definition
// ============================================================================

/// A type-safe HTTP and message client for Crab services.
///
/// `CrabClient` uses the typestate pattern to ensure correct usage at compile time:
/// - The `M` parameter specifies the client mode (`Remote` or `Local`)
/// - The `S` parameter specifies the current state (`Disconnected`, `Connected`, or `Authenticated`)
///
/// # Modes
///
/// - **Remote**: Connects to a remote Edge Server using mTLS. Requires certificates.
/// - **Local**: In-process communication via Tower oneshot. Requires "in-process" feature.
///
/// # States
///
/// - **Disconnected**: Initial state. Can call `setup()`/`connect_with_credentials()` (Remote) or `connect()` (Local).
/// - **Connected**: Connected to server but not logged in. Can call `login()`.
/// - **Authenticated**: Logged in and ready to make requests. Can call `request()`, `me()`, etc.
///
/// # Example (Remote Mode)
///
/// ```no_run
/// use crab_client::CrabClient;
///
/// # async fn example() -> Result<(), crab_client::ClientError> {
/// let client = CrabClient::remote()
///     .auth_server("https://auth.example.com")
///     .cert_path("./certs")
///     .client_name("pos-01")
///     .build()?;
///
/// let client = client.setup("tenant", "pass", "edge:8081").await?;
/// let client = client.login("employee", "pass").await?;
/// # Ok(())
/// # }
/// ```
///
/// # Example (Local Mode - requires "in-process" feature)
///
/// ```ignore
/// use crab_client::CrabClient;
/// use axum::Router;
/// use tokio::sync::broadcast;
///
/// let router: Router = build_app().with_state(state);
/// let (sender, _) = broadcast::channel(1024);
///
/// let client = CrabClient::local()
///     .with_router(router)
///     .with_message_channels(client_tx, server_tx)
///     .build()?;
///
/// let client = client.connect().await?;
/// let client = client.login("employee", "pass").await?;
/// ```
#[derive(Debug)]
pub struct CrabClient<M: ClientMode, S: ClientState = Disconnected> {
    #[allow(dead_code)] // Used for typestate pattern at compile time
    pub(crate) marker: StateMarker<M, S>,
    // Remote mode clients
    pub(crate) http: Option<NetworkHttpClient>,
    pub(crate) message: Option<NetworkMessageClient>,
    pub(crate) cert_manager: Option<CertManager>,
    /// mTLS HTTP client for Edge Server HTTPS API (health, me, etc.)
    pub(crate) edge_http: Option<reqwest::Client>,
    // Local mode clients (in-process feature)
    #[cfg(feature = "in-process")]
    pub(crate) oneshot_http: Option<OneshotHttpClient>,
    #[cfg(feature = "in-process")]
    pub(crate) memory_message: Option<InMemoryMessageClient>,
    // Common fields
    pub(crate) session: SessionData,
    pub(crate) config: ClientConfig,
}

// ============================================================================
// Builder Entry Points
// ============================================================================

impl CrabClient<Remote, Disconnected> {
    /// Creates a builder for a remote client.
    ///
    /// Remote clients connect to Edge Servers using mTLS certificates.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crab_client::CrabClient;
    ///
    /// let client = CrabClient::remote()
    ///     .auth_server("https://auth.example.com")
    ///     .cert_path("./certs")
    ///     .client_name("pos-01")
    ///     .build()
    ///     .expect("Failed to build client");
    /// ```
    pub fn remote() -> RemoteClientBuilder {
        RemoteClientBuilder::new()
    }
}

#[cfg(feature = "in-process")]
impl CrabClient<Local, Disconnected> {
    /// Creates a builder for a local (in-process) client.
    ///
    /// Local clients use Tower oneshot for HTTP calls and broadcast channels
    /// for message bus communication. Zero network overhead.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use crab_client::CrabClient;
    /// use axum::Router;
    /// use tokio::sync::broadcast;
    ///
    /// let router: Router = build_app().with_state(state);
    /// let (sender, _) = broadcast::channel(1024);
    ///
    /// let client = CrabClient::local()
    ///     .with_router(router)
    ///     .with_message_channels(client_tx, server_tx)
    ///     .build()
    ///     .expect("Failed to build client");
    /// ```
    pub fn local() -> LocalClientBuilder {
        LocalClientBuilder::new()
    }
}

// ============================================================================
// Common Methods (Available in All States)
// ============================================================================

impl<M: ClientMode, S: ClientState> CrabClient<M, S> {
    /// Returns the current employee token, if available.
    pub fn token(&self) -> Option<&str> {
        self.session.token()
    }

    /// Checks if the client is authenticated (has employee token).
    pub fn is_authenticated(&self) -> bool {
        self.session.employee_token.is_some()
    }

    /// Returns the client configuration.
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }
}

// ============================================================================
// Remote-specific Common Methods
// ============================================================================

impl<S: ClientState> CrabClient<Remote, S> {
    /// Checks if the client is connected to the message server.
    pub fn is_connected(&self) -> bool {
        self.message
            .as_ref()
            .map(|m| m.is_connected())
            .unwrap_or(false)
    }

    /// Returns the client status.
    pub fn status(&self) -> ClientStatus {
        ClientStatus {
            has_tenant_credential: self
                .cert_manager
                .as_ref()
                .map(|cm| cm.has_credential())
                .unwrap_or(false),
            has_certificates: self
                .cert_manager
                .as_ref()
                .map(|cm| cm.has_local_certificates())
                .unwrap_or(false),
            is_connected: self.is_connected(),
            is_authenticated: self.is_authenticated(),
        }
    }

    /// Checks if cached certificates are available.
    ///
    /// If this returns `true`, you can use `connect_with_credentials()` instead of `setup()`.
    pub fn has_cached_credentials(&self) -> bool {
        self.cert_manager
            .as_ref()
            .map(|cm| cm.has_local_certificates())
            .unwrap_or(false)
    }

    /// Returns the client name.
    pub fn client_name(&self) -> Option<&str> {
        self.config.client_name.as_deref()
    }

    /// Returns a reference to the underlying message client.
    ///
    /// Use this to send RPC requests directly. RPC only requires mTLS connection,
    /// no employee login needed.
    pub fn message_client(&self) -> Option<&NetworkMessageClient> {
        self.message.as_ref()
    }

    /// Transforms the client to a new state (internal use only).
    pub(crate) fn transition<NewS: ClientState>(self) -> CrabClient<Remote, NewS> {
        CrabClient {
            marker: StateMarker::new(),
            http: self.http,
            message: self.message,
            cert_manager: self.cert_manager,
            edge_http: self.edge_http,
            #[cfg(feature = "in-process")]
            oneshot_http: None,
            #[cfg(feature = "in-process")]
            memory_message: None,
            session: self.session,
            config: self.config,
        }
    }

    /// Returns the mTLS HTTP client for Edge Server HTTPS API.
    ///
    /// Use this to make HTTPS requests to the Edge Server (e.g., /health, /api/auth/me).
    /// This client is configured with mTLS and skips hostname verification.
    ///
    /// Returns `None` if the client has not been connected yet.
    pub fn edge_http_client(&self) -> Option<&reqwest::Client> {
        self.edge_http.as_ref()
    }
}

// ============================================================================
// Local-specific Common Methods (in-process feature)
// ============================================================================

#[cfg(feature = "in-process")]
impl<S: ClientState> CrabClient<Local, S> {
    /// Checks if the client is connected (always true for in-process).
    pub fn is_connected(&self) -> bool {
        self.memory_message
            .as_ref()
            .map(|m| m.is_connected())
            .unwrap_or(false)
    }

    /// Returns the client status.
    pub fn status(&self) -> ClientStatus {
        ClientStatus {
            has_tenant_credential: false, // Local mode doesn't use tenant credentials
            has_certificates: false,      // Local mode doesn't use certificates
            is_connected: self.is_connected(),
            is_authenticated: self.is_authenticated(),
        }
    }

    /// Transforms the client to a new state (internal use only).
    pub(crate) fn transition<NewS: ClientState>(self) -> CrabClient<Local, NewS> {
        CrabClient {
            marker: StateMarker::new(),
            http: None,
            message: None,
            cert_manager: None,
            edge_http: None, // Local mode doesn't use edge_http
            oneshot_http: self.oneshot_http,
            memory_message: self.memory_message,
            session: self.session,
            config: self.config,
        }
    }
}

// ============================================================================
// Authenticated State Methods (Common to Both Modes)
// ============================================================================

impl<M: ClientMode> CrabClient<M, Authenticated> {
    /// Returns the current user information.
    ///
    /// This is available only after successful login.
    pub fn me(&self) -> Option<&shared::client::UserInfo> {
        self.session.user()
    }

    /// Returns the current user information, or an error if not available.
    pub fn current_user(&self) -> ClientResult<&shared::client::UserInfo> {
        self.session
            .user()
            .ok_or_else(|| ClientError::InvalidState("No user info available".into()))
    }
}
