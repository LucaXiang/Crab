use std::path::PathBuf;
use std::sync::Arc;

use include_dir::{Dir, include_dir};
use surrealdb::Surreal;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb_migrations::MigrationRunner;

use crate::db::models::ActivationService;
use crate::message::MessageBus;
use crate::routes::{OneshotResult, OneshotRouter, build_app};
use crate::server::{Config, JwtService};

#[derive(Debug, Clone)]
pub struct ServerState {
    work_dir: PathBuf,
    db: Surreal<Db>,
    jwt_service: Arc<JwtService>,
    message_bus: Arc<MessageBus>,
}
static MIGRATIONS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/migrations");

impl ServerState {
    pub fn new(
        work_dir: PathBuf,
        db: Surreal<Db>,
        jwt_service: Arc<JwtService>,
        message_bus: Arc<MessageBus>,
    ) -> Self {
        Self {
            work_dir,
            db,
            jwt_service,
            message_bus,
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
        MigrationRunner::new(&db)
            .load_files(&MIGRATIONS_DIR) // Load embedded files
            .up() // Run all migrations
            .await
            .expect("Failed to run migrations");

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
            PathBuf::from(config.work_dir.clone()),
            db,
            jwt_service,
            message_bus.clone(),
        ));

        // Start server-side message handler with default processors
        let handler_receiver = message_bus.subscribe_to_clients();
        let handler_shutdown = message_bus.shutdown_token().clone();
        let server_tx = message_bus.sender().clone();
        let handler = crate::message::MessageHandler::with_default_processors(
            handler_receiver,
            handler_shutdown,
            state.clone(),
        )
        .with_broadcast_tx(server_tx);

        tokio::spawn(async move {
            handler.run().await;
        });

        tracing::info!("Message handler with ACID support started in background");

        // Start TCP server for message bus in background
        let bus_clone = message_bus.clone();
        tokio::spawn(async move {
            // Pass None for tls_config - will be activated later
            if let Err(e) = bus_clone.start_tcp_server(None).await {
                tracing::error!("Message bus TCP server error: {}", e);
            }
        });

        tracing::info!("Message bus TCP server started in background");

        // Return inner state by cloning (Arc references still exist in spawned tasks)
        (*state).clone()
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

    /// Process a request using oneshot pattern - no network required
    ///
    /// This is a convenience method that creates a router and processes
    /// the request directly in memory.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use http::Request;
    /// use axum::body::Body;
    ///
    /// let state = ServerState::initialize(&config).await;
    /// let request = Request::builder()
    ///     .uri("/health")
    ///     .body(Body::empty())?;
    ///
    /// let response = state.oneshot(request).await?;
    /// ```
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

    /// Activate the server with full metadata
    pub async fn activate_with_metadata(
        &self,
        tenant_id: &str,
        tenant_name: &str,
        edge_id: &str,
        edge_name: &str,
        cert_fingerprint: &str,
    ) -> Result<(), crate::common::AppError> {
        let service = self.activation_service();
        service
            .activate(
                tenant_id,
                tenant_name,
                edge_id,
                edge_name,
                cert_fingerprint,
                None, // TODO: Parse cert expiry from PEM
            )
            .await?;

        tracing::info!(
            "🚀 Server activated! tenant={}, edge={}",
            tenant_name,
            edge_name
        );
        Ok(())
    }

    /// Check if server is activated
    pub async fn is_activated(&self) -> bool {
        self.activation_service().is_activated().await
    }
}
