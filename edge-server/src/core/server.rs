//! Server Implementation
//!
//! HTTP 服务器启动和管理

use crate::core::{Config, Result, ServerState};
use axum_server::tls_rustls::RustlsConfig;

/// HTTP Server
pub struct Server {
    config: Config,
    state: Option<ServerState>,
}

impl Server {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            state: None,
        }
    }

    /// Create server with existing state (for sharing with oneshot)
    pub fn with_state(config: Config, state: ServerState) -> Self {
        Self {
            config,
            state: Some(state),
        }
    }

    pub async fn run(&self) -> Result<()> {
        // Create application state if not provided
        let state = match &self.state {
            Some(s) => s.clone(),
            None => ServerState::initialize(&self.config).await,
        };

        // Start background tasks
        state.start_background_tasks().await;

        #[allow(clippy::never_loop)]
        loop {
            // Wait for activation (includes self-check)
            // This blocks until:
            // 1. Server is activated (credential exists)
            // 2. Self-check passes (cert chain, hardware binding, credential signature, expiration)
            state.wait_for_activation().await;

            // Load TLS Config
            // If this fails after self-check passed, something is seriously wrong
            // (e.g., key file corrupted after self-check but before load)
            let tls_config = match state.load_tls_config() {
                Ok(Some(cfg)) => cfg,
                Ok(None) => {
                    // Certificates don't exist - enter unbound state
                    tracing::error!("❌ TLS certificates not found after activation!");
                    state.enter_unbound_state().await;
                    continue;
                }
                Err(e) => {
                    // Failed to load/parse certificates - enter unbound state
                    tracing::error!("❌ Failed to load TLS config: {}. Entering unbound state.", e);
                    state.enter_unbound_state().await;
                    continue;
                }
            };

            let rustls_config = RustlsConfig::from_config(tls_config.clone());

            // Start Message Bus TCP Server (mTLS)
            let message_bus_service = state.message_bus.clone();
            let tcp_tls_config = tls_config.clone();
            tokio::spawn(async move {
                if let Err(e) = message_bus_service.start_tcp_server(tcp_tls_config).await {
                    tracing::error!("Message Bus TCP server failed: {}", e);
                }
            });

            state.print_activated_banner_content().await;

            let addr = std::net::SocketAddr::from(([0, 0, 0, 0], self.config.http_port));
            tracing::info!("🦀 Crab Edge Server starting on {}", addr);

            // Start HTTPS service
            // Use the existing https service instance from state
            let shutdown = async {
                let _ = tokio::signal::ctrl_c().await;
                tracing::info!("Shutting down...");
            };

            state
                .https
                .start_server(rustls_config, shutdown)
                .await
                .map_err(|e| crate::core::ServerError::Internal(e.into()))?;

            // If server stops, we break the loop to exit the process
            // TODO: Handle soft restart (reset) without exiting process
            break;
        }

        Ok(())
    }
}
