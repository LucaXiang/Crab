//! crab-cloud — Cloud tenant management center
//!
//! Long-running service that:
//! - Receives synced data from edge-servers (mTLS + SignedBinding)
//! - Manages tenant data mirrors (products, orders, reports)
//! - Future: Stripe integration, remote access for internet users

mod api;
mod auth;
mod config;
mod db;
mod email;
mod state;
mod stripe;

use config::Config;
use state::AppState;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    // Load .env file
    let _ = dotenvy::dotenv();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "crab_cloud=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env();

    tracing::info!("Starting crab-cloud (env: {})", config.environment);

    // Initialize application state
    let state = AppState::new(&config).await?;

    // Build router
    let app = api::create_router(state);

    // Start HTTP server
    let addr = format!("0.0.0.0:{}", config.http_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("crab-cloud listening on {addr} (HTTP)");

    axum::serve(listener, app).await?;

    Ok(())
}
