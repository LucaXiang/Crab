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
mod crypto;
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
use tokio::signal;

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

    // Seed dev account (idempotent — skipped in production)
    if config.environment != "production" {
        seed_dev_account(&state.pool).await;
    }

    // Build routers
    let public_app = api::public_router(state.clone());
    let edge_app = api::edge_router(state.clone());

    // Start HTTP server (public)
    let http_addr = format!("0.0.0.0:{}", config.http_port);
    let http_listener = tokio::net::TcpListener::bind(&http_addr).await?;
    tracing::info!("crab-cloud HTTP listening on {http_addr}");

    // mTLS server handle for graceful shutdown
    let mtls_server_handle = axum_server::Handle::new();

    let http_handle = tokio::spawn({
        let shutdown = shutdown_signal();
        async move {
            if let Err(e) = axum::serve(http_listener, public_app)
                .with_graceful_shutdown(shutdown)
                .await
            {
                tracing::error!("HTTP server error: {e}");
            }
        }
    });

    // Start mTLS server (edge-only)
    let mtls_addr = std::net::SocketAddr::from(([0, 0, 0, 0], config.mtls_port));
    let mtls_handle = match build_mtls_config(&config, &state.root_ca_pem) {
        Ok(tls_config) => {
            tracing::info!("crab-cloud mTLS listening on {mtls_addr}");
            let handle = mtls_server_handle.clone();
            Some(tokio::spawn(async move {
                if let Err(e) = axum_server::bind_rustls(mtls_addr, tls_config)
                    .handle(handle)
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
            rate_limiter.cleanup();
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

    // Periodic data cleanup (every hour): audit logs, revoked tokens, webhook events, old commands, stale pending ops
    {
        let pool = state.pool.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
            loop {
                interval.tick().await;
                let now = shared::util::now_millis();
                let day_ms: i64 = 86_400_000;

                // Audit logs older than 90 days
                match sqlx::query("DELETE FROM audit_logs WHERE created_at < $1")
                    .bind(now - 90 * day_ms)
                    .execute(&pool)
                    .await
                {
                    Ok(r) if r.rows_affected() > 0 => {
                        tracing::info!(deleted = r.rows_affected(), "Cleaned up old audit_logs");
                    }
                    Err(e) => tracing::warn!("audit_logs cleanup error: {e}"),
                    _ => {}
                }

                // Expired refresh tokens older than 30 days (revoked or naturally expired)
                match sqlx::query("DELETE FROM refresh_tokens WHERE expires_at < $1")
                    .bind(now - 30 * day_ms)
                    .execute(&pool)
                    .await
                {
                    Ok(r) if r.rows_affected() > 0 => {
                        tracing::info!(
                            deleted = r.rows_affected(),
                            "Cleaned up expired refresh_tokens"
                        );
                    }
                    Err(e) => tracing::warn!("refresh_tokens cleanup error: {e}"),
                    _ => {}
                }

                // Processed webhook events older than 7 days
                match sqlx::query("DELETE FROM processed_webhook_events WHERE processed_at < $1")
                    .bind(now - 7 * day_ms)
                    .execute(&pool)
                    .await
                {
                    Ok(r) if r.rows_affected() > 0 => {
                        tracing::info!(
                            deleted = r.rows_affected(),
                            "Cleaned up old processed_webhook_events"
                        );
                    }
                    Err(e) => tracing::warn!("webhook_events cleanup error: {e}"),
                    _ => {}
                }

                // Completed/failed commands older than 90 days
                match sqlx::query(
                    "DELETE FROM store_commands WHERE status IN ('completed', 'failed') AND created_at < $1",
                )
                .bind(now - 90 * day_ms)
                .execute(&pool)
                .await
                {
                    Ok(r) if r.rows_affected() > 0 => {
                        tracing::info!(
                            deleted = r.rows_affected(),
                            "Cleaned up old store_commands"
                        );
                    }
                    Err(e) => tracing::warn!("store_commands cleanup error: {e}"),
                    _ => {}
                }

                // Note: store_pending_ops are NOT cleaned by TTL — they represent
                // undelivered Console edits for offline edges. Delivery deletes them
                // individually via pending_ops::delete_one() when the edge reconnects.
            }
        });
    }

    // Periodic orphaned image cleanup (every 10 minutes, delete images orphaned >1 hour ago)
    {
        let state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(600));
            loop {
                interval.tick().await;
                let cutoff = shared::util::now_millis() - 3_600_000; // 1 hour grace period
                match db::tenant_images::fetch_orphans(&state.pool, cutoff, 100).await {
                    Ok(orphans) if !orphans.is_empty() => {
                        let count = orphans.len();
                        for (tenant_id, hash) in orphans {
                            api::image::delete_s3_image(&state, tenant_id, &hash).await;
                            let _ =
                                db::tenant_images::delete_records(&state.pool, tenant_id, &hash)
                                    .await;
                        }
                        tracing::info!(count, "Orphaned S3 images cleaned up");
                    }
                    Ok(_) => {} // no orphans
                    Err(e) => {
                        tracing::warn!("Failed to fetch orphaned images: {e}");
                    }
                }
            }
        });
    }

    // Wait for HTTP server (it shuts down on SIGTERM via graceful_shutdown)
    http_handle.await?;

    // Gracefully shut down mTLS server
    mtls_server_handle.graceful_shutdown(Some(std::time::Duration::from_secs(30)));
    if let Some(h) = mtls_handle {
        h.await?;
    }

    tracing::info!("crab-cloud shut down gracefully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to listen for ctrl+c");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to listen for SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Received Ctrl+C, shutting down..."),
        _ = terminate => tracing::info!("Received SIGTERM, shutting down..."),
    }
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

