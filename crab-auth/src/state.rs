use crab_cert::CertificateAuthority;
use sqlx::PgPool;
use std::path::PathBuf;

pub struct AppState {
    pub db: PgPool,
    pub auth_storage: AuthStorage,
    pub s3: aws_sdk_s3::Client,
    pub s3_bucket: String,
    pub kms_key_id: Option<String>,
}

pub struct AuthStorage {
    root_path: PathBuf,
}

impl AuthStorage {
    pub fn new(root_path: PathBuf) -> Self {
        Self { root_path }
    }

    pub fn get_or_create_root_ca(
        &self,
    ) -> Result<CertificateAuthority, Box<dyn std::error::Error>> {
        let ca_dir = self.root_path.join("ca");
        if !ca_dir.exists() {
            std::fs::create_dir_all(&ca_dir)?;
        }
        crab_cert::trust::get_or_create_root_ca(&ca_dir)
            .map_err(|e| format!("Failed to get or create Root CA: {e}").into())
    }

    pub fn get_tenant_dir(&self, tenant_id: &str) -> Result<PathBuf, std::io::Error> {
        // Path traversal 防御
        if tenant_id.contains('/')
            || tenant_id.contains('\\')
            || tenant_id.contains("..")
            || tenant_id.is_empty()
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid tenant_id",
            ));
        }

        let path = self.root_path.join("tenants").join(tenant_id);
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }
        Ok(path)
    }
}
