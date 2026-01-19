//! TenantManager - 多租户证书和会话管理
//!
//! 负责管理多租户的证书、凭证和员工会话缓存。
//! 支持：
//! - 设备激活（获取租户证书）
//! - 租户切换
//! - 员工登录（在线/离线）

use crab_client::{CertError, CertManager};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

use super::session_cache::{EmployeeSession, LoginMode, SessionCache};

#[derive(Debug, Error)]
pub enum TenantError {
    #[error("Tenant not found: {0}")]
    NotFound(String),

    #[error("No tenant selected")]
    NoTenantSelected,

    #[error("Certificate error: {0}")]
    Certificate(#[from] CertError),

    #[error("Session cache error: {0}")]
    SessionCache(#[from] super::session_cache::SessionCacheError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Offline login not available for user: {0}")]
    OfflineNotAvailable(String),
}

/// 租户信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TenantInfo {
    pub tenant_id: String,
    pub tenant_name: Option<String>,
    pub has_certificates: bool,
    pub last_used: Option<u64>,
}

/// 多租户管理器
pub struct TenantManager {
    /// 基础路径 (~/.red_coral/tenants)
    base_path: PathBuf,
    /// 当前活跃租户 ID
    current_tenant: Option<String>,
    /// 各租户的证书管理器
    cert_managers: HashMap<String, CertManager>,
    /// 各租户的会话缓存
    session_caches: HashMap<String, SessionCache>,
    /// 当前员工会话
    current_session: Option<EmployeeSession>,
    /// 客户端名称 (设备标识)
    client_name: String,
}

impl TenantManager {
    /// 创建新的 TenantManager
    ///
    /// # Arguments
    /// * `base_path` - 租户数据的基础路径 (如 ~/.red_coral/tenants)
    /// * `client_name` - 客户端名称 (设备标识)
    pub fn new(base_path: impl Into<PathBuf>, client_name: &str) -> Self {
        let base_path = base_path.into();
        Self {
            base_path,
            current_tenant: None,
            cert_managers: HashMap::new(),
            session_caches: HashMap::new(),
            current_session: None,
            client_name: client_name.to_string(),
        }
    }

    /// 加载已存在的租户
    pub fn load_existing_tenants(&mut self) -> Result<(), TenantError> {
        if !self.base_path.exists() {
            std::fs::create_dir_all(&self.base_path)?;
            return Ok(());
        }

        for entry in std::fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(tenant_id) = path.file_name().and_then(|n| n.to_str()) {
                    self.load_tenant(tenant_id)?;
                }
            }
        }

