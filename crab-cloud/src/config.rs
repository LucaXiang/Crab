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
        }
    }

    #[allow(dead_code)]
    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }
}
