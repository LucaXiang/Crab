use aws_sdk_secretsmanager::Client as SmClient;
use crab_cert::{CaProfile, CertificateAuthority};
use sqlx::PgPool;
use tokio::sync::OnceCell;

pub struct AppState {
    pub db: PgPool,
    pub ca_store: CaStore,
    pub sm: SmClient,
    pub s3: aws_sdk_s3::Client,
    pub s3_bucket: String,
    pub kms_key_id: Option<String>,
}

impl AppState {
    /// 将 P12 密码存入 Secrets Manager (create or update)
    pub async fn store_p12_password(
        &self,
        tenant_id: &str,
        password: &str,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let secret_name = format!("crab-auth/p12/{tenant_id}");

        // 尝试更新已有 secret
        match self
            .sm
            .put_secret_value()
            .secret_id(&secret_name)
            .secret_string(password)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(err)
                if err
                    .as_service_error()
                    .is_some_and(|e| e.is_resource_not_found_exception()) =>
            {
                // Secret 不存在，创建新的
                self.sm
                    .create_secret()
                    .name(&secret_name)
                    .secret_string(password)
                    .send()
                    .await?;
                Ok(())
            }
            Err(err) => Err(err.into()),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CaSecret {
    cert_pem: String,
    key_pem: String,
}

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub struct CaStore {
    sm: SmClient,
    /// Root CA in-process cache — never changes after first creation
    root_ca_cache: OnceCell<CaSecret>,
}

impl CaStore {
    pub fn new(sm: SmClient) -> Self {
        Self {
            sm,
            root_ca_cache: OnceCell::new(),
        }
    }

    /// Get or create Root CA (cached in-process for Lambda warm starts)
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
        let secret_name = format!("crab-auth/tenant/{tenant_id}");
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
        let secret_name = format!("crab-auth/tenant/{tenant_id}");
        let secret = self
            .read_secret(&secret_name)
            .await?
            .ok_or_else(|| format!("Tenant CA not found for {tenant_id}"))?;
        Ok(CertificateAuthority::load(
            &secret.cert_pem,
            &secret.key_pem,
        )?)
    }

    async fn init_root_ca(&self) -> Result<CaSecret, BoxError> {
        match self.read_secret("crab-auth/root-ca").await? {
            Some(s) => Ok(s),
            None => {
                let ca = CertificateAuthority::new_root(CaProfile::root("Crab Root CA"))?;
                let s = CaSecret {
                    cert_pem: ca.cert_pem().to_string(),
                    key_pem: ca.key_pem(),
                };
                self.create_secret("crab-auth/root-ca", &s).await?;
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
