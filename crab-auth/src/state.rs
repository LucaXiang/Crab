use crab_cert::CertificateAuthority;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct AppState {
    pub auth_storage: AuthStorage,
    pub user_store: UserStore,
    pub jwt_secret: String,
}

pub struct AuthStorage {
    root_path: PathBuf,
}

impl AuthStorage {
    pub fn new(root_path: PathBuf) -> Self {
        Self { root_path }
    }

    pub fn get_or_create_root_ca(&self) -> anyhow::Result<CertificateAuthority> {
        let ca_dir = self.root_path.join("ca");
        if !ca_dir.exists() {
            std::fs::create_dir_all(&ca_dir)?;
        }

        info!("Loading/Creating Root CA in {:?}", ca_dir);
        crab_cert::trust::get_or_create_root_ca(&ca_dir)
            .map_err(|e| anyhow::anyhow!("Failed to get or create Root CA: {}", e))
    }

    pub fn get_tenant_dir(&self, tenant_id: &str) -> PathBuf {
        let path = self.root_path.join("tenants").join(tenant_id);
        if !path.exists() {
            std::fs::create_dir_all(&path).unwrap_or_default();
        }
        path
    }
}

pub struct UserStore {
    // Mock user store: username -> (password, tenant_id)
    users: Arc<RwLock<std::collections::HashMap<String, (String, String)>>>,
}

impl UserStore {
    pub fn new() -> Self {
        let mut users = std::collections::HashMap::new();
        // Default admin user
        users.insert(
            "admin".to_string(),
            ("admin123".to_string(), "tenant-1".to_string()),
        );
        Self {
            users: Arc::new(RwLock::new(users)),
        }
    }

    pub async fn authenticate(&self, username: &str, password: &str) -> Option<String> {
        let users = self.users.read().await;
        users
            .get(username)
            .filter(|(stored_pass, _)| stored_pass == password)
            .map(|(_, tenant_id)| tenant_id.clone())
    }
}