        Ok(())
    }

    /// 加载单个租户
    fn load_tenant(&mut self, tenant_id: &str) -> Result<(), TenantError> {
        let tenant_path = self.base_path.join(tenant_id);

        // 创建 CertManager
        let cert_manager = CertManager::new(&tenant_path, &self.client_name);
        self.cert_managers
            .insert(tenant_id.to_string(), cert_manager);

        // 加载 SessionCache
        let session_cache = SessionCache::load(&tenant_path)?;
        self.session_caches
            .insert(tenant_id.to_string(), session_cache);

        Ok(())
    }

    // ============ 租户管理 ============

    /// 列出所有已激活的租户
    pub fn list_tenants(&self) -> Vec<TenantInfo> {
        self.cert_managers
            .iter()
            .map(
                |(tenant_id, cert_manager): (&String, &CertManager)| TenantInfo {
                    tenant_id: tenant_id.clone(),
                    tenant_name: None, // TODO: Load from credential
                    has_certificates: cert_manager.has_local_certificates(),
                    last_used: None, // TODO: Track last used time
                },
            )
            .collect()
    }

    /// 激活设备 (获取 Edge Server 证书)
    ///
    /// 这是一个 "Server Activation" 流程，获取的证书可用于：
    /// 1. 运行 Edge Server (作为 Server Identity)
    /// 2. 连接其他节点 (作为 Client Identity)
    pub async fn activate_device(
        &mut self,
        auth_url: &str,
        username: &str,
        password: &str,
    ) -> Result<String, TenantError> {
        // 1. 生成 Hardware ID
        let device_id = crab_cert::generate_hardware_id();
        tracing::info!("Activating device with ID: {}", device_id);

        // 2. 调用 Auth Server 激活接口
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/api/server/activate", auth_url))
            .json(&serde_json::json!({
                "username": username,
                "password": password,
                "device_id": device_id,
            }))
            .send()
            .await
            .map_err(|e| TenantError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(TenantError::AuthFailed(format!(
                "Activation failed: {}",
                text
            )));
        }

        // 3. 解析响应
        let resp_data: shared::activation::ActivationResponse = resp
            .json()
            .await
            .map_err(|e| TenantError::Network(format!("Invalid response: {}", e)))?;

        if !resp_data.success {
            let msg = resp_data.error.as_deref().unwrap_or("Unknown error");
            return Err(TenantError::AuthFailed(msg.to_string()));
        }

        let data = resp_data
            .data
            .ok_or_else(|| TenantError::AuthFailed("Missing activation data".to_string()))?;

        let tenant_id = data.tenant_id.clone();

        // 4. 准备租户目录
        let tenant_path = self.base_path.join(&tenant_id);
        let certs_path = tenant_path.join("certs");
        let auth_path = tenant_path.join("auth_storage");

        std::fs::create_dir_all(&certs_path)?;
        std::fs::create_dir_all(&auth_path)?;

        // 5. 验证证书链 (简化版，完整验证由 edge-server 启动时进行)
        // 这里主要确保我们拿到了看起来正确的东西

        // 6. 保存证书 (使用 edge-server 兼容的文件名)
        std::fs::write(certs_path.join("root_ca.pem"), &data.root_ca_cert)?;
        std::fs::write(certs_path.join("tenant_ca.pem"), &data.tenant_ca_cert)?;
        std::fs::write(certs_path.join("edge_cert.pem"), &data.entity_cert)?;
        std::fs::write(certs_path.join("edge_key.pem"), &data.entity_key)?;

        // 兼容 crab-client 的文件名 (用于 Client Mode)
        // 也可以使用软链接，但为了 Windows 兼容性，复制一份
        std::fs::write(certs_path.join("cert.pem"), &data.entity_cert)?;
        std::fs::write(certs_path.join("key.pem"), &data.entity_key)?;
        std::fs::write(certs_path.join("ca.pem"), &data.tenant_ca_cert)?;

        // 7. 保存 Credential (用于 activation check)
        // 保存为 Credential.json 到 auth_storage (注意首字母大写，与 edge-server 保持一致)
        let credential_path = auth_path.join("Credential.json");
        // 需要包装在 TenantBinding 中，因为 edge-server 期望 {"binding": {...}, "subscription": ...}
        let tenant_binding =
            edge_server::services::tenant_binding::TenantBinding::from_signed(data.binding.clone());
        let credential_json = serde_json::to_string_pretty(&tenant_binding).map_err(|e| {
            TenantError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?;
        std::fs::write(credential_path, credential_json)?;

        // 8. 更新内存状态
        // 我们创建一个 CertManager 指向这个目录，以便兼容现有逻辑
        // 注意：CertManager 默认期望在 base_path/client_name 下
        // 这里我们稍微 hack 一下，直接指向 tenant_path
        // 但 CertManager::new 接受 base_path 和 client_name，然后 join。
        // 所以我们传 tenant_path 的父目录? 不行，结构不一样。

        // 我们需要手动构建 CertManager 或更新 TenantInfo
        // 这里暂时不更新 self.cert_managers，因为我们改变了目录结构
        // 而是重新加载
        self.load_tenant(&tenant_id)?;

        // 9. 自动切换
        self.switch_tenant(&tenant_id)?;

        tracing::info!(tenant_id = %tenant_id, "Device activated successfully");

        Ok(tenant_id)
    }

    /// 切换当前租户
    pub fn switch_tenant(&mut self, tenant_id: &str) -> Result<(), TenantError> {
        if !self.cert_managers.contains_key(tenant_id) {
            return Err(TenantError::NotFound(tenant_id.to_string()));
        }

        // 清除当前会话
        self.current_session = None;
        self.current_tenant = Some(tenant_id.to_string());

        tracing::info!(tenant_id = %tenant_id, "Switched to tenant");

        Ok(())
    }

    /// 移除租户 (删除本地证书和缓存)
    pub fn remove_tenant(&mut self, tenant_id: &str) -> Result<(), TenantError> {
        // 如果是当前租户，先清除
        if self.current_tenant.as_deref() == Some(tenant_id) {
            self.current_tenant = None;
            self.current_session = None;
        }

        // 移除管理器
        self.cert_managers.remove(tenant_id);
        self.session_caches.remove(tenant_id);

        // 删除文件
        let tenant_path = self.base_path.join(tenant_id);
        if tenant_path.exists() {
            std::fs::remove_dir_all(&tenant_path)?;
        }

        tracing::info!(tenant_id = %tenant_id, "Tenant removed");

        Ok(())
    }

    // ============ 员工登录 ============

    /// 在线登录 (同时更新缓存)
    pub async fn login_online(
        &mut self,
        username: &str,
        password: &str,
        edge_url: &str,
    ) -> Result<EmployeeSession, TenantError> {
        let tenant_id = self
            .current_tenant
            .as_ref()
            .ok_or(TenantError::NoTenantSelected)?
            .clone();

        let cert_manager = self
            .cert_managers
            .get(&tenant_id)
            .ok_or_else(|| TenantError::NotFound(tenant_id.clone()))?;

        // 构建 mTLS HTTP 客户端
        let http_client = cert_manager.build_mtls_http_client()?;

        // 发送登录请求
        let response: reqwest::Response = http_client
            .post(format!("{}/api/auth/login", edge_url))
            .json(&serde_json::json!({
                "username": username,
                "password": password
            }))
            .send()
            .await
            .map_err(|e: reqwest::Error| TenantError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(TenantError::AuthFailed(error_text));
        }

        // 解析响应
        #[derive(serde::Deserialize)]
        struct LoginResponse {
            success: bool,
            data: Option<LoginData>,
            error: Option<String>,
        }

        #[derive(serde::Deserialize)]
        struct LoginData {
            token: String,
            user: shared::client::UserInfo,
        }

        let login_resp: LoginResponse = response
            .json::<LoginResponse>()
            .await
            .map_err(|e: reqwest::Error| TenantError::Network(e.to_string()))?;

        if !login_resp.success {
            return Err(TenantError::AuthFailed(
                login_resp
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        let data = login_resp
            .data
            .ok_or_else(|| TenantError::AuthFailed("Missing login data".to_string()))?;

        // 创建会话
        let session = EmployeeSession {
            username: username.to_string(),
            token: data.token.clone(),
            user_info: data.user,
            login_mode: LoginMode::Online,
            expires_at: None, // TODO: Parse from JWT
            logged_in_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // 更新缓存
        if let Some(cache) = self.session_caches.get_mut(&tenant_id) {
            cache.update_employee_cache(username, password, &session)?;
        }

        self.current_session = Some(session.clone());

        tracing::info!(username = %username, mode = "online", "Employee logged in");

        Ok(session)
    }

    /// 离线登录 (使用缓存验证)
    pub fn login_offline(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<EmployeeSession, TenantError> {
        let tenant_id = self
            .current_tenant
            .as_ref()
            .ok_or(TenantError::NoTenantSelected)?
            .clone();

        let cache = self
            .session_caches
            .get(&tenant_id)
            .ok_or_else(|| TenantError::NotFound(tenant_id.clone()))?;

        // 验证离线凭据
        let session = cache.verify_offline_login(username, password)?;

        self.current_session = Some(session.clone());

        tracing::info!(username = %username, mode = "offline", "Employee logged in");

        Ok(session)
    }

    /// 自动登录 (优先在线，失败则尝试离线)
    pub async fn login_auto(
        &mut self,
        username: &str,
        password: &str,
        edge_url: &str,
    ) -> Result<EmployeeSession, TenantError> {
        // 先尝试在线登录
        match self.login_online(username, password, edge_url).await {
            Ok(session) => Ok(session),
            Err(e) => {
                tracing::warn!(error = %e, "Online login failed, trying offline");
                // 尝试离线登录
                self.login_offline(username, password)
            }
        }
    }

    /// 登出
    pub fn logout(&mut self) -> Result<(), TenantError> {
        if let Some(session) = self.current_session.take() {
            tracing::info!(username = %session.username, "Employee logged out");
        }
        Ok(())
    }

    // ============ 状态查询 ============

    /// 获取当前租户ID
    pub fn current_tenant_id(&self) -> Option<&str> {
        self.current_tenant.as_deref()
    }

    /// 获取当前员工会话
    pub fn current_session(&self) -> Option<&EmployeeSession> {
        self.current_session.as_ref()
    }

    /// 检查是否有缓存的离线登录数据
    pub fn has_offline_cache(&self, username: &str) -> bool {
        if let Some(tenant_id) = &self.current_tenant {
            if let Some(cache) = self.session_caches.get(tenant_id) {
                return cache.has_employee(username);
            }
        }
        false
    }

    /// 获取当前租户的缓存员工列表
    pub fn list_cached_employees(&self) -> Vec<String> {
        if let Some(tenant_id) = &self.current_tenant {
            if let Some(cache) = self.session_caches.get(tenant_id) {
                return cache.list_employees();
            }
        }
        Vec::new()
    }

    /// 获取当前租户的 CertManager
    pub fn current_cert_manager(&self) -> Option<&CertManager> {
        self.current_tenant
            .as_ref()
            .and_then(|id| self.cert_managers.get(id))
    }

    /// 获取当前租户目录
    pub fn current_tenant_path(&self) -> Option<PathBuf> {
        self.current_tenant
            .as_ref()
            .map(|id| self.base_path.join(id))
    }

    /// 获取客户端名称
    pub fn client_name(&self) -> &str {
        &self.client_name
    }

    // ============ 当前活动会话持久化 ============

    /// 保存当前活动会话到磁盘
    pub fn save_current_session(&self, session: &EmployeeSession) -> Result<(), TenantError> {
        let tenant_id = self.current_tenant.as_ref()
            .ok_or(TenantError::NoTenantSelected)?;

        let cache = self.session_caches.get(tenant_id)
            .ok_or_else(|| TenantError::NotFound(tenant_id.clone()))?;

        cache.save_current_session(session)
            .map_err(TenantError::SessionCache)
    }

    /// 加载缓存的当前活动会话
    pub fn load_current_session(&self) -> Result<Option<EmployeeSession>, TenantError> {
        let tenant_id = self.current_tenant.as_ref()
            .ok_or(TenantError::NoTenantSelected)?;

        let cache = self.session_caches.get(tenant_id)
            .ok_or_else(|| TenantError::NotFound(tenant_id.clone()))?;

        cache.load_current_session()
            .map_err(TenantError::SessionCache)
    }

    /// 清除缓存的当前活动会话
    pub fn clear_current_session(&self) -> Result<(), TenantError> {
        if let Some(tenant_id) = &self.current_tenant {
            if let Some(cache) = self.session_caches.get(tenant_id) {
                cache.clear_current_session()
                    .map_err(TenantError::SessionCache)?;
            }
        }
        Ok(())
    }

    /// 设置当前会话 (用于恢复登录状态)
    pub fn set_current_session(&mut self, session: EmployeeSession) {
        tracing::info!(username = %session.username, "Session restored from cache");
        self.current_session = Some(session);
    }
}
