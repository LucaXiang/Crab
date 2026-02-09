mod api;
mod config;
mod db;
mod state;

use config::Config;
use sqlx::postgres::PgPoolOptions;
use state::{AppState, AuthStorage};
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "crab_auth=info".into()),
        )
        .init();

    if let Err(e) = run().await {
        tracing::error!(error = %e, "crab-auth failed to start");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let config = Config::from_env();

    // Connect to PostgreSQL
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .idle_timeout(std::time::Duration::from_secs(600))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .connect(&config.database_url)
        .await?;

    info!("Connected to PostgreSQL");

    // Run migrations (only activations + p12_certificates — tenants/subscriptions managed externally)
    sqlx::migrate!("./migrations").run(&pool).await?;

    let auth_storage = AuthStorage::new(config.auth_storage_path);

    // Verify Root CA is accessible
    auth_storage.get_or_create_root_ca()?;
    info!("Root CA ready");

    // Initialize AWS SDK
    let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    let state = Arc::new(AppState {
        db: pool,
        auth_storage,
        s3: s3_client,
        s3_bucket: config.s3_bucket,
        kms_key_id: config.kms_key_id,
    });

    let app = api::router(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(addr = %addr, "crab-auth started");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("crab-auth stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => info!("Received SIGINT"),
            _ = sigterm.recv() => info!("Received SIGTERM"),
        }
    }

    #[cfg(not(unix))]
    {
        ctrl_c.await.ok();
        info!("Received SIGINT");
    }
}
