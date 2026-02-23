//! crab-cloud — Cloud tenant management center
//!
//! Long-running service that:
//! - Receives synced data from edge-servers (mTLS + SignedBinding)
//! - Manages tenant data mirrors (products, orders, reports)
//! - Provides tenant management API (JWT authenticated)
//! - Relays commands from tenants to edge-servers

mod api;
mod auth;
mod config;
mod db;
mod email;
pub mod error;
mod live;
mod services;
mod state;
mod stripe;
pub mod util;

use config::Config;
use state::AppState;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    // Install rustls crypto provider (required before any TLS operations)
    // SAFETY: Called once at process start; `install_default` is idempotent (returns Err if already installed)
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // Load .env file
    let _ = dotenvy::dotenv();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "crab_cloud=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env()?;

    tracing::info!("Starting crab-cloud (env: {})", config.environment);

    // Initialize application state
    let state = AppState::new(&config).await?;

    // Build routers
    let public_app = api::public_router(state.clone());
    let edge_app = api::edge_router(state.clone());

    // Start HTTP server (public)
    let http_addr = format!("0.0.0.0:{}", config.http_port);
    let http_listener = tokio::net::TcpListener::bind(&http_addr).await?;
    tracing::info!("crab-cloud HTTP listening on {http_addr}");

    let http_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(http_listener, public_app).await {
            tracing::error!("HTTP server error: {e}");
        }
    });

    // Start mTLS server (edge-only)
    let mtls_addr = std::net::SocketAddr::from(([0, 0, 0, 0], config.mtls_port));
    let mtls_handle = match build_mtls_config(&config) {
        Ok(tls_config) => {
            tracing::info!("crab-cloud mTLS listening on {mtls_addr}");
            Some(tokio::spawn(async move {
                if let Err(e) = axum_server::bind_rustls(mtls_addr, tls_config)
                    .serve(edge_app.into_make_service())
                    .await
                {
                    tracing::error!("mTLS server error: {e}");
                }
            }))
        }
        Err(e) => {
            tracing::warn!(
                "mTLS server disabled: {e}. Edge sync will not be available. \
                 Set SERVER_CERT_PATH, SERVER_KEY_PATH, ROOT_CA_PATH to enable."
            );
            None
        }
    };

    // Periodic rate limiter cleanup (every 5 minutes)
    let rate_limiter = state.rate_limiter.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            rate_limiter.cleanup().await;
        }
    });

    // Periodic order detail cleanup (every hour, delete details older than 30 days)
    let cleanup_pool = state.pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            let cutoff = shared::util::now_millis() - 30 * 24 * 3600 * 1000;
            match sqlx::query("DELETE FROM cloud_order_details WHERE synced_at < $1")
                .bind(cutoff)
                .execute(&cleanup_pool)
                .await
            {
                Ok(result) => {
                    if result.rows_affected() > 0 {
                        tracing::info!(
                            deleted = result.rows_affected(),
                            "Cleaned up old order details (>30 days)"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("Order detail cleanup failed: {e}");
                }
            }
        }
    });

    // Periodic pending_rpcs cleanup (every 30s, remove entries older than 60s)
    let pending_rpcs = state.edges.pending_rpcs.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let cutoff = shared::util::now_millis() - 60_000;
            let stale_keys: Vec<String> = pending_rpcs
                .iter()
                .filter(|entry| entry.value().0 < cutoff)
                .map(|entry| entry.key().clone())
                .collect();
            for key in &stale_keys {
                pending_rpcs.remove(key);
            }
            if !stale_keys.is_empty() {
                tracing::debug!(
                    cleaned = stale_keys.len(),
                    "Cleaned up stale pending_rpcs entries"
                );
            }
        }
    });

    // Wait for servers
    http_handle.await?;
    if let Some(h) = mtls_handle {
        h.await?;
    }

    Ok(())
}

/// Load PEM bytes from env var content (preferred) or file path (fallback).
fn load_pem(
    pem_content: &Option<String>,
    file_path: &str,
    label: &str,
) -> Result<Vec<u8>, BoxError> {
    if let Some(content) = pem_content {
        tracing::info!("Loading {label} from environment variable");
        Ok(content.as_bytes().to_vec())
    } else {
        let path = std::path::PathBuf::from(file_path);
        if !path.exists() {
            return Err(format!("{label} not found: {file_path}").into());
        }
        tracing::info!("Loading {label} from file: {file_path}");
        Ok(std::fs::read(&path)?)
    }
}

/// Build rustls ServerConfig with mandatory client certificate verification.
///
/// Supports two modes:
/// - **PEM env vars** (containerized): SERVER_CERT_PEM, SERVER_KEY_PEM, ROOT_CA_PEM
/// - **File paths** (local dev): SERVER_CERT_PATH, SERVER_KEY_PATH, ROOT_CA_PATH
///
/// PEM env vars take priority when set.
fn build_mtls_config(config: &Config) -> Result<axum_server::tls_rustls::RustlsConfig, BoxError> {
    let cert_pem = load_pem(
        &config.server_cert_pem,
        &config.server_cert_path,
        "server cert",
    )?;
    let key_pem = load_pem(
        &config.server_key_pem,
        &config.server_key_path,
        "server key",
    )?;
    let ca_pem = load_pem(&config.root_ca_pem, &config.root_ca_path, "root CA")?;

    // Parse server certs
    let certs: Vec<rustls_pki_types::CertificateDer<'static>> =
        rustls_pemfile::certs(&mut &cert_pem[..]).collect::<Result<Vec<_>, _>>()?;

    // Parse server key
    let key = rustls_pemfile::private_key(&mut &key_pem[..])?
        .ok_or("No private key found in server key PEM")?;

    // Parse Root CA for client verification
    let mut root_store = rustls::RootCertStore::empty();
    let ca_certs: Vec<rustls_pki_types::CertificateDer<'static>> =
        rustls_pemfile::certs(&mut &ca_pem[..]).collect::<Result<Vec<_>, _>>()?;
    for cert in ca_certs {
        root_store.add(cert)?;
    }

    // Build client cert verifier (mandatory)
    let client_verifier =
        rustls::server::WebPkiClientVerifier::builder(std::sync::Arc::new(root_store)).build()?;

    let mut tls_config = rustls::ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(certs, key)?;

    tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(axum_server::tls_rustls::RustlsConfig::from_config(
        std::sync::Arc::new(tls_config),
    ))
}
