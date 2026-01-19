//! ClientBridge - 统一的客户端桥接层
//!
//! 提供 Server/Client 模式的统一接口，屏蔽底层差异。
//! - Server 模式: 本地运行 edge-server，使用 In-Process 通信
//! - Client 模式: 连接远程 edge-server，使用 mTLS 通信

use std::path::PathBuf;
use std::sync::Arc;
use tauri::Emitter;
use thiserror::Error;
use tokio::sync::RwLock;

use crab_client::{Authenticated, Connected, CrabClient, Local, Remote};
use edge_server::services::tenant_binding::SubscriptionStatus;
use edge_server::ServerState;

use super::tenant_manager::{TenantError, TenantManager};

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("Not initialized")]
    NotInitialized,

    #[error("Not authenticated")]
    NotAuthenticated,

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Already running in {0} mode")]
    AlreadyRunning(String),

    #[error("Tenant error: {0}")]
    Tenant(#[from] TenantError),

    #[error("Client error: {0}")]
    Client(#[from] crab_client::ClientError),

    #[error("Server error: {0}")]
    Server(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// 运行模式类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ModeType {
    Server,
    Client,
    Disconnected,
}

/// 应用状态 (统一 Server/Client 模式)
///
/// 用于前端路由守卫和状态展示。
/// 参考设计文档: `docs/plans/2026-01-18-application-state-machine.md`
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum AppState {
    // === 通用状态 ===
    /// 未初始化
    Uninitialized,

    // === Server 模式专属 ===
    /// Server: 无租户
    ServerNoTenant,
    /// Server: 需要激活 (有租户目录但证书不完整或自检失败)
    ServerNeedActivation,
    /// Server: 正在激活
    ServerActivating,
    /// Server: 已激活，验证订阅中
    ServerCheckingSubscription,
    /// Server: 订阅无效，阻止使用
    ServerSubscriptionBlocked { reason: String },
    /// Server: 服务器就绪，等待员工登录
    ServerReady,
    /// Server: 员工已登录
    ServerAuthenticated,

    // === Client 模式专属 ===
    /// Client: 未连接
    ClientDisconnected,
    /// Client: 需要设置 (无缓存证书)
    ClientNeedSetup,
    /// Client: 正在连接
    ClientConnecting,
    /// Client: 已连接，等待员工登录
    ClientConnected,
    /// Client: 员工已登录
    ClientAuthenticated,
}

impl AppState {
    /// 是否可以访问 POS 主界面
    pub fn can_access_pos(&self) -> bool {
        matches!(self, AppState::ServerAuthenticated | AppState::ClientAuthenticated)
    }

    /// 是否需要员工登录
    pub fn needs_employee_login(&self) -> bool {
        matches!(self, AppState::ServerReady | AppState::ClientConnected)
    }

    /// 是否需要设置/激活
    pub fn needs_setup(&self) -> bool {
        matches!(
            self,
            AppState::Uninitialized
                | AppState::ServerNoTenant
                | AppState::ServerNeedActivation
                | AppState::ClientDisconnected
                | AppState::ClientNeedSetup
        )
    }

    /// 是否被订阅阻止
    pub fn is_subscription_blocked(&self) -> bool {
        matches!(self, AppState::ServerSubscriptionBlocked { .. })
    }
}

impl std::fmt::Display for ModeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModeType::Server => write!(f, "server"),
            ModeType::Client => write!(f, "client"),
            ModeType::Disconnected => write!(f, "disconnected"),
        }
    }
}

/// Server 模式配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerModeConfig {
    /// HTTP 端口
    pub http_port: u16,
    /// 数据目录
    pub data_dir: PathBuf,
    /// 消息总线端口
    pub message_port: u16,
}

impl Default for ServerModeConfig {
    fn default() -> Self {
        Self {
            http_port: 9625,
            data_dir: PathBuf::from("./data"),
            message_port: 9626,
        }
    }
}

/// Client 模式配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClientModeConfig {
    /// Edge Server URL (HTTPS)
    pub edge_url: String,
    /// 消息总线地址
    pub message_addr: String,
    /// Auth Server URL
    pub auth_url: String,
}

/// 应用配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    /// 当前模式
    pub current_mode: ModeType,
    /// 当前租户
    pub current_tenant: Option<String>,
    /// Server 模式配置
    pub server_config: ServerModeConfig,
    /// Client 模式配置
    pub client_config: Option<ClientModeConfig>,
    /// 已知租户列表
    pub known_tenants: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            current_mode: ModeType::Disconnected,
            current_tenant: None,
            server_config: ServerModeConfig::default(),
            client_config: None,
            known_tenants: Vec::new(),
        }
    }
}

