//! Application state for crab-cloud (merged with crab-auth)

use aws_sdk_s3::Client as S3Client;
use aws_sdk_secretsmanager::Client as SmClient;
use aws_sdk_sesv2::Client as SesClient;
use crab_cert::{CaProfile, CertificateAuthority};
use sqlx::PgPool;
use tokio::sync::OnceCell;

use crate::auth::QuotaCache;
use crate::config::Config;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Shared application state
#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    /// PostgreSQL connection pool
    pub pool: PgPool,
    /// Full CA store (PKI operations: create/load Root CA, Tenant CA, sign)
    pub ca_store: CaStore,
    /// Root CA PEM for mTLS verification
    pub root_ca_pem: String,
    /// AWS SES client for sending emails
    pub ses: SesClient,
    /// AWS Secrets Manager client (for P12 password storage)
    pub sm: SmClient,
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
    /// AWS S3 client (update artifacts)
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
    /// Stripe Price ID for Basic plan (checkout session)
    pub stripe_basic_price_id: String,
}

impl AppState {
    /// Store P12 binary (base64) + password in Secrets Manager (create or update)
    pub async fn store_p12_secret(
        &self,
        tenant_id: &str,
        p12_data: &[u8],
        password: &str,
    ) -> Result<String, BoxError> {
        use base64::Engine;
        let p12_base64 = base64::engine::general_purpose::STANDARD.encode(p12_data);
        let secret_value = serde_json::json!({
            "p12_base64": p12_base64,
            "password": password,
        })
        .to_string();

        let secret_name = format!("crab/p12/{tenant_id}");
        match self
            .sm
            .put_secret_value()
            .secret_id(&secret_name)
            .secret_string(&secret_value)
            .send()
            .await
        {
            Ok(_) => Ok(secret_name),
            Err(err)
                if err
                    .as_service_error()
                    .is_some_and(|e| e.is_resource_not_found_exception()) =>
            {
                self.sm
                    .create_secret()
                    .name(&secret_name)
                    .secret_string(&secret_value)
                    .send()
                    .await?;
                Ok(secret_name)
            }
            Err(err) => Err(err.into()),
        }
    }

    /// Create a new AppState
    pub async fn new(config: &Config) -> Result<Self, BoxError> {
        let pool = PgPool::connect(&config.database_url).await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let sm_client = SmClient::new(&aws_config);
        let s3 = S3Client::new(&aws_config);

        let ses = if let Ok(ses_region) = std::env::var("SES_REGION") {
            let ses_config = aws_config
                .to_builder()
                .region(aws_config::Region::new(ses_region))
                .build();
            SesClient::new(&ses_config)
        } else {
            SesClient::new(&aws_config)
        };

        let ca_store = CaStore::new(sm_client.clone());

        // Verify Root CA is accessible (warm cache)
        ca_store.get_or_create_root_ca().await?;
        tracing::info!("Root CA ready");

        // Load Root CA PEM for mTLS verification
        let root_ca_pem = if let Some(ref pem) = config.root_ca_pem {
            pem.clone()
        } else {
            match std::fs::read_to_string(&config.root_ca_path) {
                Ok(pem) => pem,
                Err(_) => {
                    tracing::info!(
                        "Root CA file not found at {}, loading from Secrets Manager",
                        config.root_ca_path
                    );
                    ca_store
                        .get_or_create_root_ca()
                        .await?
                        .cert_pem()
                        .to_string()
                }
            }
        };

        Ok(Self {
            pool,
            ca_store,
            root_ca_pem,
            ses,
            sm: sm_client,
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
            stripe_basic_price_id: config.stripe_basic_price_id.clone(),
        })
    }
}

/// Full Certificate Authority store (merged from crab-auth)
///
/// Supports both read-only operations (mTLS verification) and
/// PKI operations (Root CA / Tenant CA creation, cert signing).
#[derive(Clone)]
pub struct CaStore {
    sm: SmClient,
    /// Root CA in-process cache (cert + key, never changes after creation)
    root_ca_cache: std::sync::Arc<OnceCell<CaSecret>>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct CaSecret {
    cert_pem: String,
    key_pem: String,
}

impl CaStore {
    pub fn new(sm: SmClient) -> Self {
        Self {
            sm,
            root_ca_cache: std::sync::Arc::new(OnceCell::new()),
        }
    }

