//! Cloud server configuration

type BoxError = Box<dyn std::error::Error + Send + Sync>;

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
    /// Root CA PEM content (env: ROOT_CA_PEM) or file path (env: ROOT_CA_PATH)
    pub root_ca_pem: Option<String>,
    pub root_ca_path: String,
    /// Server cert PEM content (env: SERVER_CERT_PEM) or file path (env: SERVER_CERT_PATH)
    pub server_cert_pem: Option<String>,
    pub server_cert_path: String,
    /// Server key PEM content (env: SERVER_KEY_PEM) or file path (env: SERVER_KEY_PATH)
    pub server_key_pem: Option<String>,
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
    /// JWT secret for tenant authentication
    pub jwt_secret: String,
    /// Stripe Price ID for Basic plan (monthly)
    pub stripe_basic_price_id: String,
    /// Stripe Price ID for Pro plan (monthly)
    pub stripe_pro_price_id: String,
}

impl Config {
    /// Require a secret env var: must be set and non-empty in non-development environments.
    fn require_secret(name: &str, environment: &str) -> Result<String, BoxError> {
        let val = match std::env::var(name) {
            Ok(v) => v,
            Err(_) => {
                if environment != "development" {
                    return Err(format!("{name} must be set in {environment} environment").into());
                }
                format!("dev-{name}-not-for-production")
            }
        };
        if val.is_empty() && environment != "development" {
            return Err(format!("{name} must not be empty in {environment} environment").into());
        }
        Ok(val)
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, BoxError> {
        let environment = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".into());

        Ok(Self {
            database_url: std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set")?,
            http_port: std::env::var("HTTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            mtls_port: std::env::var("MTLS_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8443),
            root_ca_pem: std::env::var("ROOT_CA_PEM").ok().filter(|s| !s.is_empty()),
            root_ca_path: std::env::var("ROOT_CA_PATH")
                .unwrap_or_else(|_| "certs/root_ca.pem".into()),
            server_cert_pem: std::env::var("SERVER_CERT_PEM")
                .ok()
                .filter(|s| !s.is_empty()),
            server_cert_path: std::env::var("SERVER_CERT_PATH")
                .unwrap_or_else(|_| "certs/server.pem".into()),
            server_key_pem: std::env::var("SERVER_KEY_PEM")
                .ok()
                .filter(|s| !s.is_empty()),
            server_key_path: std::env::var("SERVER_KEY_PATH")
                .unwrap_or_else(|_| "certs/server.key".into()),
            environment: environment.clone(),
            ses_from_email: std::env::var("SES_FROM_EMAIL")
                .unwrap_or_else(|_| "noreply@redcoral.app".into()),
            stripe_secret_key: Self::require_secret("STRIPE_SECRET_KEY", &environment)?,
            stripe_webhook_secret: Self::require_secret("STRIPE_WEBHOOK_SECRET", &environment)?,
            registration_success_url: std::env::var("REGISTRATION_SUCCESS_URL")
                .unwrap_or_else(|_| "https://redcoral.app/registration/success".into()),
            registration_cancel_url: std::env::var("REGISTRATION_CANCEL_URL")
                .unwrap_or_else(|_| "https://redcoral.app/registration/cancel".into()),
            update_s3_bucket: std::env::var("UPDATE_S3_BUCKET")
                .unwrap_or_else(|_| "crab-app-updates".into()),
            update_download_base_url: std::env::var("UPDATE_DOWNLOAD_BASE_URL")
                .unwrap_or_else(|_| "https://updates.redcoral.app".into()),
            jwt_secret: Self::require_secret("JWT_SECRET", &environment)?,
            stripe_basic_price_id: std::env::var("STRIPE_BASIC_PRICE_ID")
                .unwrap_or_else(|_| "price_1T30z63Ednyw0kfvGYVXXDaB".into()),
            stripe_pro_price_id: std::env::var("STRIPE_PRO_PRICE_ID")
                .unwrap_or_else(|_| "price_1T30zB3Ednyw0kfvoGku9ZbF".into()),
        })
    }
}