impl AppConfig {
    /// 从文件加载配置
    pub fn load(path: &std::path::Path) -> Result<Self, BridgeError> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            serde_json::from_str(&content).map_err(|e| BridgeError::Config(e.to_string()))
        } else {
            Ok(Self::default())
        }
    }

    /// 保存配置到文件
    pub fn save(&self, path: &std::path::Path) -> Result<(), BridgeError> {
        let content =
            serde_json::to_string_pretty(self).map_err(|e| BridgeError::Config(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// 模式信息 (用于前端显示)
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModeInfo {
    pub mode: ModeType,
    pub is_connected: bool,
    pub is_authenticated: bool,
    pub tenant_id: Option<String>,
    pub username: Option<String>,
}

/// Server 模式的客户端状态
#[allow(dead_code)]
enum LocalClientState {
    Connected(CrabClient<Local, Connected>),
    Authenticated(CrabClient<Local, Authenticated>),
}

/// Client 模式的客户端状态 (参考 message_client 示例)
#[allow(dead_code)]
enum RemoteClientState {
    Connected(CrabClient<Remote, Connected>),
    Authenticated(CrabClient<Remote, Authenticated>),
}

/// 客户端模式枚举
#[allow(dead_code)]
enum ClientMode {
    /// Server 模式: 本地运行 edge-server
    Server {
        server_state: Arc<ServerState>,
        client: Option<LocalClientState>,
        server_task: tokio::task::JoinHandle<()>,
    },
    /// Client 模式: 连接远程 edge-server
    Client {
        client: Option<RemoteClientState>,
        edge_url: String,
        message_addr: String,
    },
    /// 未连接状态
    Disconnected,
}

/// 客户端桥接层
pub struct ClientBridge {
    /// 多租户管理器
    tenant_manager: Arc<RwLock<TenantManager>>,
    /// 当前模式
    mode: RwLock<ClientMode>,
    /// 应用配置
    config: RwLock<AppConfig>,
    /// 配置文件路径
    config_path: PathBuf,
    /// 基础数据目录
    #[allow(dead_code)]
    base_path: PathBuf,
    /// Tauri AppHandle for emitting events (optional for testing)
    app_handle: Option<tauri::AppHandle>,
}

impl ClientBridge {
    /// 创建新的 ClientBridge (convenience wrapper without AppHandle)
    pub fn new(base_path: impl Into<PathBuf>, client_name: &str) -> Result<Self, BridgeError> {
        Self::with_app_handle(base_path, client_name, None)
    }

    /// 创建新的 ClientBridge with optional AppHandle for emitting Tauri events
    pub fn with_app_handle(
        base_path: impl Into<PathBuf>,
        client_name: &str,
        app_handle: Option<tauri::AppHandle>,
    ) -> Result<Self, BridgeError> {
        let base_path = base_path.into();
        std::fs::create_dir_all(&base_path)?;

        let config_path = base_path.join("config.json");
        let config = AppConfig::load(&config_path)?;

        let tenants_path = base_path.join("tenants");
        let mut tenant_manager = TenantManager::new(&tenants_path, client_name);
        tenant_manager.load_existing_tenants()?;

        Ok(Self {
            tenant_manager: Arc::new(RwLock::new(tenant_manager)),
            mode: RwLock::new(ClientMode::Disconnected),
            config: RwLock::new(config),
            config_path,
            base_path,
            app_handle,
        })
    }

    /// Set the app handle after initialization
    pub fn set_app_handle(&mut self, handle: tauri::AppHandle) {
        self.app_handle = Some(handle);
    }

    /// 保存当前配置
    async fn save_config(&self) -> Result<(), BridgeError> {
        let config = self.config.read().await;
        config.save(&self.config_path)
    }

    // ============ 租户管理辅助 ============

    /// 切换当前租户并保存配置
    pub async fn switch_tenant(&self, tenant_id: &str) -> Result<(), BridgeError> {
        // Check if we need to restart server
        let is_server_mode = {
            let mode = self.mode.read().await;
            matches!(*mode, ClientMode::Server { .. })
        };

        // If server mode, stop it first to release resources/locks
        if is_server_mode {
            tracing::info!("Stopping server to switch tenant...");
            self.stop().await?;
        }

        // 1. 切换内存状态
        {
            let mut tm = self.tenant_manager.write().await;
            tm.switch_tenant(tenant_id)?;
        }

        // 2. 更新并保存配置
        {
            let mut config = self.config.write().await;
            config.current_tenant = Some(tenant_id.to_string());
            config.save(&self.config_path)?;
        }

        // If it was server mode, restart it with new tenant data
        if is_server_mode {
            tracing::info!("Restarting server with new tenant...");
            self.start_server_mode().await?;
        }

        tracing::info!(tenant_id = %tenant_id, "Switched tenant and saved config");
        Ok(())
    }

    /// 激活设备并自动切换租户，保存配置
    pub async fn handle_activation(
        &self,
        auth_url: &str,
        username: &str,
        password: &str,
    ) -> Result<String, BridgeError> {
        // 1. 调用 TenantManager 激活
        let tenant_id = {
            let mut tm = self.tenant_manager.write().await;
            tm.activate_device(auth_url, username, password).await?
        };

        // 2. 更新已知租户列表和当前租户
        {
            let mut config = self.config.write().await;
            if !config.known_tenants.contains(&tenant_id) {
                config.known_tenants.push(tenant_id.clone());
            }
            config.current_tenant = Some(tenant_id.clone());
            config.save(&self.config_path)?;
        }

        tracing::info!(tenant_id = %tenant_id, "Device activated and config saved");
        Ok(tenant_id)
    }

    // ============ 模式管理 ============

    /// 恢复上次的会话状态 (启动时调用)
    pub async fn restore_last_session(&self) -> Result<(), BridgeError> {
        let config = self.config.read().await;
        let mode = config.current_mode;
        let client_config = config.client_config.clone();
        let current_tenant = config.current_tenant.clone();
        drop(config);

        // 恢复租户选择
        if let Some(tenant_id) = current_tenant {
            tracing::info!("Restoring tenant selection: {}", tenant_id);
            let mut tm = self.tenant_manager.write().await;
            if let Err(e) = tm.switch_tenant(&tenant_id) {
                tracing::warn!("Failed to restore tenant {}: {}", tenant_id, e);
            }
        }

        match mode {
            ModeType::Server => {
                tracing::info!("Restoring Server mode...");
                if let Err(e) = self.start_server_mode().await {
                    tracing::error!("Failed to restore Server mode: {}", e);
                    // Fallback to disconnected?
                    // We shouldn't change config here to avoid overwriting user preference on transient errors
                    return Err(e);
                }
            }
            ModeType::Client => {
                if let Some(cfg) = client_config {
                    tracing::info!("Restoring Client mode...");
                    if let Err(e) = self
                        .start_client_mode(&cfg.edge_url, &cfg.message_addr)
                        .await
                    {
                        tracing::error!("Failed to restore Client mode: {}", e);
                        return Err(e);
                    }
                } else {
                    tracing::warn!("Client mode configured but missing client_config");
                }
            }
            ModeType::Disconnected => {
                tracing::info!("Starting in Disconnected mode");
            }
        }
        Ok(())
    }

    /// 获取当前模式信息
    pub async fn get_mode_info(&self) -> ModeInfo {
        let mode_guard = self.mode.read().await;
        let tenant_manager = self.tenant_manager.read().await;

        let (mode, is_connected, is_authenticated, client_check_info) = match &*mode_guard {
            ClientMode::Disconnected => (ModeType::Disconnected, false, false, None),
            ClientMode::Server { client, .. } => {
                let is_auth = matches!(client, Some(LocalClientState::Authenticated(_)));
                // Server mode is always considered connected to itself
                (ModeType::Server, true, is_auth, None)
            }
            ClientMode::Client {
                client, edge_url, ..
            } => {
                let is_auth = matches!(client, Some(RemoteClientState::Authenticated(_)));
                // Extract info needed for health check
                let check_info = if let Some(state) = client {
                    let http = match state {
                        RemoteClientState::Connected(c) => c.edge_http_client().cloned(),
                        RemoteClientState::Authenticated(c) => c.edge_http_client().cloned(),
                    };
                    Some((edge_url.clone(), http))
                } else {
                    None
                };
                (ModeType::Client, false, is_auth, check_info)
            }
        };

        // Release lock before async network call
        drop(mode_guard);

        // Perform real network health check for Client mode
        let final_is_connected = if mode == ModeType::Client {
            if let Some((url, Some(http))) = client_check_info {
                match http
                    .get(format!("{}/health", url))
                    .timeout(std::time::Duration::from_secs(2))
                    .send()
                    .await
                {
                    Ok(resp) => resp.status().is_success(),
                    Err(e) => {
                        tracing::warn!("Health check failed: {}", e);
                        false
                    }
                }
            } else {
                false
            }
        } else {
            is_connected
        };

        ModeInfo {
            mode,
            is_connected: final_is_connected,
            is_authenticated,
            tenant_id: tenant_manager.current_tenant_id().map(|s| s.to_string()),
            username: tenant_manager.current_session().map(|s| s.username.clone()),
        }
    }

    /// 获取应用状态 (用于前端路由守卫)
    ///
    /// 根据当前模式和内部状态计算出应用所处的状态。
    /// 参考设计文档: `docs/plans/2026-01-18-application-state-machine.md`
    pub async fn get_app_state(&self) -> AppState {
        let mode_guard = self.mode.read().await;
        let tenant_manager = self.tenant_manager.read().await;

        match &*mode_guard {
            ClientMode::Disconnected => {
                // 检查是否有租户
                if tenant_manager.current_tenant_id().is_none() {
                    // 无租户选择
                    AppState::ServerNoTenant
                } else {
                    // 有租户但未启动任何模式
                    let has_certs = tenant_manager
                        .current_cert_manager()
                        .map(|cm| cm.has_local_certificates())
                        .unwrap_or(false);

                    if has_certs {
                        // 有证书，可能需要选择模式 (Server/Client)
                        AppState::Uninitialized
                    } else {
                        // 无证书，需要激活
                        AppState::ServerNeedActivation
                    }
                }
            }

            ClientMode::Server {
                server_state,
                client,
                ..
            } => {
                // Server 模式: 检查激活状态和订阅
                let is_activated = server_state.is_activated().await;

                if !is_activated {
                    return AppState::ServerNeedActivation;
                }

                // 检查订阅状态
                let credential = server_state
                    .activation_service()
                    .get_credential()
                    .await
                    .ok()
                    .flatten();

                if let Some(cred) = credential {
                    if let Some(sub) = &cred.subscription {
                        // 检查订阅状态
                        match sub.status {
                            SubscriptionStatus::Active | SubscriptionStatus::Trial => {
                                // 订阅有效，检查员工登录状态
                                match client {
                                    Some(LocalClientState::Authenticated(_)) => {
                                        AppState::ServerAuthenticated
                                    }
                                    _ => AppState::ServerReady,
                                }
                            }
                            SubscriptionStatus::PastDue => {
                                // 宽限期，允许使用但显示警告
                                match client {
                                    Some(LocalClientState::Authenticated(_)) => {
                                        AppState::ServerAuthenticated
                                    }
                                    _ => AppState::ServerReady,
                                }
                            }
                            SubscriptionStatus::Canceled | SubscriptionStatus::Unpaid => {
                                // 订阅无效，阻止使用
                                AppState::ServerSubscriptionBlocked {
                                    reason: format!("订阅状态: {:?}", sub.status),
                                }
                            }
                        }
                    } else {
                        // 无订阅信息，检查是否正在同步
                        AppState::ServerCheckingSubscription
                    }
                } else {
                    // 无凭证，需要激活
                    AppState::ServerNeedActivation
                }
            }

            ClientMode::Client { client, .. } => {
                // Client 模式: 检查连接状态和员工登录
                match client {
                    Some(RemoteClientState::Authenticated(_)) => AppState::ClientAuthenticated,
                    Some(RemoteClientState::Connected(_)) => AppState::ClientConnected,
                    None => {
                        // 检查是否有缓存证书
                        let has_certs = tenant_manager
                            .current_cert_manager()
                            .map(|cm| cm.has_local_certificates())
                            .unwrap_or(false);

                        if has_certs {
                            AppState::ClientDisconnected
                        } else {
                            AppState::ClientNeedSetup
                        }
                    }
                }
            }
        }
    }

    /// 以 Server 模式启动
    pub async fn start_server_mode(&self) -> Result<(), BridgeError> {
        let mut mode_guard = self.mode.write().await;

        // 检查当前模式
        if !matches!(&*mode_guard, ClientMode::Disconnected) {
            let current = match &*mode_guard {
                ClientMode::Server { .. } => "server",
                ClientMode::Client { .. } => "client",
                ClientMode::Disconnected => "disconnected",
            };
            return Err(BridgeError::AlreadyRunning(current.to_string()));
        }

        let config = self.config.read().await;
        let server_config = &config.server_config;

        // 获取当前租户目录作为工作目录
        let tenant_manager = self.tenant_manager.read().await;
        let work_dir = if let Some(path) = tenant_manager.current_tenant_path() {
            tracing::info!("Using tenant directory for server: {:?}", path);
            path.to_string_lossy().to_string()
        } else {
            tracing::warn!(
                "No active tenant, falling back to default data dir: {:?}",
                server_config.data_dir
            );
            server_config.data_dir.to_string_lossy().to_string()
        };
        drop(tenant_manager);

        // 创建 EdgeServer 配置
        let edge_config = edge_server::Config::builder()
            .work_dir(work_dir)
            .http_port(server_config.http_port)
            .message_tcp_port(server_config.message_port)
            .build();

        // 初始化 ServerState
        let server_state = ServerState::initialize(&edge_config).await;
        // 注意: initialize 会调用 start_background_tasks (在 Server::run 中也会调用，但多调用一次无害，或者我们可以依靠 Server::run)
        // 但我们需要 state_arc 来初始化本地客户端

        // 关键修改: 使用 edge_server::Server::run 来启动完整的服务器 (HTTP + TCP + Background Tasks)
        // 这样可以支持外部客户端连接 (如收银机)
        let server_instance =
            edge_server::Server::with_state(edge_config.clone(), server_state.clone());

        let server_task = tokio::spawn(async move {
            tracing::info!("🚀 Starting Edge Server background task...");
            if let Err(e) = server_instance.run().await {
                tracing::error!("❌ Server run error: {}", e);
            }
        });

        let state_arc = Arc::new(server_state);

        // 获取 Router 和消息通道
        // 注意: Server::run 会启动 HTTPS 服务，但我们本地 UI 仍然可以使用 router 直接通信 (In-Process)
        let router = state_arc
            .https_service()
            .router()
            .ok_or_else(|| BridgeError::Server("Router not initialized".to_string()))?;

        let message_bus = state_arc.message_bus();
        let client_tx = message_bus.sender_to_server().clone();
        let server_tx = message_bus.sender().clone();

        // 启动消息广播订阅 (转发给前端)
        if let Some(handle) = &self.app_handle {
            let mut server_rx = message_bus.subscribe();
            let handle_clone = handle.clone();

            tokio::spawn(async move {
                loop {
                    match server_rx.recv().await {
                        Ok(msg) => {
                            let event = crate::events::ServerMessageEvent::from(msg);
                            if let Err(e) = handle_clone.emit("server-message", &event) {
                                tracing::warn!("Failed to emit server message: {}", e);
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("Server message listener lagged {} messages", n);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            tracing::debug!("Server message channel closed");
                            break;
                        }
                    }
                }
            });

            tracing::info!("Server message listener started");
        }

        // 创建 CrabClient<Local>
        let client = CrabClient::local()
            .with_router(router)
            .with_message_channels(client_tx, server_tx)
            .build()?;

        // 连接客户端
        let connected_client = client.connect().await?;

        tracing::info!(
            port = server_config.http_port,
            "Server mode initialized with In-Process client and Background Server"
        );

        *mode_guard = ClientMode::Server {
            server_state: state_arc,
            client: Some(LocalClientState::Connected(connected_client)),
            server_task,
        };

        // 更新配置
        drop(config);
        {
            let mut config = self.config.write().await;
            config.current_mode = ModeType::Server;
        }
        self.save_config().await?;

        Ok(())
    }

    /// 以 Client 模式连接
    ///
    /// 参考 crab-client/examples/message_client.rs 的 /reconnect 命令
    pub async fn start_client_mode(
        &self,
        edge_url: &str,
        message_addr: &str,
    ) -> Result<(), BridgeError> {
        let mut mode_guard = self.mode.write().await;

        // 检查当前模式
        if !matches!(&*mode_guard, ClientMode::Disconnected) {
            let current = match &*mode_guard {
                ClientMode::Server { .. } => "server",
                ClientMode::Client { .. } => "client",
                ClientMode::Disconnected => "disconnected",
            };
            return Err(BridgeError::AlreadyRunning(current.to_string()));
        }

        let tenant_manager = self.tenant_manager.read().await;
        let cert_manager = tenant_manager
            .current_cert_manager()
            .ok_or(TenantError::NoTenantSelected)?;

        let config = self.config.read().await;
        let auth_url = config
            .client_config
            .as_ref()
            .map(|c| c.auth_url.clone())
            .unwrap_or_else(|| "https://auth.example.com".to_string());
        drop(config);

        // 检查是否有缓存的证书
        if !cert_manager.has_local_certificates() {
            return Err(BridgeError::Config(
                "No cached certificates. Please activate tenant first.".into(),
            ));
        }

        // 创建 CrabClient<Remote> - 参考 message_client 示例
        let client = CrabClient::remote()
            .auth_server(&auth_url)
            .edge_server(edge_url) // 需要设置 edge_server 用于 HTTP API
            .cert_path(cert_manager.cert_path())
            .client_name(tenant_manager.client_name())
            .build()?;

        // 使用缓存的证书重连 (包含 self-check 和 timestamp refresh)
        let connected_client = client.reconnect(message_addr).await?;

        tracing::info!(edge_url = %edge_url, message_addr = %message_addr, "Client mode connected");

        // 启动消息广播订阅 (转发给前端)
        if let Some(handle) = &self.app_handle {
            if let Some(mc) = connected_client.message_client() {
                let mut rx = mc.subscribe();
                let handle_clone = handle.clone();

                tokio::spawn(async move {
                    loop {
                        match rx.recv().await {
                            Ok(msg) => {
                                // NetworkMessageClient returns BusMessage
                                // Convert to ServerMessageEvent using From impl
                                let event = crate::events::ServerMessageEvent::from(msg);
                                if let Err(e) = handle_clone.emit("server-message", &event) {
                                    tracing::warn!("Failed to emit server message: {}", e);
                                }
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                tracing::warn!("Client message listener lagged {} messages", n);
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                tracing::debug!("Client message channel closed");
                                break;
                            }
                        }
                    }
                });

                tracing::info!("Client message listener started");
            }
        }

        // 保存 Remote client 到 ClientMode::Client
        *mode_guard = ClientMode::Client {
            client: Some(RemoteClientState::Connected(connected_client)),
            edge_url: edge_url.to_string(),
            message_addr: message_addr.to_string(),
        };

        // 更新配置
        {
            let mut config = self.config.write().await;
            config.current_mode = ModeType::Client;
            config.client_config = Some(ClientModeConfig {
                edge_url: edge_url.to_string(),
                message_addr: message_addr.to_string(),
                auth_url,
            });
        }
        self.save_config().await?;

        Ok(())
    }

    /// 停止当前模式
    pub async fn stop(&self) -> Result<(), BridgeError> {
        let mut mode_guard = self.mode.write().await;

        // 如果是 Server 模式，中止后台任务
        if let ClientMode::Server { server_task, .. } = &*mode_guard {
            server_task.abort();
            tracing::info!("Server background task aborted");
        }

        *mode_guard = ClientMode::Disconnected;

        {
            let mut config = self.config.write().await;
            config.current_mode = ModeType::Disconnected;
        }
        self.save_config().await?;

        tracing::info!("Mode stopped, now disconnected");

        Ok(())
    }

    // ============ 租户管理代理 ============

    /// 获取租户管理器的只读引用
    pub fn tenant_manager(&self) -> &Arc<RwLock<TenantManager>> {
        &self.tenant_manager
    }

    /// 获取服务器模式配置
    pub async fn get_server_config(&self) -> ServerModeConfig {
        self.config.read().await.server_config.clone()
    }

    /// 获取客户端模式配置
    pub async fn get_client_config(&self) -> Option<ClientModeConfig> {
        self.config.read().await.client_config.clone()
    }

    // ============ 员工认证 ============

    /// 员工登录 (使用 CrabClient)
    ///
    /// Server 模式下使用 In-Process 登录，Client 模式下使用 mTLS HTTP
    pub async fn login_employee(
        &self,
        username: &str,
        password: &str,
    ) -> Result<super::session_cache::EmployeeSession, BridgeError> {
        let mut mode_guard = self.mode.write().await;

        match &mut *mode_guard {
            ClientMode::Server {
                server_state: _,
                client,
                ..
            } => {
                // 取出当前 client
                let current_client = client.take().ok_or(BridgeError::NotInitialized)?;

                match current_client {
                    LocalClientState::Connected(connected) => {
                        // 执行登录
                        match connected.login(username, password).await {
                            Ok(authenticated) => {
                                // 获取用户信息 - 使用 me() 和 token() 方法
                                let user_info = authenticated.me().cloned().ok_or_else(|| {
                                    BridgeError::Client(crab_client::ClientError::Auth(
                                        "No user info after login".into(),
                                    ))
                                })?;
                                let token = authenticated.token().unwrap_or_default().to_string();

                                // 创建会话
                                let session = super::session_cache::EmployeeSession {
                                    username: username.to_string(),
                                    token,
                                    user_info,
                                    login_mode: super::session_cache::LoginMode::Online,
                                    expires_at: None,
                                    logged_in_at: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs(),
                                };

                                // 保存 authenticated client
                                *client = Some(LocalClientState::Authenticated(authenticated));

                                tracing::info!(username = %username, "Employee logged in via CrabClient (local)");
                                Ok(session)
                            }
                            Err(e) => {
                                // 登录失败，client 已被消费，设置为 None
                                *client = None;
                                Err(BridgeError::Client(e))
                            }
                        }
                    }
                    LocalClientState::Authenticated(auth) => {
                        // 已经登录，先登出再重新登录
                        let connected = auth.logout().await;
                        match connected.login(username, password).await {
                            Ok(authenticated) => {
                                // 使用 me() 和 token() 方法获取会话数据
                                let user_info = authenticated.me().cloned().ok_or_else(|| {
                                    BridgeError::Client(crab_client::ClientError::Auth(
                                        "No user info after login".into(),
                                    ))
                                })?;
                                let token = authenticated.token().unwrap_or_default().to_string();

                                let session = super::session_cache::EmployeeSession {
                                    username: username.to_string(),
                                    token,
                                    user_info,
                                    login_mode: super::session_cache::LoginMode::Online,
                                    expires_at: None,
                                    logged_in_at: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs(),
                                };

                                *client = Some(LocalClientState::Authenticated(authenticated));
                                tracing::info!(username = %username, "Employee re-logged in via CrabClient (local)");
                                Ok(session)
                            }
                            Err(e) => {
                                *client = None;
                                Err(BridgeError::Client(e))
                            }
                        }
                    }
                }
            }
            ClientMode::Client { client, .. } => {
                // Client 模式使用 CrabClient 登录 (参考 message_client 示例)
                let current_client = client.take().ok_or(BridgeError::NotInitialized)?;

                match current_client {
                    RemoteClientState::Connected(connected) => {
                        match connected.login(username, password).await {
                            Ok(authenticated) => {
                                let user_info = authenticated.me().cloned().ok_or_else(|| {
                                    BridgeError::Client(crab_client::ClientError::Auth(
                                        "No user info after login".into(),
                                    ))
                                })?;
                                let token = authenticated.token().unwrap_or_default().to_string();

                                let session = super::session_cache::EmployeeSession {
                                    username: username.to_string(),
                                    token,
                                    user_info,
                                    login_mode: super::session_cache::LoginMode::Online,
                                    expires_at: None,
                                    logged_in_at: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs(),
                                };

                                *client = Some(RemoteClientState::Authenticated(authenticated));
                                tracing::info!(username = %username, "Employee logged in via CrabClient (remote)");
                                Ok(session)
                            }
                            Err(e) => {
                                *client = None;
                                Err(BridgeError::Client(e))
                            }
                        }
                    }
                    RemoteClientState::Authenticated(auth) => {
                        // 已登录，先登出再重新登录
                        let connected = auth.logout().await;
                        match connected.login(username, password).await {
                            Ok(authenticated) => {
                                let user_info = authenticated.me().cloned().ok_or_else(|| {
                                    BridgeError::Client(crab_client::ClientError::Auth(
                                        "No user info after login".into(),
                                    ))
                                })?;
                                let token = authenticated.token().unwrap_or_default().to_string();

                                let session = super::session_cache::EmployeeSession {
                                    username: username.to_string(),
                                    token,
                                    user_info,
                                    login_mode: super::session_cache::LoginMode::Online,
                                    expires_at: None,
                                    logged_in_at: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs(),
                                };

                                *client = Some(RemoteClientState::Authenticated(authenticated));
                                tracing::info!(username = %username, "Employee re-logged in via CrabClient (remote)");
                                Ok(session)
                            }
                            Err(e) => {
                                *client = None;
                                Err(BridgeError::Client(e))
                            }
                        }
                    }
                }
            }
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// 员工登出
    pub async fn logout_employee(&self) -> Result<(), BridgeError> {
        let mut mode_guard = self.mode.write().await;

        match &mut *mode_guard {
            ClientMode::Server {
                server_state: _,
                client,
                ..
            } => {
                if let Some(current_client) = client.take() {
                    match current_client {
                        LocalClientState::Authenticated(auth) => {
                            let connected = auth.logout().await;
                            *client = Some(LocalClientState::Connected(connected));
                            tracing::info!("Employee logged out (local)");
                        }
                        LocalClientState::Connected(c) => {
                            *client = Some(LocalClientState::Connected(c));
                        }
                    }
                }
            }
            ClientMode::Client { client, .. } => {
                if let Some(current_client) = client.take() {
                    match current_client {
                        RemoteClientState::Authenticated(auth) => {
                            let connected = auth.logout().await;
                            *client = Some(RemoteClientState::Connected(connected));
                            tracing::info!("Employee logged out (remote)");
                        }
                        RemoteClientState::Connected(c) => {
                            *client = Some(RemoteClientState::Connected(c));
                        }
                    }
                }
            }
            ClientMode::Disconnected => {}
        }

        Ok(())
    }

    // ============ 统一业务 API ============

    /// 通用 GET 请求 (使用 CrabClient)
    ///
    /// Server 模式: 使用 In-Process 客户端
    /// Client 模式: 使用 mTLS edge_http_client
    pub async fn get<T>(&self, path: &str) -> Result<T, BridgeError>
    where
        T: serde::de::DeserializeOwned,
    {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { client, .. } => match client {
                Some(LocalClientState::Authenticated(auth)) => {
                    auth.get(path).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Client {
                client, edge_url, ..
            } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    let http = auth
                        .edge_http_client()
                        .ok_or_else(|| BridgeError::NotInitialized)?;
                    let token = auth.token().ok_or_else(|| BridgeError::NotAuthenticated)?;
                    let url = format!("{}{}", edge_url, path);

                    let resp = http
                        .get(&url)
                        .header("Authorization", format!("Bearer {}", token))
                        .send()
                        .await
                        .map_err(|e| BridgeError::Server(e.to_string()))?;

                    if !resp.status().is_success() {
                        let text = resp.text().await.unwrap_or_default();
                        return Err(BridgeError::Server(text));
                    }

                    resp.json::<T>()
                        .await
                        .map_err(|e| BridgeError::Server(e.to_string()))
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// 通用 POST 请求 (使用 CrabClient)
    pub async fn post<T, B>(&self, path: &str, body: &B) -> Result<T, BridgeError>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize + Sync,
    {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { client, .. } => match client {
                Some(LocalClientState::Authenticated(auth)) => {
                    auth.post(path, body).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Client {
                client, edge_url, ..
            } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    let http = auth
                        .edge_http_client()
                        .ok_or_else(|| BridgeError::NotInitialized)?;
                    let token = auth.token().ok_or_else(|| BridgeError::NotAuthenticated)?;
                    let url = format!("{}{}", edge_url, path);

                    let resp = http
                        .post(&url)
                        .header("Authorization", format!("Bearer {}", token))
                        .json(body)
                        .send()
                        .await
                        .map_err(|e| BridgeError::Server(e.to_string()))?;

                    if !resp.status().is_success() {
                        let text = resp.text().await.unwrap_or_default();
                        return Err(BridgeError::Server(text));
                    }

                    resp.json::<T>()
                        .await
                        .map_err(|e| BridgeError::Server(e.to_string()))
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// 通用 PUT 请求 (使用 CrabClient)
    pub async fn put<T, B>(&self, path: &str, body: &B) -> Result<T, BridgeError>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize + Sync,
    {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { client, .. } => match client {
                Some(LocalClientState::Authenticated(auth)) => {
                    auth.put(path, body).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Client {
                client, edge_url, ..
            } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    let http = auth
                        .edge_http_client()
                        .ok_or_else(|| BridgeError::NotInitialized)?;
                    let token = auth.token().ok_or_else(|| BridgeError::NotAuthenticated)?;
                    let url = format!("{}{}", edge_url, path);

                    let resp = http
                        .put(&url)
                        .header("Authorization", format!("Bearer {}", token))
                        .json(body)
                        .send()
                        .await
                        .map_err(|e| BridgeError::Server(e.to_string()))?;

                    if !resp.status().is_success() {
                        let text = resp.text().await.unwrap_or_default();
                        return Err(BridgeError::Server(text));
                    }

                    resp.json::<T>()
                        .await
                        .map_err(|e| BridgeError::Server(e.to_string()))
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// 通用 DELETE 请求 (使用 CrabClient)
    pub async fn delete<T>(&self, path: &str) -> Result<T, BridgeError>
    where
        T: serde::de::DeserializeOwned,
    {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { client, .. } => match client {
                Some(LocalClientState::Authenticated(auth)) => {
                    auth.delete(path).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Client {
                client, edge_url, ..
            } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    let http = auth
                        .edge_http_client()
                        .ok_or_else(|| BridgeError::NotInitialized)?;
                    let token = auth.token().ok_or_else(|| BridgeError::NotAuthenticated)?;
                    let url = format!("{}{}", edge_url, path);

                    let resp = http
                        .delete(&url)
                        .header("Authorization", format!("Bearer {}", token))
                        .send()
                        .await
                        .map_err(|e| BridgeError::Server(e.to_string()))?;

                    if !resp.status().is_success() {
                        let text = resp.text().await.unwrap_or_default();
                        return Err(BridgeError::Server(text));
                    }

                    resp.json::<T>()
                        .await
                        .map_err(|e| BridgeError::Server(e.to_string()))
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// 通用 DELETE 请求 (带 body)
    pub async fn delete_with_body<T, B>(&self, path: &str, body: &B) -> Result<T, BridgeError>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize + Sync,
    {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { client, .. } => match client {
                Some(LocalClientState::Authenticated(auth)) => auth
                    .delete_with_body(path, body)
                    .await
                    .map_err(BridgeError::Client),
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Client {
                client, edge_url, ..
            } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    let http = auth
                        .edge_http_client()
                        .ok_or_else(|| BridgeError::NotInitialized)?;
                    let token = auth.token().ok_or_else(|| BridgeError::NotAuthenticated)?;
                    let url = format!("{}{}", edge_url, path);

                    let resp = http
                        .delete(&url)
                        .header("Authorization", format!("Bearer {}", token))
                        .json(body)
                        .send()
                        .await
                        .map_err(|e| BridgeError::Server(e.to_string()))?;

                    if !resp.status().is_success() {
                        let text = resp.text().await.unwrap_or_default();
                        return Err(BridgeError::Server(text));
                    }

                    resp.json::<T>()
                        .await
                        .map_err(|e| BridgeError::Server(e.to_string()))
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }
}
