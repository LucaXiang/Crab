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

    // Wait for servers
    http_handle.await?;
    if let Some(h) = mtls_handle {
        h.await?;
    }

    Ok(())
}

/// Build rustls ServerConfig with mandatory client certificate verification
fn build_mtls_config(config: &Config) -> Result<axum_server::tls_rustls::RustlsConfig, BoxError> {
    use std::path::PathBuf;

    let cert_path = PathBuf::from(&config.server_cert_path);
    let key_path = PathBuf::from(&config.server_key_path);
    let ca_path = PathBuf::from(&config.root_ca_path);

    // Verify files exist
    if !cert_path.exists() {
        return Err(format!("Server cert not found: {}", config.server_cert_path).into());
    }
    if !key_path.exists() {
        return Err(format!("Server key not found: {}", config.server_key_path).into());
    }
    if !ca_path.exists() {
        return Err(format!("Root CA not found: {}", config.root_ca_path).into());
    }

    // Build rustls config with client auth
    let cert_pem = std::fs::read(&cert_path)?;
    let key_pem = std::fs::read(&key_path)?;
    let ca_pem = std::fs::read(&ca_path)?;

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