/// Seed a dev test account with active pro subscription.
/// Idempotent — does nothing if the account already exists.
async fn seed_dev_account(pool: &sqlx::PgPool) {
    let email = "dev@redcoral.app";
    let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM tenants WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
    if exists.is_some() {
        tracing::debug!("Dev account already exists, skipping seed");
        return;
    }
    // password: dev123456
    let hashed = crate::util::hash_password("dev123456").expect("hash_password");
    let now = shared::util::now_millis();
    let tenant_id: i64 = 1_000_000_000_000_001;
    if let Err(e) = sqlx::query(
        "INSERT INTO tenants (id, email, hashed_password, name, status, created_at, verified_at)
         VALUES ($1, $2, $3, 'Dev Tenant', 'active', $4, $4)",
    )
    .bind(tenant_id)
    .bind(email)
    .bind(&hashed)
    .bind(now)
    .execute(pool)
    .await
    {
        tracing::warn!("Failed to seed dev tenant: {e}");
        return;
    }
    if let Err(e) = sqlx::query(
        "INSERT INTO subscriptions (id, tenant_id, status, plan, max_stores, features, cancel_at_period_end, created_at)
         VALUES ('dev_sub_001', $1, 'active', 'pro', 5, '{}', false, $2)",
    )
    .bind(tenant_id)
    .bind(now)
    .execute(pool)
    .await
    {
        tracing::warn!("Failed to seed dev subscription: {e}");
        return;
    }
    // Seed mock P12 certificate metadata (no real P12 data, just metadata for UI/Verifactu flow)
    let expires_at = now + 365 * 24 * 60 * 60 * 1000; // 1 year from now
    if let Err(e) = sqlx::query(
        "INSERT INTO p12_certificates
            (tenant_id, fingerprint, common_name, serial_number, organization_id,
             organization, issuer, country, expires_at, not_before, uploaded_at, updated_at)
         VALUES ($1, 'DEV-MOCK-FINGERPRINT', 'DEV TENANT - CERTIFICADO DE PRUEBA',
                 'IDCES-B99999999', 'B99999999', 'DEV TENANT SL',
                 'AC FNMT Usuarios (DEV)', 'ES', $2, $3, $3, $3)",
    )
    .bind(tenant_id)
    .bind(expires_at)
    .bind(now)
    .execute(pool)
    .await
    {
        tracing::warn!("Failed to seed dev P12: {e}");
    }
    tracing::info!(
        "Dev account seeded: {email} / dev123456 (pro plan, 5 stores, mock P12 NIF=B99999999)"
    );
}

/// Build rustls ServerConfig with mandatory client certificate verification.
///
/// `root_ca_pem` comes from AppState (Secrets Manager → local cache fallback),
/// ensuring the mTLS trust root is always consistent with the PKI that signed
/// tenant/entity certs during activation.
fn build_mtls_config(
    config: &Config,
    root_ca_pem: &str,
) -> Result<axum_server::tls_rustls::RustlsConfig, BoxError> {
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
    let ca_pem = root_ca_pem.as_bytes().to_vec();

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

    let mut tls_config =
        rustls::ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
            .with_client_cert_verifier(client_verifier)
            .with_single_cert(certs, key)?;

    tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(axum_server::tls_rustls::RustlsConfig::from_config(
        std::sync::Arc::new(tls_config),
    ))
}
