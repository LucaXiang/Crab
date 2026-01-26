use crab_cert::CertificateAuthority;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct AppState {
    pub auth_storage: AuthStorage,
    pub user_store: UserStore,
    pub jwt_secret: String,
    pub revocation_store: RevocationStore,
}

/// 实体撤销存储
///
/// 存储被撤销的实体 ID，防止已禁用设备继续刷新 binding
pub struct RevocationStore {
    /// tenant_id -> Set<entity_id>
    revoked: Arc<RwLock<HashMap<String, HashSet<String>>>>,
}

impl RevocationStore {
    pub fn new() -> Self {
        Self {
            revoked: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 检查实体是否被撤销
    pub async fn is_revoked(&self, tenant_id: &str, entity_id: &str) -> bool {
        let revoked = self.revoked.read().await;
        revoked
            .get(tenant_id)
            .is_some_and(|entities| entities.contains(entity_id))
    }

    /// 撤销实体
    pub async fn revoke(&self, tenant_id: &str, entity_id: &str) {
        let mut revoked = self.revoked.write().await;
        revoked
            .entry(tenant_id.to_string())
            .or_default()
            .insert(entity_id.to_string());
        info!("🚫 Revoked entity={} for tenant={}", entity_id, tenant_id);
    }

    /// 恢复实体
    pub async fn restore(&self, tenant_id: &str, entity_id: &str) -> bool {
        let mut revoked = self.revoked.write().await;
        if let Some(entities) = revoked.get_mut(tenant_id) {
            let removed = entities.remove(entity_id);
            if removed {
                info!("✅ Restored entity={} for tenant={}", entity_id, tenant_id);
            }
            removed
        } else {
            false
        }
    }

    /// 获取租户的所有已撤销实体
    pub async fn list_revoked(&self, tenant_id: &str) -> Vec<String> {
        let revoked = self.revoked.read().await;
        revoked
            .get(tenant_id)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }
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
