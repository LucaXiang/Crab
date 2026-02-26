//! TenantManager - 多租户证书和会话管理
//!
//! 负责管理多租户的证书、凭证和员工会话缓存。
//! 支持：
//! - 设备激活（获取租户证书）
//! - 租户切换
//! - 员工登录（在线/离线）
//!
//! ## 路径管理
//!
//! 使用 `TenantPaths` 统一管理路径，不再依赖 `CertManager`。
//! - Server 模式: edge-server 自行处理证书，我们只检查文件存在性
//! - Client 模式: 路径与 CertManager 兼容，CrabClient 可直接使用

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

use super::paths::TenantPaths;
use super::session_cache::{EmployeeSession, LoginMode, SessionCache};

#[derive(Debug, Error)]
pub enum TenantError {
    #[error("Tenant not found: {0}")]
    NotFound(String),

    #[error("No tenant selected")]
    NoTenantSelected,

    #[error("Certificate error: {0}")]
    Certificate(String),

    #[error("Session cache error: {0}")]
    SessionCache(#[from] super::session_cache::SessionCacheError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Invalid credentials: {0}")]
    CredentialsInvalid(String),

    #[error("No active subscription: {0}")]
    NoSubscription(String),

    #[error("Auth server error: {0}")]
    AuthServerError(String),

    #[error("Activation failed: {0}")]
    AuthFailed(String),

    #[error("Device limit reached")]
    DeviceLimitReached(shared::activation::QuotaInfo),

    #[error("Client limit reached")]
    ClientLimitReached(shared::activation::QuotaInfo),

    #[error("Offline login not available for user: {0}")]
    OfflineNotAvailable(String),
}

/// Classify auth server error by ErrorCode
fn classify_auth_error_code(code: shared::error::ErrorCode) -> TenantError {
    use shared::error::ErrorCode;
    match code {
        ErrorCode::TenantCredentialsInvalid | ErrorCode::InvalidCredentials => {
            TenantError::CredentialsInvalid(code.message().to_string())
        }
        ErrorCode::TenantNoSubscription | ErrorCode::SubscriptionBlocked => {
            TenantError::NoSubscription(code.message().to_string())
        }
        ErrorCode::InternalError | ErrorCode::AuthServerError => {
            TenantError::AuthServerError(code.message().to_string())
        }
        _ => TenantError::AuthFailed(code.message().to_string()),
    }
}

/// 使用 TenantPaths 构建 mTLS HTTP 客户端
///
/// 用于 Client 模式连接远程 edge-server
fn build_mtls_http_client(paths: &TenantPaths) -> Result<reqwest::Client, TenantError> {
    // 加载证书
    let cert_pem = std::fs::read_to_string(paths.client_cert())
        .map_err(|e| TenantError::Certificate(format!("Failed to read client cert: {}", e)))?;
    let key_pem = std::fs::read_to_string(paths.client_key())
        .map_err(|e| TenantError::Certificate(format!("Failed to read client key: {}", e)))?;
    let ca_cert_pem = std::fs::read_to_string(paths.client_tenant_ca())
        .map_err(|e| TenantError::Certificate(format!("Failed to read CA cert: {}", e)))?;

    // 解析客户端证书
    let client_certs = crab_cert::to_rustls_certs(&cert_pem)
        .map_err(|e| TenantError::Certificate(format!("Failed to parse client cert: {}", e)))?;

    // 解析客户端私钥
    let client_key = crab_cert::to_rustls_key(&key_pem)
        .map_err(|e| TenantError::Certificate(format!("Failed to parse client key: {}", e)))?;

    // 解析 CA 证书
    let ca_certs = crab_cert::to_rustls_certs(&ca_cert_pem)
        .map_err(|e| TenantError::Certificate(format!("Failed to parse CA cert: {}", e)))?;

    let mut root_store = rustls::RootCertStore::empty();
    for cert in ca_certs {
        root_store
            .add(cert)
            .map_err(|e| TenantError::Certificate(format!("Failed to add CA cert: {}", e)))?;
    }

    // 创建 SkipHostnameVerifier
    let verifier = Arc::new(crab_cert::SkipHostnameVerifier::new(root_store));

    // 创建 rustls ClientConfig
    let config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_client_auth_cert(client_certs, client_key)
        .map_err(|e| TenantError::Certificate(format!("Failed to build TLS config: {}", e)))?;

    // 创建 reqwest 客户端
    reqwest::Client::builder()
        .use_preconfigured_tls(config)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| TenantError::Network(format!("Failed to build HTTP client: {}", e)))
}

