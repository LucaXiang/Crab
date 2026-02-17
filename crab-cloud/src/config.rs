//! Cloud server configuration

/// Cloud server configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    /// PostgreSQL connection URL
    pub database_url: String,
    /// HTTP port (health check, non-mTLS)
    pub http_port: u16,
    /// mTLS port (edge-server sync API)
    pub mtls_port: u16,
    /// Path to Root CA PEM (for mTLS client verification)
    pub root_ca_path: String,
    /// Path to server TLS cert PEM
    pub server_cert_path: String,
    /// Path to server TLS key PEM
    pub server_key_path: String,
    /// Environment: development | staging | production
    pub environment: String,
    /// SES sender email address
    pub ses_from_email: String,
    /// Stripe secret key
    pub stripe_secret_key: String,
    /// Stripe webhook signing secret
    pub stripe_webhook_secret: String,
    /// URL to redirect after successful registration checkout
    pub registration_success_url: String,
    /// URL to redirect after cancelled registration checkout
    pub registration_cancel_url: String,
    /// S3 bucket for update artifacts
    pub update_s3_bucket: String,
    /// CloudFront or S3 base URL for download
    pub update_download_base_url: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            http_port: std::env::var("HTTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            mtls_port: std::env::var("MTLS_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8443),
            root_ca_path: std::env::var("ROOT_CA_PATH")
                .unwrap_or_else(|_| "certs/root_ca.pem".into()),
            server_cert_path: std::env::var("SERVER_CERT_PATH")
                .unwrap_or_else(|_| "certs/server.pem".into()),
            server_key_path: std::env::var("SERVER_KEY_PATH")
                .unwrap_or_else(|_| "certs/server.key".into()),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".into()),
            ses_from_email: std::env::var("SES_FROM_EMAIL")
                .unwrap_or_else(|_| "noreply@crab.es".into()),
            stripe_secret_key: std::env::var("STRIPE_SECRET_KEY").unwrap_or_default(),
            stripe_webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_default(),
            registration_success_url: std::env::var("REGISTRATION_SUCCESS_URL")
                .unwrap_or_else(|_| "https://crab.es/registration/success".into()),
            registration_cancel_url: std::env::var("REGISTRATION_CANCEL_URL")
                .unwrap_or_else(|_| "https://crab.es/registration/cancel".into()),
            update_s3_bucket: std::env::var("UPDATE_S3_BUCKET")
                .unwrap_or_else(|_| "crab-app-updates".into()),
            update_download_base_url: std::env::var("UPDATE_DOWNLOAD_BASE_URL")
                .unwrap_or_else(|_| "https://updates.crab.es".into()),
        }
    }

    #[allow(dead_code)]
    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }
}
