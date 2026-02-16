//! Builder pattern for CrabClient construction.
//!
//! This module provides a type-safe builder API for creating clients
//! in different modes (Remote/Local).

use std::path::PathBuf;

use crate::error::ClientError;
use crate::types::{Disconnected, Remote, StateMarker};

use super::CrabClient;

// ============================================================================
// Remote Builder
// ============================================================================

/// Builder for `CrabClient<Remote>`.
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
#[derive(Debug, Clone)]
pub struct RemoteClientBuilder {
    auth_server_url: Option<String>,
    edge_server_url: Option<String>,
    cert_path: Option<PathBuf>,
    client_name: Option<String>,
}

impl Default for RemoteClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RemoteClientBuilder {
    /// Creates a new builder for a remote client.
    pub fn new() -> Self {
        Self {
            auth_server_url: None,
            edge_server_url: None,
            cert_path: None,
            client_name: None,
        }
    }

    /// Sets the Auth Server URL.
    ///
    /// This is the URL of the authentication server that issues certificates.
    pub fn auth_server(mut self, url: impl Into<String>) -> Self {
        self.auth_server_url = Some(url.into());
        self
    }

    /// Sets the Edge Server URL for HTTPS API.
    ///
    /// This is the URL of the Edge Server's HTTPS endpoint (e.g., "https://192.168.1.100:3000").
    /// Used for mTLS HTTP API calls (login, me, etc.).
    pub fn edge_server(mut self, url: impl Into<String>) -> Self {
        self.edge_server_url = Some(url.into());
        self
    }

    /// Sets the certificate storage path.
    ///
    /// Certificates will be stored in `{cert_path}/{client_name}/`.
    pub fn cert_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.cert_path = Some(path.into());
        self
    }

    /// Sets the client name.
    ///
    /// This name identifies the client and is used in certificate requests.
    pub fn client_name(mut self, name: impl Into<String>) -> Self {
        self.client_name = Some(name.into());
        self
    }

    /// Builds the remote client.
    ///
    /// # Errors
    ///
    /// Returns `ClientError::Config` if any required field is missing.
    pub fn build(self) -> Result<CrabClient<Remote, Disconnected>, ClientError> {
        let auth_server_url = self
            .auth_server_url
            .ok_or_else(|| ClientError::Config("auth_server is required".into()))?;

        let cert_path = self
            .cert_path
            .ok_or_else(|| ClientError::Config("cert_path is required".into()))?;

        let client_name = self
            .client_name
            .ok_or_else(|| ClientError::Config("client_name is required".into()))?;

        let edge_server_url = self.edge_server_url;

        // Create HTTP client for Auth Server communication
        let http_client = crate::client::http::NetworkHttpClient::new(&auth_server_url)
            .map_err(|e| ClientError::Config(format!("Failed to create HTTP client: {}", e)))?;

        // Create certificate manager
        let cert_manager = crate::CertManager::new(&cert_path, &client_name);

        Ok(CrabClient {
            marker: StateMarker::new(),
            http: Some(http_client),
            message: None,
            cert_manager: Some(cert_manager),
            edge_http: None, // Created during setup/reconnect
            #[cfg(feature = "in-process")]
            oneshot_http: None,
            #[cfg(feature = "in-process")]
            memory_message: None,
            session: Default::default(),
            config: ClientConfig {
                auth_server_url: Some(auth_server_url),
                edge_url: edge_server_url,
                cert_path: Some(cert_path),
                client_name: Some(client_name),
            },
        })
    }
}

// ============================================================================
// Local Builder (in-process feature)
// ============================================================================

#[cfg(feature = "in-process")]
use crate::types::Local;
#[cfg(feature = "in-process")]
use axum::Router;
#[cfg(feature = "in-process")]
use shared::message::BusMessage;
#[cfg(feature = "in-process")]
use tokio::sync::broadcast;

