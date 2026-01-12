pub mod auth;
mod config;
mod error;
pub mod middleware;
mod state;

use std::net::SocketAddr;

pub use auth::{CurrentUser, JwtConfig, JwtService};
pub use config::Config;
pub use error::Result;
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
            None => ServerState::initialize(&self.config).await,
        };

        // Build fully configured app with all middleware, then apply state
        let app = build_app(&state).with_state(state);

        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.http_port));
        tracing::info!("🚀 Starting server on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .unwrap();

        tracing::info!("✅ Server shutdown complete");

        Ok(())
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
