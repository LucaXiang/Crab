use std::path::PathBuf;
use std::sync::Arc;

use include_dir::{Dir, include_dir};
use surrealdb::Surreal;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb_migrations::MigrationRunner;

use crate::db::models::ActivationService;
use crate::message::MessageBus;
use crate::routes::{OneshotResult, OneshotRouter, build_app};
use crate::server::{Config, JwtService, ProvisioningService};

#[derive(Debug, Clone)]
pub struct ServerState {
    work_dir: PathBuf,
    config: Config,
    db: Surreal<Db>,
    jwt_service: Arc<JwtService>,
    message_bus: Arc<MessageBus>,
    activation_notify: Arc<tokio::sync::Notify>,
}
static MIGRATIONS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/migrations");

// Force rebuild when migrations change (trigger rebuild)
impl ServerState {
    pub fn new(
        config: Config,
        db: Surreal<Db>,
        jwt_service: Arc<JwtService>,
        message_bus: Arc<MessageBus>,
    ) -> Self {
        Self {
            work_dir: PathBuf::from(config.work_dir.clone()),
            config,
            db,
            jwt_service,
            message_bus,
            activation_notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Initialize server components and create state
    ///
    /// This performs heavy initialization including:
    /// - Database connection and migration
    /// - JWT service setup
    /// - Message bus setup and background task spawning
    pub async fn initialize(config: &Config) -> Self {
        let database_path = PathBuf::from(config.work_dir.clone()).join("database");
        let db = Surreal::new::<RocksDb>(database_path).await.unwrap();
        db.use_ns("edge_server").use_db("edge_server").await.ok();
        if let Err(e) = MigrationRunner::new(&db)
            .load_files(&MIGRATIONS_DIR) // Load embedded files
            .up() // Run all migrations
            .await
        {
            panic!("Failed to run migrations: {}", e);
        }

        // Initialize JWT service
        let jwt_service = Arc::new(JwtService::with_config(config.jwt.clone()));

        // Initialize message bus with configuration
        let transport_config = crate::message::TransportConfig {
            tcp_listen_addr: format!("0.0.0.0:{}", config.message_tcp_port),
            channel_capacity: 1024,
            tls_config: None, // Will be set after activation
        };
        let message_bus = Arc::new(MessageBus::from_config(transport_config));

        // Create state first (needed by handler)
        let state = Arc::new(Self::new(
            config.clone(),
            db,
            jwt_service,
            message_bus.clone(),
        ));

        // Return inner state by cloning (Arc references still exist in spawned tasks)
        (*state).clone()
    }

    /// Start background tasks (MessageBus, MessageHandler)
    pub async fn start_background_tasks(&self) {
        // Start server-side message handler with default processors
        let handler_receiver = self.message_bus.subscribe_to_clients();
        let handler_shutdown = self.message_bus.shutdown_token().clone();
        let server_tx = self.message_bus.sender().clone();
        let state = Arc::new(self.clone()); // Wrap self in Arc for the handler

        let handler = crate::message::MessageHandler::with_default_processors(
            handler_receiver,
            handler_shutdown,
            state,
        )
        .with_broadcast_tx(server_tx);

        tokio::spawn(async move {
            handler.run().await;
        });

        tracing::info!("Message handler with ACID support started in background");
    }

    /// Wait until the server is activated
    ///
    /// This method will block (asynchronously) until the server is activated.
    /// If the server is already activated, it returns immediately.
    pub async fn wait_for_activation(&self) {
        if !self.is_activated().await {
            tracing::info!("Waiting for activation signal...");
            self.activation_notify.notified().await;
            tracing::info!("Activation signal received!");
        }
    }

    /// Load TLS configuration from certificates directory
    pub fn load_tls_config(
        &self,
    ) -> Result<Option<Arc<rustls::ServerConfig>>, crate::common::AppError> {
        use std::fs;

        let certs_dir = self.work_dir.join("certs");
        let tenant_ca_path = certs_dir.join("tenant_ca.pem");
        let edge_cert_path = certs_dir.join("edge_cert.pem");
        let edge_key_path = certs_dir.join("edge_key.pem");

        if !tenant_ca_path.exists() || !edge_cert_path.exists() || !edge_key_path.exists() {
            return Ok(None);
        }

        tracing::info!("🔒 Loading mTLS certificates from {:?}", certs_dir);

        // 1. Load CA cert for client verification
        let ca_pem = fs::read_to_string(&tenant_ca_path).map_err(|e| {
            crate::common::AppError::internal(format!("Failed to read tenant CA: {}", e))
        })?;
        let ca_certs = crab_cert::to_rustls_certs(&ca_pem).map_err(|e| {
            crate::common::AppError::internal(format!("Failed to parse tenant CA: {}", e))
        })?;

        let mut client_auth_roots = rustls::RootCertStore::empty();
        for cert in ca_certs {
            client_auth_roots.add(cert).map_err(|e| {
                crate::common::AppError::internal(format!("Failed to add CA cert to store: {}", e))
            })?;
        }

        let client_auth =
            rustls::server::WebPkiClientVerifier::builder(Arc::new(client_auth_roots))
                .build()
                .map_err(|e| {
                    crate::common::AppError::internal(format!(
                        "Failed to build client verifier: {}",
                        e
                    ))
                })?;

        // 2. Load Server cert and key
        let cert_pem = fs::read_to_string(&edge_cert_path).map_err(|e| {
            crate::common::AppError::internal(format!("Failed to read edge cert: {}", e))
        })?;
        let key_pem = fs::read_to_string(&edge_key_path).map_err(|e| {
            crate::common::AppError::internal(format!("Failed to read edge key: {}", e))
        })?;

        let certs = crab_cert::to_rustls_certs(&cert_pem).map_err(|e| {
            crate::common::AppError::internal(format!("Failed to parse edge cert: {}", e))
        })?;
        let key = crab_cert::to_rustls_key(&key_pem).map_err(|e| {
            crate::common::AppError::internal(format!("Failed to parse edge key: {}", e))
        })?;

        // 3. Build ServerConfig
        let config = rustls::ServerConfig::builder()
            .with_client_cert_verifier(client_auth)
            .with_single_cert(certs, key)
            .map_err(|e| {
                crate::common::AppError::internal(format!("Failed to build server config: {}", e))
            })?;

        Ok(Some(Arc::new(config)))
    }

    pub fn get_db(&self) -> Surreal<Db> {
        self.db.clone()
    }

    pub fn work_dir(&self) -> &PathBuf {
        &self.work_dir
    }

    pub fn get_jwt_service(&self) -> Arc<JwtService> {
        self.jwt_service.clone()
    }

    /// Get the message bus for publishing and subscribing to messages
    pub fn message_bus(&self) -> &Arc<MessageBus> {
        &self.message_bus
    }

    /// Get a clone of the message bus (for publishing)
    pub fn get_message_bus(&self) -> Arc<MessageBus> {
        self.message_bus.clone()
    }

    /// Create a fully configured router with middleware for this state
    pub fn router(&self) -> axum::Router<ServerState> {
        build_app(self)
    }

    pub async fn oneshot(&self, request: http::Request<axum::body::Body>) -> OneshotResult {
        let mut router = self.router();
        OneshotRouter::oneshot(&mut router, self, request).await
    }

    /// Save certificates for mTLS (called during activation)
    pub async fn save_certificates(
        &self,
        tenant_ca_pem: &str,
        edge_cert_pem: &str,
        edge_key_pem: &str,
    ) -> Result<(), crate::common::AppError> {
        use std::fs;

        let certs_dir = self.work_dir.join("certs");
        fs::create_dir_all(&certs_dir).map_err(|e| {
            crate::common::AppError::internal(format!("Failed to create certs dir: {}", e))
        })?;

        fs::write(certs_dir.join("tenant_ca.pem"), tenant_ca_pem).map_err(|e| {
            crate::common::AppError::internal(format!("Failed to write tenant CA: {}", e))
        })?;
        fs::write(certs_dir.join("edge_cert.pem"), edge_cert_pem).map_err(|e| {
            crate::common::AppError::internal(format!("Failed to write edge cert: {}", e))
        })?;
        fs::write(certs_dir.join("edge_key.pem"), edge_key_pem).map_err(|e| {
            crate::common::AppError::internal(format!("Failed to write edge key: {}", e))
        })?;

        tracing::info!("📜 Certificates saved to {:?}", certs_dir);
        Ok(())
    }

    /// Get activation service
    pub fn activation_service(&self) -> ActivationService {
        ActivationService::new(self.db.clone())
    }

    /// Create a provisioning service for remote activation
    pub fn provisioning_service(&self, auth_url: String) -> ProvisioningService {
        ProvisioningService::new(self.clone(), auth_url)
    }

    /// Activate the server with full metadata
    pub async fn activate_with_metadata(
        &self,
        tenant_id: &str,
        tenant_name: &str,
        edge_id: &str,
        edge_name: &str,
        device_id: &str,
        cert_fingerprint: &str,
    ) -> Result<(), crate::common::AppError> {
        let service = self.activation_service();
        service
            .activate(crate::db::models::activation::ActivationParams {
                tenant_id,
                tenant_name,
                edge_id,
                edge_name,
                device_id,
                cert_fingerprint,
                cert_expires_at: None, // TODO: Parse cert expiry from PEM
            })
            .await?;

        tracing::info!(
            "🚀 Server activated! tenant={}, edge={}, device={}",
            tenant_name,
            edge_name,
            device_id
        );

        // Notify main thread to start services
        self.activation_notify.notify_waiters();

        Ok(())
    }

    /// Check if server is activated
    pub async fn is_activated(&self) -> bool {
        self.activation_service().is_activated().await
    }

    /// Deactivate server and reset state (delete certificates)
    pub async fn deactivate_and_reset(&self) -> Result<(), crate::common::AppError> {
        tracing::warn!("⚠️ Deactivating server and resetting state due to certificate error");

        // 1. Deactivate in DB
        self.activation_service().deactivate().await?;

        // 2. Delete certificates
        let certs_dir = self.work_dir.join("certs");
        if certs_dir.exists() {
            tracing::info!("🗑️ Removing invalid certificates from {:?}", certs_dir);
            std::fs::remove_dir_all(&certs_dir).map_err(|e| {
                crate::common::AppError::internal(format!("Failed to delete certs dir: {}", e))
            })?;
        }

        Ok(())
    }

    /// Print a banner with activation status to stdout
    pub async fn print_activation_banner(&self) {
        // 1. Check if theoretically activated in DB
        if !self.is_activated().await {
            self.print_not_activated_banner();
            return;
        }

        // 2. Integrity Check: Verify certificates
        // If DB says activated, certificates MUST be valid.
        match self.load_tls_config() {
            Ok(Some(_)) => {
                // ✅ Integrity Passed: DB Active + Certs Valid
                self.print_activated_banner_content().await;
            }
            Err(e) => {
                // ❌ Integrity Failed: Certs Invalid
                tracing::error!(
                    "❌ System Integrity Check Failed: Certificates are invalid: {}",
                    e
                );
                self.handle_integrity_failure().await;
            }
            Ok(None) => {
                // ❌ Integrity Failed: Certs Missing
                tracing::error!("❌ System Integrity Check Failed: Certificates are missing.");
                self.handle_integrity_failure().await;
            }
        }
    }

    async fn handle_integrity_failure(&self) {
        tracing::warn!("⚠️  Resetting system state to ensure security...");
        if let Err(e) = self.deactivate_and_reset().await {
            tracing::error!("❌ Failed to reset state: {}", e);
        } else {
            tracing::info!("✅ System state reset successfully.");
        }
        self.print_not_activated_banner();
    }

    async fn print_activated_banner_content(&self) {
        let activation = self
            .activation_service()
            .get_status()
            .await
            .unwrap_or_default();
        let tenant_id = activation
            .tenant_id
            .unwrap_or_else(|| "Unknown".to_string());
        let edge_id = activation.edge_id.unwrap_or_else(|| "Unknown".to_string());
        let device_id = activation
            .device_id
            .unwrap_or_else(|| "Unknown".to_string());

        // Calculate Fingerprints
        let certs_dir = self.work_dir.join("certs");
        let tenant_ca_pem =
            std::fs::read_to_string(certs_dir.join("tenant_ca.pem")).unwrap_or_default();
        let edge_cert_pem =
            std::fs::read_to_string(certs_dir.join("edge_cert.pem")).unwrap_or_default();

        let ca_fingerprint = Self::get_fingerprint(&tenant_ca_pem);
        let edge_fingerprint = Self::get_fingerprint(&edge_cert_pem);

        // Format IPs
        let https_addr = format!("0.0.0.0:{}", self.config.http_port);
        let tcp_addr = format!("0.0.0.0:{}", self.config.message_tcp_port);

        tracing::info!("");
        tracing::info!(
            "╔════════════════════════════════════════════════════════════════════════════╗"
        );
        tracing::info!(
            "║                   🦀 Crab Edge Server - Activated 🚀                       ║"
        );
        tracing::info!(
            "╠════════════════════════════════════════════════════════════════════════════╣"
        );
        tracing::info!("║ 🏢 Tenant ID       : {:<53} ║", tenant_id);
        tracing::info!("║ 🆔 Edge ID         : {:<53} ║", edge_id);
        tracing::info!("║ 🖥️  Device ID       : {:<53} ║", device_id);
        tracing::info!(
            "║ 🔒 CA Fingerprint  : {:<53} ║",
            Self::truncate_fingerprint(&ca_fingerprint)
        );
        tracing::info!(
            "║ 🔑 Edge Fingerprint: {:<53} ║",
            Self::truncate_fingerprint(&edge_fingerprint)
        );
        tracing::info!("║ 🌐 HTTPS Listen    : {:<53} ║", https_addr);
        tracing::info!("║ 🔌 TCP mTLS Listen : {:<53} ║", tcp_addr);
        tracing::info!(
            "╚════════════════════════════════════════════════════════════════════════════╝"
        );
        tracing::info!("");
        tracing::info!("✅ mTLS Certificates loaded and verified.");
    }

    fn get_fingerprint(pem_content: &str) -> String {
        use sha2::{Digest, Sha256};

        match crab_cert::to_rustls_certs(pem_content) {
            Ok(certs) => {
                if let Some(cert) = certs.first() {
                    let mut hasher = Sha256::new();
                    hasher.update(cert.as_ref());
                    let result = hasher.finalize();
                    hex::encode(result)
                } else {
                    "Unknown (No Certs Found)".to_string()
                }
            }
            Err(_) => "Unknown (Parse Error)".to_string(),
        }
    }

    fn truncate_fingerprint(fp: &str) -> String {
        if fp.len() > 40 {
            format!("{}...", &fp[0..40])
        } else {
            fp.to_string()
        }
    }

    fn print_not_activated_banner(&self) {
        tracing::info!("");
        tracing::info!(
            "╔════════════════════════════════════════════════════════════════════════════╗"
        );
        tracing::info!(
            "║                   ⚠️  Edge Server NOT Activated                            ║"
        );
        tracing::info!(
            "╚════════════════════════════════════════════════════════════════════════════╝"
        );
    }
}
