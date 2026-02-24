//! Application state for crab-cloud (merged with crab-auth)

use aws_sdk_s3::Client as S3Client;
use aws_sdk_secretsmanager::Client as SmClient;
use crab_cert::{CaProfile, CertificateAuthority};
use dashmap::DashMap;
use shared::cloud::CloudMessage;
use shared::cloud::ws::CloudRpcResult;
use sqlx::PgPool;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use tokio::sync::{OnceCell, mpsc, oneshot};

use crate::auth::QuotaCache;
use crate::config::Config;
use crate::live::LiveOrderHub;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Stripe 相关配置
#[derive(Clone)]
pub struct StripeConfig {
    pub secret_key: String,
    pub webhook_secret: String,
    pub basic_price_id: String,
    pub pro_price_id: String,
    pub basic_yearly_price_id: String,
    pub pro_yearly_price_id: String,
}

/// S3 + 更新下载配置
#[derive(Clone)]
pub struct S3Config {
    pub client: S3Client,
    pub bucket: String,
    pub download_base_url: String,
}

/// Edge 连接管理
#[derive(Clone)]
pub struct EdgeConnections {
    pub connected: Arc<DashMap<i64, mpsc::Sender<CloudMessage>>>,
    pub pending_rpcs: Arc<DashMap<String, (i64, oneshot::Sender<CloudRpcResult>)>>,
}

impl EdgeConnections {
    pub fn new() -> Self {
        Self {
            connected: Arc::new(DashMap::new()),
            pending_rpcs: Arc::new(DashMap::new()),
        }
    }
}

/// Shared application state
#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    pub pool: PgPool,
    pub ca_store: CaStore,
    pub root_ca_pem: String,
    pub email: crate::email::EmailService,
    pub sm: SmClient,
    pub console_base_url: String,
    pub jwt_secret: String,
    pub quota_cache: QuotaCache,
    pub rate_limiter: crate::auth::rate_limit::RateLimiter,
    pub stripe: StripeConfig,
    pub s3: S3Config,
    pub edges: EdgeConnections,
    pub live_orders: LiveOrderHub,
    /// Console WS connections per tenant (tenant_id → count)
    pub console_connections: Arc<DashMap<String, AtomicUsize>>,
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
        let s3_client = S3Client::new(&aws_config);

        let email = crate::email::EmailService::new(
            config.resend_api_key.clone(),
            config.email_from.clone(),
        );

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
            email,
            sm: sm_client,
            console_base_url: config.console_base_url.clone(),
            jwt_secret: config.jwt_secret.clone(),
            quota_cache: QuotaCache::new(),
            rate_limiter: crate::auth::rate_limit::RateLimiter::new(),
            stripe: StripeConfig {
                secret_key: config.stripe_secret_key.clone(),
                webhook_secret: config.stripe_webhook_secret.clone(),
                basic_price_id: config.stripe_basic_price_id.clone(),
                pro_price_id: config.stripe_pro_price_id.clone(),
                basic_yearly_price_id: config.stripe_basic_yearly_price_id.clone(),
                pro_yearly_price_id: config.stripe_pro_yearly_price_id.clone(),
            },
            s3: S3Config {
                client: s3_client,
                bucket: config.update_s3_bucket.clone(),
                download_base_url: config.update_download_base_url.clone(),
            },
            edges: EdgeConnections::new(),
            live_orders: LiveOrderHub::new(),
            console_connections: Arc::new(DashMap::new()),
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
    /// Tenant CA in-process cache (tenant_id → CaSecret, never changes after creation)
    tenant_ca_cache: Arc<DashMap<String, CaSecret>>,
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
            tenant_ca_cache: Arc::new(DashMap::new()),
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

    /// Get or create Tenant CA (cached in-process — CAs never change after creation)
    pub async fn get_or_create_tenant_ca(
        &self,
        tenant_id: &str,
        root_ca: &CertificateAuthority,
    ) -> Result<CertificateAuthority, BoxError> {
        if let Some(cached) = self.tenant_ca_cache.get(tenant_id) {
            return Ok(CertificateAuthority::load(
                &cached.cert_pem,
                &cached.key_pem,
            )?);
        }

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

        self.tenant_ca_cache
            .insert(tenant_id.to_string(), secret.clone());
        Ok(CertificateAuthority::load(
            &secret.cert_pem,
            &secret.key_pem,
        )?)
    }

    /// Load existing Tenant CA (cached, errors if not found)
    pub async fn load_tenant_ca(&self, tenant_id: &str) -> Result<CertificateAuthority, BoxError> {
        if let Some(cached) = self.tenant_ca_cache.get(tenant_id) {
            return Ok(CertificateAuthority::load(
                &cached.cert_pem,
                &cached.key_pem,
            )?);
        }

        let secret_name = format!("crab/tenant/{tenant_id}");
        let secret = self
            .read_secret(&secret_name)
            .await?
            .ok_or_else(|| format!("Tenant CA not found for {tenant_id}"))?;

        self.tenant_ca_cache
            .insert(tenant_id.to_string(), secret.clone());
        Ok(CertificateAuthority::load(
            &secret.cert_pem,
            &secret.key_pem,
        )?)
    }

    /// Load Tenant CA cert PEM only (cached, for mTLS verification in edge_auth)
    pub async fn load_tenant_ca_cert(&self, tenant_id: &str) -> Result<String, BoxError> {
        if let Some(cached) = self.tenant_ca_cache.get(tenant_id) {
            return Ok(cached.cert_pem.clone());
        }

        let secret_name = format!("crab/tenant/{tenant_id}");
        let output = self
            .sm
            .get_secret_value()
            .secret_id(&secret_name)
            .send()
            .await?;

        let json = output.secret_string().ok_or("Secret has no string value")?;
        let secret: CaSecret = serde_json::from_str(json)?;
        let pem = secret.cert_pem.clone();
        self.tenant_ca_cache.insert(tenant_id.to_string(), secret);
        Ok(pem)
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
