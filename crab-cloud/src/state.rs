//! Application state for crab-cloud

use aws_sdk_s3::Client as S3Client;
use aws_sdk_secretsmanager::Client as SmClient;
use aws_sdk_sesv2::Client as SesClient;
use sqlx::PgPool;
use tokio::sync::OnceCell;

use crate::auth::QuotaCache;
use crate::config::Config;

/// Shared application state
#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    /// PostgreSQL connection pool (shared with crab-auth)
    pub pool: PgPool,
    /// CA store for loading Tenant CA certs from Secrets Manager
    pub ca_store: CaStore,
    /// Root CA PEM for mTLS verification
    pub root_ca_pem: String,
    /// AWS SES client for sending emails
    pub ses: SesClient,
    /// Stripe secret key
    pub stripe_secret_key: String,
    /// Stripe webhook signing secret
    pub stripe_webhook_secret: String,
    /// SES sender email address
    pub ses_from_email: String,
    /// URL to redirect after successful registration checkout
    pub registration_success_url: String,
    /// URL to redirect after cancelled registration checkout
    pub registration_cancel_url: String,
    /// AWS S3 client for update artifacts
    pub s3: S3Client,
    /// S3 bucket for update artifacts
    pub update_s3_bucket: String,
    /// Base URL for update downloads
    pub update_download_base_url: String,
    /// JWT secret for tenant authentication
    pub jwt_secret: String,
    /// Quota validation cache
    pub quota_cache: QuotaCache,
    /// Rate limiter for login/registration routes
    pub rate_limiter: crate::auth::rate_limit::RateLimiter,
}

/// Certificate Authority store (reads from AWS Secrets Manager)
///
/// Used to load Tenant CA certificates for SignedBinding verification.
#[derive(Clone)]
#[allow(dead_code)]
pub struct CaStore {
    sm: SmClient,
    /// Root CA cert cache
    root_ca_cache: std::sync::Arc<OnceCell<String>>,
}

impl CaStore {
    pub fn new(sm: SmClient) -> Self {
        Self {
            sm,
            root_ca_cache: std::sync::Arc::new(OnceCell::new()),
        }
    }

    /// Load Tenant CA cert PEM from Secrets Manager
    pub async fn load_tenant_ca_cert(
        &self,
        tenant_id: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let secret_name = format!("crab-auth/tenant/{tenant_id}");
        let output = self
            .sm
            .get_secret_value()
            .secret_id(&secret_name)
            .send()
            .await?;

        let json = output.secret_string().ok_or("Secret has no string value")?;

        #[derive(serde::Deserialize)]
        struct CaSecret {
            cert_pem: String,
        }

        let secret: CaSecret = serde_json::from_str(json)?;
        Ok(secret.cert_pem)
    }

    /// Get Root CA cert PEM (cached)
    #[allow(dead_code)]
    pub async fn get_root_ca_cert(
        &self,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.root_ca_cache
            .get_or_try_init(|| async {
                let output = self
                    .sm
                    .get_secret_value()
                    .secret_id("crab-auth/root-ca")
                    .send()
                    .await?;

                let json = output.secret_string().ok_or("Secret has no string value")?;

                #[derive(serde::Deserialize)]
                struct CaSecret {
                    cert_pem: String,
                }

                let secret: CaSecret = serde_json::from_str(json)?;
                Ok(secret.cert_pem)
            })
            .await
            .cloned()
    }
}

impl AppState {
    /// Create a new AppState
    pub async fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Connect to PostgreSQL
        let pool = PgPool::connect(&config.database_url).await?;

        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;

        // Initialize AWS SDK
        let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let sm_client = SmClient::new(&aws_config);
        let s3 = S3Client::new(&aws_config);

        // SES may run in a different region (eu-south-2 doesn't support SES)
        let ses = if let Ok(ses_region) = std::env::var("SES_REGION") {
            let ses_config = aws_config
                .to_builder()
                .region(aws_config::Region::new(ses_region))
                .build();
            SesClient::new(&ses_config)
        } else {
            SesClient::new(&aws_config)
        };
        let ca_store = CaStore::new(sm_client);

        // Load Root CA PEM
        let root_ca_pem = std::fs::read_to_string(&config.root_ca_path).unwrap_or_else(|_| {
            tracing::warn!(
                "Root CA file not found at {}, will use Secrets Manager",
                config.root_ca_path
            );
            String::new()
        });

        Ok(Self {
            pool,
            ca_store,
            root_ca_pem,
            ses,
            stripe_secret_key: config.stripe_secret_key.clone(),
            stripe_webhook_secret: config.stripe_webhook_secret.clone(),
            ses_from_email: config.ses_from_email.clone(),
            registration_success_url: config.registration_success_url.clone(),
            registration_cancel_url: config.registration_cancel_url.clone(),
            s3,
            update_s3_bucket: config.update_s3_bucket.clone(),
            update_download_base_url: config.update_download_base_url.clone(),
            jwt_secret: config.jwt_secret.clone(),
            quota_cache: QuotaCache::new(),
            rate_limiter: crate::auth::rate_limit::RateLimiter::new(),
        })
    }
}
