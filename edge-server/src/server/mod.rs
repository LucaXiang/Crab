pub mod auth;
mod config;
pub mod credential;
mod error;
pub mod middleware;
pub mod provisioning;
mod state;

use std::net::SocketAddr;

pub use auth::{CurrentUser, JwtConfig, JwtService};
pub use config::Config;
pub use error::Result;
pub use provisioning::ProvisioningService;
pub use state::ServerState;

use crate::routes::build_app;

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
            None => {
                let s = ServerState::initialize(&self.config).await;
                // If we initialized the state, we must start the background tasks
                s.start_background_tasks().await;
                s
            }
        };

        // Build fully configured app with all middleware, then apply state
        let app = build_app(&state).with_state(state.clone());
        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.http_port));

        // Loop to handle activation and potential re-activation
        loop {
            tracing::info!("Waiting for activation to start HTTPS server...");

            // 1. Wait for Activation
            state.wait_for_activation().await;
            tracing::info!("Activation signal received! Initializing HTTPS server...");

            // 2. Load mTLS Certificates
            let tls_config = match state.load_tls_config() {
                Ok(Some(config)) => config,
                Ok(None) => {
                    tracing::error!(
                        "❌ Server activated but certificates missing! Resetting activation state..."
                    );
                    if let Err(e) = state.deactivate_and_reset().await {
                        tracing::error!("Failed to reset state: {}", e);
                    }
                    continue;
                }
                Err(e) => {
                    tracing::error!(
                        "❌ Certificate chain broken: {}. Resetting activation state...",
                        e
                    );
                    if let Err(e) = state.deactivate_and_reset().await {
                        tracing::error!("Failed to reset state: {}", e);
                    }
                    continue;
                }
            };

            // 3. Start Services
            self.print_activation_banner(&state).await;
            tracing::info!("🔒 mTLS certificates loaded. Starting services...");

            // --- Unified Start of Services ---

            // A. Start TCP Message Bus Server
            let tcp_tls_config = tls_config.clone();
            let bus = state.message_bus().clone();
            tokio::spawn(async move {
                tracing::info!("Starting Message Bus TCP server...");
                if let Err(e) = bus.start_tcp_server(Some(tcp_tls_config)).await {
                    tracing::error!("Message bus TCP server error: {}", e);
                }
            });

            // B. Start HTTPS Server
            tracing::info!("Starting HTTPS server...");
            let rustls_config = axum_server::tls_rustls::RustlsConfig::from_config(tls_config);
            let handle = axum_server::Handle::new();
            let shutdown_future = shutdown_signal();
            let handle_clone = handle.clone();

            tokio::spawn(async move {
                shutdown_future.await;
                handle_clone.graceful_shutdown(Some(std::time::Duration::from_secs(10)));
            });

            tracing::info!("🚀 Starting HTTPS server on {}", addr);

            axum_server::bind_rustls(addr, rustls_config)
                .handle(handle)
                .serve(app.into_make_service())
                .await
                .map_err(|e| {
                    crate::server::error::ServerError::Internal(anyhow::anyhow!(
                        "Server error: {}",
                        e
                    ))
                })?;

            tracing::info!("✅ Server shutdown complete");
            return Ok(());
        }
    }

    async fn print_activation_banner(&self, state: &ServerState) {
        let activation = state
            .activation_service()
            .get_status()
            .await
            .unwrap_or_default();
        let tenant_id = activation
            .tenant_id
            .unwrap_or_else(|| "Unknown".to_string());
        let edge_id = activation.edge_id.unwrap_or_else(|| "Unknown".to_string());
        let cert_fingerprint = activation
            .cert_fingerprint
            .unwrap_or_else(|| "Unknown".to_string());
        let cert_expiry = activation
            .cert_expires_at
            .map(|d| d.to_rfc3339())
            .unwrap_or_else(|| "Unknown".to_string());

        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.http_port));

        println!("\n");
        println!("╔════════════════════════════════════════════════════════════════════════════╗");
        println!("║                   🦀 Crab Edge Server - Activated 🚀                       ║");
        println!("╠════════════════════════════════════════════════════════════════════════════╣");
        println!("║ 🏢 Tenant ID       : {:<53} ║", tenant_id);
        println!("║ 🆔 Edge ID         : {:<53} ║", edge_id);
        println!("╠════════════════════════════════════════════════════════════════════════════╣");
        println!("║ 🔒 Certificate     : {:<53} ║", cert_fingerprint);
        println!("║ 📅 Expires At      : {:<53} ║", cert_expiry);
        println!("╠════════════════════════════════════════════════════════════════════════════╣");
        println!("║ 🌐 HTTPS Listener  : https://{:<45} ║", addr);
        println!(
            "║ 📨 TCP Listener    : 0.0.0.0:{:<45} ║",
            self.config.message_tcp_port
        );
        println!("╚════════════════════════════════════════════════════════════════════════════╝");
        println!("\n");
    }
}

/// Graceful shutdown handler
///
/// Listens for SIGTERM (Kubernetes) and Ctrl+C signals
async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C signal, shutting down gracefully...");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM signal, shutting down gracefully...");
        },
    }
}
