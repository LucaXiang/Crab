use crab_cert::{CaProfile, CertificateAuthority, KeyType};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct AppState {
    pub cert_store: CertStore,
    pub user_store: UserStore,
    pub jwt_secret: String,
}

pub struct CertStore {
    root_path: PathBuf,
}

impl CertStore {
    pub fn new(root_path: PathBuf) -> Self {
        Self { root_path }
    }

    pub fn get_or_create_root_ca(&self) -> anyhow::Result<CertificateAuthority> {
        // Try to load from workspace crab-cert (for Dev/Test consistency)
        // Check both relative to crate root and workspace root
        let possible_paths = vec![
            (
                "../crab-cert/certs/root_ca.pem",
                "../crab-cert/certs/root_key.pem",
            ),
            (
                "crab-cert/certs/root_ca.pem",
                "crab-cert/certs/root_key.pem",
            ),
        ];

        for (cert, key) in possible_paths {
            let cert_path = PathBuf::from(cert);
            let key_path = PathBuf::from(key);
            if cert_path.exists() && key_path.exists() {
                info!("Loading Workspace Root CA from {:?}", cert_path);
                return CertificateAuthority::load_from_file(&cert_path, &key_path)
                    .map_err(|e| anyhow::anyhow!("Failed to load Workspace Root CA: {}", e));
            }
        }

        let ca_dir = self.root_path.join("ca");
        if !ca_dir.exists() {
            std::fs::create_dir_all(&ca_dir)?;
        }

        let cert_path = ca_dir.join("root_ca.crt");
        let key_path = ca_dir.join("root_ca.key");

        if cert_path.exists() && key_path.exists() {
            info!("Loading existing Root CA from {:?}", cert_path);
            CertificateAuthority::load_from_file(&cert_path, &key_path)
                .map_err(|e| anyhow::anyhow!("Failed to load Root CA: {}", e))
        } else {
            info!("Creating new Root CA");
            let mut profile = CaProfile::default();
            profile.common_name = "Crab Global Root CA".to_string();
            profile.organization = "Crab Inc.".to_string();
            profile.validity_days = 365 * 10;
            profile.key_type = KeyType::P256;

            let ca = CertificateAuthority::new_root(profile)
                .map_err(|e| anyhow::anyhow!("Failed to create Root CA: {}", e))?;

            ca.save(&ca_dir, "root_ca")
                .map_err(|e| anyhow::anyhow!("Failed to save Root CA: {}", e))?;

            Ok(ca)
        }
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
    // Mock user store: username -> password
    users: Arc<RwLock<std::collections::HashMap<String, String>>>,
}

impl UserStore {
    pub fn new() -> Self {
        let mut users = std::collections::HashMap::new();
        // Default admin user
        users.insert("admin".to_string(), "admin123".to_string());
        Self {
            users: Arc::new(RwLock::new(users)),
        }
    }

    pub async fn authenticate(&self, username: &str, password: &str) -> bool {
        let users = self.users.read().await;
        if let Some(stored_pass) = users.get(username) {
            return stored_pass == password;
        }
        false
    }
}
