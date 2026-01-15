use std::path::PathBuf;
use std::sync::Arc;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::server::services::{ActivationService, CertService, HttpsService, MessageBusService};
use crate::server::{Config, JwtService};

#[derive(Clone, Debug)]
pub struct ServerState {
    pub config: Config,
    pub db: Surreal<Db>,
    pub activation: ActivationService,
    pub cert_service: CertService,
    pub message_bus: MessageBusService,
    pub https: HttpsService,
    pub jwt_service: Arc<JwtService>,
}

impl ServerState {
    pub fn new(
        config: Config,
        db: Surreal<Db>,
        activation: ActivationService,
        cert_service: CertService,
        message_bus: MessageBusService,
        https: HttpsService,
        jwt_service: Arc<JwtService>,
    ) -> Self {
        Self {
            config,
            db,
            activation,
            cert_service,
            message_bus,
            https,
            jwt_service,
        }
    }

    pub async fn initialize(config: &Config) -> Self {
        // 1. Initialize DB
        // Use work_dir/crab.db for database path
        let db_path = PathBuf::from(&config.work_dir).join("crab.db");
        let db_path_str = db_path.to_string_lossy();

        let db_service = crate::db::DbService::new(&db_path_str)
            .await
            .expect("Failed to initialize database");
        let db = db_service.db;

        // 2. Initialize Services
        let activation = ActivationService::new(
            config.auth_server_url.clone(),
            PathBuf::from(&config.work_dir),
        );
        let cert_service = CertService::new(PathBuf::from(&config.work_dir));
        let message_bus = MessageBusService::new(config);
        let https = HttpsService::new(config.clone());
        let jwt_service = Arc::new(JwtService::default());

        let state = Self::new(
            config.clone(),
            db,
            activation,
            cert_service,
            message_bus,
            https.clone(),
            jwt_service,
        );

        // 3. Late initialization for HttpsService (needs state)
        https.initialize(state.clone());

        state
    }

    pub async fn start_background_tasks(&self) {
        // Start MessageBus background tasks
        self.message_bus.start_background_tasks(self.clone());
    }

    pub fn get_db(&self) -> Surreal<Db> {
        self.db.clone()
    }

    pub fn work_dir(&self) -> PathBuf {
        PathBuf::from(&self.config.work_dir)
    }

    pub fn get_jwt_service(&self) -> Arc<JwtService> {
        self.jwt_service.clone()
    }

    pub fn message_bus(&self) -> &Arc<crate::message::MessageBus> {
        self.message_bus.bus()
    }

    pub fn activation_service(&self) -> &ActivationService {
        &self.activation
    }

    pub fn cert_service(&self) -> &CertService {
        &self.cert_service
    }

    pub fn https_service(&self) -> &HttpsService {
        &self.https
    }

    pub async fn is_activated(&self) -> bool {
        self.activation.is_activated().await
    }

    pub async fn wait_for_activation(&self) {
        self.activation
            .wait_for_activation(&self.cert_service)
            .await
    }

    pub fn provisioning_service(&self, auth_url: String) -> crate::server::ProvisioningService {
        crate::server::ProvisioningService::new(self.clone(), auth_url)
    }

    pub fn load_tls_config(
        &self,
    ) -> Result<Option<Arc<rustls::ServerConfig>>, crate::common::AppError> {
        self.cert_service.load_tls_config()
    }

    pub async fn save_certificates(
        &self,
        tenant_ca_pem: &str,
        edge_cert_pem: &str,
        edge_key_pem: &str,
    ) -> Result<(), crate::common::AppError> {
        self.cert_service
            .save_certificates(tenant_ca_pem, edge_cert_pem, edge_key_pem)
            .await
    }

    pub async fn deactivate_and_reset(&self) -> Result<(), crate::common::AppError> {
        self.cert_service.delete_certificates()?;
        self.activation.deactivate_and_reset().await
    }

    pub async fn print_activated_banner_content(&self) {
        // Simply log for now, as detailed banner is printed by Server::print_activation_banner
        tracing::info!("Server state is active and running.");
    }
}