    /// Get or create Root CA (cached in-process)
    pub async fn get_or_create_root_ca(&self) -> Result<CertificateAuthority, BoxError> {
        let secret = self
            .root_ca_cache
            .get_or_try_init(|| self.init_root_ca())
            .await?;
        Ok(CertificateAuthority::load(
            &secret.cert_pem,
            &secret.key_pem,
        )?)
    }

    /// Get or create Tenant CA (reads from Secrets Manager each time)
    pub async fn get_or_create_tenant_ca(
        &self,
        tenant_id: &str,
        root_ca: &CertificateAuthority,
    ) -> Result<CertificateAuthority, BoxError> {
        let secret_name = format!("crab/tenant/{tenant_id}");
        let secret = match self.read_secret(&secret_name).await? {
            Some(s) => s,
            None => {
                let profile = CaProfile::intermediate(tenant_id, &format!("Tenant {tenant_id}"));
                let ca = CertificateAuthority::new_intermediate(profile, root_ca)?;
                let s = CaSecret {
                    cert_pem: ca.cert_pem().to_string(),
                    key_pem: ca.key_pem(),
                };
                self.create_secret(&secret_name, &s).await?;
                s
            }
        };
        Ok(CertificateAuthority::load(
            &secret.cert_pem,
            &secret.key_pem,
        )?)
    }

    /// Load existing Tenant CA (errors if not found)
    pub async fn load_tenant_ca(&self, tenant_id: &str) -> Result<CertificateAuthority, BoxError> {
        let secret_name = format!("crab/tenant/{tenant_id}");
        let secret = self
            .read_secret(&secret_name)
            .await?
            .ok_or_else(|| format!("Tenant CA not found for {tenant_id}"))?;
        Ok(CertificateAuthority::load(
            &secret.cert_pem,
            &secret.key_pem,
        )?)
    }

    /// Load Tenant CA cert PEM only (for mTLS verification in edge_auth)
    pub async fn load_tenant_ca_cert(&self, tenant_id: &str) -> Result<String, BoxError> {
        let secret_name = format!("crab/tenant/{tenant_id}");
        let output = self
            .sm
            .get_secret_value()
            .secret_id(&secret_name)
            .send()
            .await?;

        let json = output.secret_string().ok_or("Secret has no string value")?;
        let secret: CaSecret = serde_json::from_str(json)?;
        Ok(secret.cert_pem)
    }

    async fn init_root_ca(&self) -> Result<CaSecret, BoxError> {
        match self.read_secret("crab/root-ca").await? {
            Some(s) => Ok(s),
            None => {
                let ca = CertificateAuthority::new_root(CaProfile::root("Crab Root CA"))?;
                let s = CaSecret {
                    cert_pem: ca.cert_pem().to_string(),
                    key_pem: ca.key_pem(),
                };
                self.create_secret("crab/root-ca", &s).await?;
                Ok(s)
            }
        }
    }

    async fn read_secret(&self, name: &str) -> Result<Option<CaSecret>, BoxError> {
        match self.sm.get_secret_value().secret_id(name).send().await {
            Ok(output) => {
                let json = output.secret_string().ok_or("Secret has no string value")?;
                Ok(Some(serde_json::from_str(json)?))
            }
            Err(err) => {
                if err
                    .as_service_error()
                    .is_some_and(|e| e.is_resource_not_found_exception())
                {
                    Ok(None)
                } else {
                    Err(err.into())
                }
            }
        }
    }

    async fn create_secret(&self, name: &str, secret: &CaSecret) -> Result<(), BoxError> {
        let json = serde_json::to_string(secret)?;
        self.sm
            .create_secret()
            .name(name)
            .secret_string(json)
            .send()
            .await?;
        Ok(())
    }
}