/// 租户信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TenantInfo {
    pub tenant_id: String,
    pub tenant_name: Option<String>,
    pub has_certificates: bool,
    pub last_used: Option<u64>,
    /// 订阅状态 (从 credential.json 读取)
    pub subscription_status: Option<String>,
}

/// 多租户管理器
///
/// 使用 TenantPaths 管理路径，不依赖 CertManager。
pub struct TenantManager {
    /// 基础路径 (~/.red_coral/tenants)
    base_path: PathBuf,
    /// 当前活跃租户 ID
    current_tenant: Option<String>,
    /// 各租户的路径管理器
    tenant_paths: HashMap<String, TenantPaths>,
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
            tenant_paths: HashMap::new(),
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

        // 创建 TenantPaths
        let paths = TenantPaths::new(&tenant_path);
        self.tenant_paths.insert(tenant_id.to_string(), paths);

        // 加载 SessionCache
        let session_cache = SessionCache::load(&tenant_path)?;
        self.session_caches
            .insert(tenant_id.to_string(), session_cache);

        Ok(())
    }

    // ============ 租户管理 ============

    /// 列出所有已激活的租户
    pub fn list_tenants(&self) -> Vec<TenantInfo> {
        self.tenant_paths
            .iter()
            .map(|(tenant_id, paths)| {
                // 从 credential.json 读取订阅状态
                let subscription_status = Self::read_subscription_status(paths);
                TenantInfo {
                    tenant_id: tenant_id.clone(),
                    tenant_name: None,
                    has_certificates: paths.is_server_activated(),
                    last_used: None,
                    subscription_status,
                }
            })
            .collect()
    }

    /// 获取指定租户的订阅状态
    pub fn get_subscription_status(&self, tenant_id: &str) -> Option<String> {
        let paths = self.tenant_paths.get(tenant_id)?;
        Self::read_subscription_status(paths)
    }

    /// 从 credential.json 读取订阅状态
    fn read_subscription_status(paths: &TenantPaths) -> Option<String> {
        let cred_path = paths.credential_file();
        let content = std::fs::read_to_string(&cred_path).ok()?;
        let binding: edge_server::services::tenant_binding::TenantBinding =
            serde_json::from_str(&content).ok()?;
        let sub = binding.subscription?;
        Some(sub.status.as_str().to_string())
    }

    /// 激活设备 (获取 Edge Server 证书)
    ///
    /// 这是一个 "Server Activation" 流程，获取的证书可用于：
    /// 1. 运行 Edge Server (作为 Server Identity)
    /// 2. 连接其他节点 (作为 Client Identity)
    pub async fn activate_device(
        &mut self,
        auth_url: &str,
        token: &str,
        replace_entity_id: Option<&str>,
    ) -> Result<(String, String), TenantError> {
        // 1. 生成 Hardware ID
        let device_id = crab_cert::generate_hardware_id();
        tracing::info!("Activating device with ID: {}", device_id);

        // 2. 调用 Auth Server 激活接口
        let mut body = serde_json::json!({
            "token": token,
            "device_id": device_id,
        });
        if let Some(replace_id) = replace_entity_id {
            body["replace_entity_id"] = serde_json::json!(replace_id);
        }

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/api/server/activate", auth_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| TenantError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
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
            let code = resp_data
                .error_code
                .unwrap_or(shared::error::ErrorCode::InternalError);
            if code == shared::error::ErrorCode::DeviceLimitReached {
                if let Some(quota_info) = resp_data.quota_info {
                    return Err(TenantError::DeviceLimitReached(quota_info));
                }
            }
            return Err(classify_auth_error_code(code));
        }

        let data = resp_data
            .data
            .ok_or_else(|| TenantError::AuthFailed("Missing activation data".to_string()))?;

        let tenant_id = data.tenant_id.clone();
        let entity_id = data.entity_id.clone();

        // 4. 准备租户目录 (使用 TenantPaths)
        let tenant_path = self.base_path.join(&tenant_id);
        let paths = TenantPaths::new(&tenant_path);

        // 创建所有必要的目录
        paths.ensure_server_dirs()?;

        // 5. 验证证书链 (简化版，完整验证由 edge-server 启动时进行)
        // 这里主要确保我们拿到了看起来正确的东西

        // 6. 保存 Edge Server 证书到 {tenant}/server/certs/
        // edge-server 的 work_dir = {tenant}/server/，从 work_dir/certs/ 读取
        // 注意：不再写 client 路径，Client 模式需要调用 activate_client() 获取专用证书
        std::fs::write(paths.server_cert(), &data.entity_cert)?;
        crab_cert::write_secret_file(paths.server_key(), &data.entity_key)?;
        std::fs::write(paths.server_tenant_ca(), &data.tenant_ca_cert)?;
        std::fs::write(paths.server_root_ca(), &data.root_ca_cert)?;

        // 8. 保存 Credential (用于 activation check)
        let credential_path = paths.credential_file();
        tracing::info!("Saving credential to: {:?}", credential_path);

        // 包装在 TenantBinding 中，subscription 直接使用 shared::activation::SubscriptionInfo
        let mut tenant_binding = edge_server::services::tenant_binding::TenantBinding::from_signed(
            data.binding.clone(),
            data.store_number,
        );

        if let Some(mut sub_info) = data.subscription.clone() {
            sub_info.last_checked_at = shared::util::now_millis();
            tenant_binding.subscription = Some(sub_info);
        }
        let credential_json = serde_json::to_string_pretty(&tenant_binding).map_err(|e| {
            tracing::error!("Failed to serialize credential: {}", e);
            TenantError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?;

        crab_cert::write_secret_file(&credential_path, &credential_json).map_err(|e| {
            tracing::error!("Failed to write credential to {:?}: {}", credential_path, e);
            TenantError::Io(e)
        })?;

        tracing::info!(
            "Credential saved successfully ({} bytes)",
            credential_json.len()
        );

        // 8. 更新内存状态 - 使用 TenantPaths
        self.load_tenant(&tenant_id)?;

        // 9. 自动切换
        self.switch_tenant(&tenant_id)?;

        tracing::info!(tenant_id = %tenant_id, entity_id = %entity_id, "Device activated successfully");

        Ok((tenant_id, entity_id))
    }

    /// 激活客户端 (获取 Client 证书)
    ///
    /// 这是一个 "Client Activation" 流程，获取的证书用于：
    /// 1. 以 Client 身份连接远程 Edge Server (mTLS)
    ///
    /// 与 activate_device 的区别:
    /// - 调用 `/api/client/activate` 而非 `/api/server/activate`
    /// - 获取 EntityType::Client 证书
    /// - 只保存到 client cert 路径，不保存 server cert
    pub async fn activate_client(
        &mut self,
        auth_url: &str,
        token: &str,
        replace_entity_id: Option<&str>,
    ) -> Result<(String, String), TenantError> {
        // 1. 生成 Hardware ID
        let device_id = crab_cert::generate_hardware_id();
        tracing::info!("Activating client with device ID: {}", device_id);

        // 2. 调用 Auth Server 客户端激活接口
        let mut body = serde_json::json!({
            "token": token,
            "device_id": device_id,
        });
        if let Some(replace_id) = replace_entity_id {
            body["replace_entity_id"] = serde_json::json!(replace_id);
        }

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/api/client/activate", auth_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| TenantError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            return Err(TenantError::AuthFailed(format!(
                "Client activation failed: {}",
                text
            )));
        }

        // 3. 解析响应
        let resp_data: shared::activation::ActivationResponse = resp
            .json()
            .await
            .map_err(|e| TenantError::Network(format!("Invalid response: {}", e)))?;

        if !resp_data.success {
            let code = resp_data
                .error_code
                .unwrap_or(shared::error::ErrorCode::InternalError);
            if code == shared::error::ErrorCode::ClientLimitReached {
                if let Some(quota_info) = resp_data.quota_info {
                    return Err(TenantError::ClientLimitReached(quota_info));
                }
            }
            return Err(classify_auth_error_code(code));
        }

        let data = resp_data
            .data
            .ok_or_else(|| TenantError::AuthFailed("Missing activation data".to_string()))?;

        let tenant_id = data.tenant_id.clone();
        let entity_id = data.entity_id.clone();

        // 4. 准备租户目录
        let tenant_path = self.base_path.join(&tenant_id);
        let paths = TenantPaths::new(&tenant_path);

        // 5. 保存客户端证书到 {tenant}/certs/ (用于 mTLS Client Mode)
        paths.ensure_client_dirs()?;
        std::fs::write(paths.client_cert(), &data.entity_cert)?;
        crab_cert::write_secret_file(paths.client_key(), &data.entity_key)?;
        std::fs::write(paths.client_tenant_ca(), &data.tenant_ca_cert)?;

        // 6. 保存 Credential
        let credential_path = paths.credential_file();
        tracing::info!("Saving credential to: {:?}", credential_path);

        let mut tenant_binding = edge_server::services::tenant_binding::TenantBinding::from_signed(
            data.binding.clone(),
            data.store_number,
        );

        if let Some(mut sub_info) = data.subscription.clone() {
            sub_info.last_checked_at = shared::util::now_millis();
            tenant_binding.subscription = Some(sub_info);
        }
        let credential_json = serde_json::to_string_pretty(&tenant_binding).map_err(|e| {
            tracing::error!("Failed to serialize credential: {}", e);
            TenantError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?;

        crab_cert::write_secret_file(&credential_path, &credential_json).map_err(|e| {
            tracing::error!("Failed to write credential to {:?}: {}", credential_path, e);
            TenantError::Io(e)
        })?;

        tracing::info!(
            "Credential saved successfully ({} bytes)",
            credential_json.len()
        );

        // 7. 更新内存状态
        self.load_tenant(&tenant_id)?;

        // 8. 自动切换
        self.switch_tenant(&tenant_id)?;

        tracing::info!(tenant_id = %tenant_id, entity_id = %entity_id, "Client activated successfully");

        Ok((tenant_id, entity_id))
    }

    /// 验证租户凭据 (不签发证书)
    pub async fn verify_tenant(
        &self,
        auth_url: &str,
        username: &str,
        password: &str,
    ) -> Result<shared::activation::TenantVerifyData, TenantError> {
        let device_id = crab_cert::generate_hardware_id();

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/api/tenant/verify", auth_url))
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
            return Err(TenantError::AuthFailed(format!("Verify failed: {}", text)));
        }

        let resp_data: shared::activation::TenantVerifyResponse = resp
            .json()
            .await
            .map_err(|e| TenantError::Network(format!("Invalid response: {}", e)))?;

        if !resp_data.success {
            let code = resp_data
                .error_code
                .unwrap_or(shared::error::ErrorCode::InternalError);
            return Err(classify_auth_error_code(code));
        }

        resp_data
            .data
            .ok_or_else(|| TenantError::AuthFailed("Missing verify data".to_string()))
    }

    /// 注销 Server 激活记录 (释放配额)
    pub async fn deactivate_server(
        &self,
        auth_url: &str,
        token: &str,
        entity_id: &str,
    ) -> Result<(), TenantError> {
        let device_id = crab_cert::generate_hardware_id();

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/api/server/deactivate", auth_url))
            .json(&serde_json::json!({
                "token": token,
                "device_id": device_id,
                "entity_id": entity_id,
            }))
            .send()
            .await
            .map_err(|e| TenantError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(TenantError::AuthFailed(format!(
                "Deactivate server failed: {}",
                text
            )));
        }

        let resp_data: shared::activation::DeactivateResponse = resp
            .json()
            .await
            .map_err(|e| TenantError::Network(format!("Invalid response: {}", e)))?;

        if !resp_data.success {
            let code = resp_data
                .error_code
                .unwrap_or(shared::error::ErrorCode::InternalError);
            return Err(classify_auth_error_code(code));
        }

        Ok(())
    }

    /// 注销 Client 激活记录 (释放配额)
    pub async fn deactivate_client(
        &self,
        auth_url: &str,
        token: &str,
        entity_id: &str,
    ) -> Result<(), TenantError> {
        let device_id = crab_cert::generate_hardware_id();

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/api/client/deactivate", auth_url))
            .json(&serde_json::json!({
                "token": token,
                "device_id": device_id,
                "entity_id": entity_id,
            }))
            .send()
            .await
            .map_err(|e| TenantError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(TenantError::AuthFailed(format!(
                "Deactivate client failed: {}",
                text
            )));
        }

        let resp_data: shared::activation::DeactivateResponse = resp
            .json()
            .await
            .map_err(|e| TenantError::Network(format!("Invalid response: {}", e)))?;

        if !resp_data.success {
            let code = resp_data
                .error_code
                .unwrap_or(shared::error::ErrorCode::InternalError);
            return Err(classify_auth_error_code(code));
        }

        Ok(())
    }

    /// 删除 Server 模式证书
    pub fn delete_server_certs(&self) -> Result<(), TenantError> {
        let paths = self.current_paths().ok_or(TenantError::NoTenantSelected)?;
        paths.delete_server_certs()?;
        Ok(())
    }

    /// 删除 Client 模式证书
    pub fn delete_client_certs(&self) -> Result<(), TenantError> {
        let paths = self.current_paths().ok_or(TenantError::NoTenantSelected)?;
        paths.delete_client_certs()?;
        Ok(())
    }

    /// 切换当前租户
    pub fn switch_tenant(&mut self, tenant_id: &str) -> Result<(), TenantError> {
        if !self.tenant_paths.contains_key(tenant_id) {
            return Err(TenantError::NotFound(tenant_id.to_string()));
        }

        // 清除当前会话
        self.current_session = None;
        self.current_tenant = Some(tenant_id.to_string());

        tracing::info!(tenant_id = %tenant_id, "Switched to tenant");

        Ok(())
    }

    /// 清除当前租户选择（不删除文件）
    pub fn clear_current_tenant(&mut self) {
        self.current_tenant = None;
        self.current_session = None;
    }

    /// 移除租户 (删除本地证书和缓存)
    pub fn remove_tenant(&mut self, tenant_id: &str) -> Result<(), TenantError> {
        // 如果是当前租户，先清除
        if self.current_tenant.as_deref() == Some(tenant_id) {
            self.current_tenant = None;
            self.current_session = None;
        }

        // 移除管理器
        self.tenant_paths.remove(tenant_id);
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

        let paths = self
            .tenant_paths
            .get(&tenant_id)
            .ok_or_else(|| TenantError::NotFound(tenant_id.clone()))?;

        // 构建 mTLS HTTP 客户端
        let http_client = build_mtls_http_client(paths)?;

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
        let data: shared::client::LoginResponse = response
            .json()
            .await
            .map_err(|e: reqwest::Error| TenantError::Network(e.to_string()))?;

        // 创建会话
        let session = EmployeeSession {
            username: username.to_string(),
            token: data.token.clone(),
            user_info: data.user,
            login_mode: LoginMode::Online,
            expires_at: EmployeeSession::parse_jwt_exp(&data.token),
            logged_in_at: shared::util::now_millis(),
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

    /// 获取基础路径
    pub fn base_path(&self) -> &std::path::Path {
        &self.base_path
    }

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

    /// 获取当前租户的路径管理器
    pub fn current_paths(&self) -> Option<&TenantPaths> {
        self.current_tenant
            .as_ref()
            .and_then(|id| self.tenant_paths.get(id))
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
        let tenant_id = self
            .current_tenant
            .as_ref()
            .ok_or(TenantError::NoTenantSelected)?;

        let cache = self
            .session_caches
            .get(tenant_id)
            .ok_or_else(|| TenantError::NotFound(tenant_id.clone()))?;

        cache
            .save_current_session(session)
            .map_err(TenantError::SessionCache)
    }

    /// 加载缓存的当前活动会话
    pub fn load_current_session(&self) -> Result<Option<EmployeeSession>, TenantError> {
        let tenant_id = self
            .current_tenant
            .as_ref()
            .ok_or(TenantError::NoTenantSelected)?;

        let cache = self
            .session_caches
            .get(tenant_id)
            .ok_or_else(|| TenantError::NotFound(tenant_id.clone()))?;

        cache
            .load_current_session()
            .map_err(TenantError::SessionCache)
    }

    /// 清除缓存的当前活动会话（同时清内存 + 磁盘）
    pub fn clear_current_session(&mut self) -> Result<(), TenantError> {
        self.current_session = None;
        if let Some(tenant_id) = &self.current_tenant {
            if let Some(cache) = self.session_caches.get(tenant_id) {
                cache
                    .clear_current_session()
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
