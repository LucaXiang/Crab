//! ClientBridge - 统一的客户端桥接层
//!
//! 提供 Server/Client 模式的统一接口，屏蔽底层差异。
//! - Server 模式: 本地运行 edge-server，使用 In-Process 通信
//! - Client 模式: 连接远程 edge-server，使用 mTLS 通信

use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

use crab_client::{Authenticated, Connected, CrabClient, Local, Remote};
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
            serde_json::from_str(&content)
                .map_err(|e| BridgeError::Config(e.to_string()))
        } else {
            Ok(Self::default())
        }
    }

    /// 保存配置到文件
    pub fn save(&self, path: &std::path::Path) -> Result<(), BridgeError> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| BridgeError::Config(e.to_string()))?;
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

/// 客户端模式枚举
#[allow(dead_code)]
enum ClientMode {
    /// Server 模式: 本地运行 edge-server
    Server {
        server_state: Arc<ServerState>,
        client: Option<LocalClientState>,
    },
    /// Client 模式: 连接远程 edge-server
    Client {
        client: CrabClient<Remote, Authenticated>,
        edge_url: String,
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
}

impl ClientBridge {
    /// 创建新的 ClientBridge
    pub fn new(base_path: impl Into<PathBuf>, client_name: &str) -> Result<Self, BridgeError> {
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
        })
    }

    /// 保存当前配置
    async fn save_config(&self) -> Result<(), BridgeError> {
        let config = self.config.read().await;
        config.save(&self.config_path)
    }

    // ============ 模式管理 ============

    /// 获取当前模式信息
    pub async fn get_mode_info(&self) -> ModeInfo {
        let mode_guard = self.mode.read().await;
        let tenant_manager = self.tenant_manager.read().await;

        let (mode, is_connected, is_authenticated) = match &*mode_guard {
            ClientMode::Disconnected => (ModeType::Disconnected, false, false),
            ClientMode::Server { .. } => (ModeType::Server, true, true),
            ClientMode::Client { .. } => (ModeType::Client, true, true),
        };

        ModeInfo {
            mode,
            is_connected,
            is_authenticated,
            tenant_id: tenant_manager.current_tenant_id().map(|s| s.to_string()),
            username: tenant_manager.current_session().map(|s| s.username.clone()),
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

        // 创建 EdgeServer 配置
        let edge_config = edge_server::Config::builder()
            .work_dir(server_config.data_dir.to_string_lossy().to_string())
            .http_port(server_config.http_port)
            .message_tcp_port(server_config.message_port)
            .build();

        // 初始化 ServerState
        let server_state = ServerState::initialize(&edge_config).await;
        let state_arc = Arc::new(server_state);

        // 启动后台任务
        state_arc.start_background_tasks().await;

        // 获取 Router 和消息通道
        let router = state_arc.https_service().router()
            .ok_or_else(|| BridgeError::Server("Router not initialized".to_string()))?;

        let message_bus = state_arc.message_bus();
        let client_tx = message_bus.sender_to_server().clone();
        let server_tx = message_bus.sender().clone();

        // 创建 CrabClient<Local>
        let client = CrabClient::local()
            .with_router(router)
            .with_message_channels(client_tx, server_tx)
            .build()?;

        // 连接客户端
        let connected_client = client.connect().await?;

        tracing::info!(port = server_config.http_port, "Server mode initialized with In-Process client");

        *mode_guard = ClientMode::Server {
            server_state: state_arc,
            client: Some(LocalClientState::Connected(connected_client)),
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
        let cert_manager = tenant_manager.current_cert_manager()
            .ok_or(TenantError::NoTenantSelected)?;

        let config = self.config.read().await;
        let auth_url = config.client_config.as_ref()
            .map(|c| c.auth_url.clone())
            .unwrap_or_else(|| "https://auth.example.com".to_string());

        // 创建 CrabClient<Remote>
        let client = CrabClient::remote()
            .auth_server(&auth_url)
            .cert_path(cert_manager.cert_path())
            .client_name(tenant_manager.client_name())
            .build()?;

        // 使用缓存的证书重连
        let _connected_client = client.reconnect(message_addr).await?;

        tracing::info!(edge_url = %edge_url, "Client mode connected");

        // 暂时保存为 Disconnected，因为还没有登录
        *mode_guard = ClientMode::Disconnected;

        // 更新配置
        drop(config);
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
            ClientMode::Server { server_state: _, client } => {
                // 取出当前 client
                let current_client = client.take()
                    .ok_or(BridgeError::NotInitialized)?;

                match current_client {
                    LocalClientState::Connected(connected) => {
                        // 执行登录
                        match connected.login(username, password).await {
                            Ok(authenticated) => {
                                // 获取用户信息 - 使用 me() 和 token() 方法
                                let user_info = authenticated.me().cloned()
                                    .ok_or_else(|| BridgeError::Client(
                                        crab_client::ClientError::Auth("No user info after login".into())
                                    ))?;
                                let token = authenticated.token()
                                    .unwrap_or_default()
                                    .to_string();

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
                                let user_info = authenticated.me().cloned()
                                    .ok_or_else(|| BridgeError::Client(
                                        crab_client::ClientError::Auth("No user info after login".into())
                                    ))?;
                                let token = authenticated.token()
                                    .unwrap_or_default()
                                    .to_string();

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
            ClientMode::Client { .. } => {
                // Client 模式使用 TenantManager 的 HTTP 登录
                let mut tenant_manager = self.tenant_manager.write().await;
                let config = self.config.read().await;
                let edge_url = config.client_config.as_ref()
                    .map(|c| c.edge_url.clone())
                    .unwrap_or_else(|| "https://localhost:9625".to_string());
                drop(config);

                tenant_manager
                    .login_online(username, password, &edge_url)
                    .await
                    .map_err(BridgeError::Tenant)
            }
            ClientMode::Disconnected => {
                Err(BridgeError::NotInitialized)
            }
        }
    }

    /// 员工登出
    pub async fn logout_employee(&self) -> Result<(), BridgeError> {
        let mut mode_guard = self.mode.write().await;

        match &mut *mode_guard {
            ClientMode::Server { server_state: _, client } => {
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
            ClientMode::Client { .. } => {
                let mut tenant_manager = self.tenant_manager.write().await;
                tenant_manager.logout()?;
            }
            ClientMode::Disconnected => {}
        }

        Ok(())
    }

    // ============ 统一业务 API ============

    /// 通用 GET 请求 (使用 CrabClient)
    ///
    /// 此方法会根据当前模式选择合适的客户端：
    /// - Server 模式: 使用 In-Process 客户端
    /// - Client 模式: 使用 mTLS 远程客户端 (TODO)
    pub async fn get<T>(&self, path: &str) -> Result<T, BridgeError>
    where
        T: serde::de::DeserializeOwned,
    {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { client, .. } => {
                match client {
                    Some(LocalClientState::Authenticated(auth)) => {
                        auth.get(path)
                            .await
                            .map_err(BridgeError::Client)
                    }
                    _ => {
                        Err(BridgeError::NotAuthenticated)
                    }
                }
            }
            ClientMode::Client { .. } => {
                // TODO: 使用 Remote client 的 HTTP API
                Err(BridgeError::NotImplemented("Remote GET not implemented".into()))
            }
            ClientMode::Disconnected => {
                Err(BridgeError::NotInitialized)
            }
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
            ClientMode::Server { client, .. } => {
                match client {
                    Some(LocalClientState::Authenticated(auth)) => {
                        auth.post(path, body)
                            .await
                            .map_err(BridgeError::Client)
                    }
                    _ => {
                        Err(BridgeError::NotAuthenticated)
                    }
                }
            }
            ClientMode::Client { .. } => {
                // TODO: 使用 Remote client 的 HTTP API
                Err(BridgeError::NotImplemented("Remote POST not implemented".into()))
            }
            ClientMode::Disconnected => {
                Err(BridgeError::NotInitialized)
            }
        }
    }
}
