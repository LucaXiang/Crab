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
use crate::crypto::MasterKey;
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
    pub console_base_url: String,
    pub jwt_secret: String,
    pub quota_cache: QuotaCache,
    pub rate_limiter: crate::auth::rate_limit::RateLimiter,
    pub stripe: StripeConfig,
    pub s3: S3Config,
    pub master_key: Arc<MasterKey>,
    pub edges: EdgeConnections,
    pub live_orders: LiveOrderHub,
    /// Console WS connections per tenant (tenant_id → count)
    pub console_connections: Arc<DashMap<String, AtomicUsize>>,
}

impl AppState {
    /// Create a new AppState
    pub async fn new(config: &Config) -> Result<Self, BoxError> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(50)
            .min_connections(2)
            .acquire_timeout(std::time::Duration::from_secs(5))
            .connect(&config.database_url)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let sm_client = SmClient::new(&aws_config);
        let s3_client = S3Client::new(&aws_config);

        let email = crate::email::EmailService::new(
            config.resend_api_key.clone(),
            config.email_from.clone(),
        );

        let master_key = Arc::new(MasterKey::from_secrets_manager(&sm_client).await?);

        let ca_store = CaStore::new(sm_client, pool.clone(), master_key.clone());

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
            master_key,
            edges: EdgeConnections::new(),
            live_orders: LiveOrderHub::new(),
            console_connections: Arc::new(DashMap::new()),
        })
    }
}

/// Certificate Authority 存储
///
/// Root CA 从 Secrets Manager 读写，Tenant CA 从 PostgreSQL 读写。
/// 内存缓存：Root CA 和 Tenant CA 创建后不变，缓存在进程内。
#[derive(Clone)]
pub struct CaStore {
    sm: SmClient,
    pool: PgPool,
    master_key: Arc<MasterKey>,
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
    pub fn new(sm: SmClient, pool: PgPool, master_key: Arc<MasterKey>) -> Self {
        Self {
            sm,
            pool,
            master_key,
            root_ca_cache: std::sync::Arc::new(OnceCell::new()),
            tenant_ca_cache: Arc::new(DashMap::new()),
        }
    }

    /// Get or create Root CA (cached in-process, stored in Secrets Manager)
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

    /// Get or create Tenant CA (cached in-process, stored in PostgreSQL)
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

        // 从 PostgreSQL 读取 tenant CA (key is encrypted)
        let secret = match sqlx::query_as::<_, (String, String)>(
            "SELECT ca_cert_pem, ca_key_encrypted FROM tenants WHERE id = $1 AND ca_cert_pem IS NOT NULL",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        {
            Some((cert_pem, key_encrypted)) => {
                let key_pem = self.master_key.decrypt_string(&key_encrypted)
                    .map_err(|e| format!("Failed to decrypt tenant CA key: {e}"))?;
                CaSecret { cert_pem, key_pem }
            }
            None => {
                // 创建新 Tenant CA 并写入 PostgreSQL (key encrypted)
                let profile = CaProfile::intermediate(tenant_id, &format!("Tenant {tenant_id}"));
                let ca = CertificateAuthority::new_intermediate(profile, root_ca)?;
                let key_encrypted = self.master_key.encrypt_string(&ca.key_pem())
                    .map_err(|e| format!("Failed to encrypt tenant CA key: {e}"))?;
                let s = CaSecret {
                    cert_pem: ca.cert_pem().to_string(),
                    key_pem: ca.key_pem(),
                };
                sqlx::query("UPDATE tenants SET ca_cert_pem = $1, ca_key_encrypted = $2 WHERE id = $3")
                    .bind(&s.cert_pem)
                    .bind(&key_encrypted)
                    .bind(tenant_id)
                    .execute(&self.pool)
                    .await?;
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

    /// Load existing Tenant CA (cached, errors if not found, reads from PostgreSQL)
    pub async fn load_tenant_ca(&self, tenant_id: &str) -> Result<CertificateAuthority, BoxError> {
        if let Some(cached) = self.tenant_ca_cache.get(tenant_id) {
            return Ok(CertificateAuthority::load(
                &cached.cert_pem,
                &cached.key_pem,
            )?);
        }

        let (cert_pem, key_encrypted) = sqlx::query_as::<_, (String, String)>(
            "SELECT ca_cert_pem, ca_key_encrypted FROM tenants WHERE id = $1 AND ca_cert_pem IS NOT NULL",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| format!("Tenant CA not found for {tenant_id}"))?;

        let key_pem = self
            .master_key
            .decrypt_string(&key_encrypted)
            .map_err(|e| format!("Failed to decrypt tenant CA key: {e}"))?;

        let secret = CaSecret { cert_pem, key_pem };
        self.tenant_ca_cache
            .insert(tenant_id.to_string(), secret.clone());
        Ok(CertificateAuthority::load(
            &secret.cert_pem,
            &secret.key_pem,
        )?)
    }

    /// Load Tenant CA cert PEM only (cached, for mTLS verification in edge_auth, reads from PostgreSQL)
    pub async fn load_tenant_ca_cert(&self, tenant_id: &str) -> Result<String, BoxError> {
        if let Some(cached) = self.tenant_ca_cache.get(tenant_id) {
            return Ok(cached.cert_pem.clone());
        }

        let (cert_pem, key_encrypted) = sqlx::query_as::<_, (String, String)>(
            "SELECT ca_cert_pem, ca_key_encrypted FROM tenants WHERE id = $1 AND ca_cert_pem IS NOT NULL",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| format!("Tenant CA cert not found for {tenant_id}"))?;

        let key_pem = self
            .master_key
            .decrypt_string(&key_encrypted)
            .map_err(|e| format!("Failed to decrypt tenant CA key: {e}"))?;

        let pem = cert_pem.clone();
        self.tenant_ca_cache
            .insert(tenant_id.to_string(), CaSecret { cert_pem, key_pem });
        Ok(pem)
    }

    /// Root CA 初始化（从 Secrets Manager 读取或创建）
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

    /// 从 Secrets Manager 读取 secret（仅 Root CA 使用）
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

    /// 在 Secrets Manager 创建 secret（仅 Root CA 使用）
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