/// Builder for `CrabClient<Local>`.
///
/// Local clients use Tower oneshot for HTTP calls and broadcast channels
/// for message bus communication. Zero network overhead.
///
/// # Example
///
/// ```ignore
/// use crab_client::CrabClient;
/// use axum::Router;
///
/// // 从 ServerState 获取组件
/// let router = state.https_service().router().unwrap();
/// let client_tx = state.message_bus().sender_to_server().clone();
/// let server_tx = state.message_bus().sender().clone();
///
/// let client = CrabClient::local()
///     .with_router(router)
///     .with_message_channels(client_tx, server_tx)
///     .build()
///     .expect("Failed to build client");
/// ```
#[cfg(feature = "in-process")]
#[derive(Debug)]
pub struct LocalClientBuilder {
    router: Option<Router>,
    /// 客户端 → 服务器通道
    client_tx: Option<broadcast::Sender<BusMessage>>,
    /// 服务器 → 客户端通道
    server_tx: Option<broadcast::Sender<BusMessage>>,
}

#[cfg(feature = "in-process")]
impl Default for LocalClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "in-process")]
impl LocalClientBuilder {
    /// Creates a new builder for a local client.
    pub fn new() -> Self {
        Self {
            router: None,
            client_tx: None,
            server_tx: None,
        }
    }

    /// Sets the Axum Router for HTTP oneshot calls.
    ///
    /// The router should already have state attached via `with_state()`.
    pub fn with_router(mut self, router: Router) -> Self {
        self.router = Some(router);
        self
    }

    /// Sets the message channels for bidirectional communication.
    ///
    /// # Arguments
    /// * `client_tx` - 客户端→服务器通道 (MessageBus.sender_to_server())
    /// * `server_tx` - 服务器→客户端通道 (MessageBus.sender())
    pub fn with_message_channels(
        mut self,
        client_tx: broadcast::Sender<BusMessage>,
        server_tx: broadcast::Sender<BusMessage>,
    ) -> Self {
        self.client_tx = Some(client_tx);
        self.server_tx = Some(server_tx);
        self
    }

    /// Builds the local client.
    ///
    /// # Errors
    ///
    /// Returns `ClientError::Config` if router or message channels are missing.
    pub fn build(self) -> Result<CrabClient<Local, Disconnected>, ClientError> {
        let router = self
            .router
            .ok_or_else(|| ClientError::Config("router is required for local mode".into()))?;

        let client_tx = self.client_tx.ok_or_else(|| {
            ClientError::Config("message channels are required for local mode".into())
        })?;

        let server_tx = self.server_tx.ok_or_else(|| {
            ClientError::Config("message channels are required for local mode".into())
        })?;

        // Create oneshot HTTP client
        let oneshot_http = super::http_oneshot::OneshotHttpClient::new(router);

        // Create in-memory message client with bidirectional channels
        let memory_message = super::message::InMemoryMessageClient::new(client_tx, server_tx);

        Ok(CrabClient {
            marker: StateMarker::new(),
            http: None,
            message: None,
            cert_manager: None,
            edge_http: None, // Local mode doesn't use edge_http
            oneshot_http: Some(oneshot_http),
            memory_message: Some(memory_message),
            session: Default::default(),
            config: ClientConfig {
                auth_server_url: None,
                edge_url: None,
                cert_path: None,
                client_name: None,
            },
        })
    }
}

// ============================================================================
// Client Configuration
// ============================================================================

/// Internal client configuration.
#[derive(Debug, Clone, Default)]
pub struct ClientConfig {
    /// Auth Server URL (Remote mode only).
    pub auth_server_url: Option<String>,
    /// Edge Server URL for HTTPS API (e.g., "https://127.0.0.1:3000").
    pub edge_url: Option<String>,
    /// Certificate storage path (Remote mode only).
    pub cert_path: Option<PathBuf>,
    /// Client name (Remote mode only).
    pub client_name: Option<String>,
}
