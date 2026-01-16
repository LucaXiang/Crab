use std::path::PathBuf;
use std::sync::Arc;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::auth::JwtService;
use crate::core::Config;
use crate::db::DbService;
use crate::services::{
    ActivationService, CertService, HttpsService, MessageBusService, ProvisioningService,
};

/// 服务器状态 - 持有所有服务的单例引用
///
/// ServerState 是边缘节点的核心数据结构，持有所有服务的共享引用。
/// 使用 Arc 实现浅拷贝，所有权成本极低。
///
/// # 服务组件
///
/// | 字段 | 类型 | 说明 |
/// |------|------|------|
/// | config | Config | 配置项 (不可变) |
/// | db | Surreal<Db> | 嵌入式数据库 |
/// | activation | ActivationService | 激活状态管理 |
/// | cert_service | CertService | 证书管理服务 |
/// | message_bus | MessageBusService | 消息总线服务 |
/// | https | HttpsService | HTTPS 服务 |
/// | jwt_service | Arc<JwtService> | JWT 认证服务 |
///
/// # 使用示例
///
/// ```ignore
/// // 获取数据库连接
/// let db = state.get_db();
///
/// // 获取消息总线
/// let bus = state.message_bus();
///
/// // 检查激活状态
/// if state.is_activated().await {
///     println!("服务器已激活");
/// }
/// ```
#[derive(Clone, Debug)]
pub struct ServerState {
    /// 服务器配置
    pub config: Config,
    /// 嵌入式数据库 (SurrealDB)
    pub db: Surreal<Db>,
    /// 激活状态管理
    pub activation: ActivationService,
    /// 证书管理服务 (mTLS)
    pub cert_service: CertService,
    /// 消息总线服务
    pub message_bus: MessageBusService,
    /// HTTPS 服务
    pub https: HttpsService,
    /// JWT 认证服务 (Arc 共享所有权)
    pub jwt_service: Arc<JwtService>,
}

impl ServerState {
    /// 创建服务器状态 (手动构造)
    ///
    /// 通常使用 [`initialize()`] 方法代替
    pub fn new(
        config: Config,
        db: Surreal<Db>,
        activation: ActivationService,
        cert_service: CertService,
        message_bus: MessageBusService,
        https: HttpsService,
        jwt_service: Arc<JwtService>,
    ) -> Self {
        Self {
            config,
            db,
            activation,
            cert_service,
            message_bus,
            https,
            jwt_service,
        }
    }

    /// 初始化服务器状态
    ///
    /// 按顺序初始化：
    /// 1. 数据库 (work_dir/crab.db)
    /// 2. 各服务 (Activation, Cert, MessageBus, HTTPS, JWT)
    /// 3. HTTPS 服务延迟初始化
    ///
    /// # Panics
    ///
    /// 数据库初始化失败时 panic
    pub async fn initialize(config: &Config) -> Self {
        // 1. Initialize DB
        // Use work_dir/crab.db for database path
        let db_path = PathBuf::from(&config.work_dir).join("crab.db");
        let db_path_str = db_path.to_string_lossy();

        let db_service = DbService::new(&db_path_str)
            .await
            .expect("Failed to initialize database");
        let db = db_service.db;

        // 2. Initialize Services
        let activation = ActivationService::new(
            config.auth_server_url.clone(),
            PathBuf::from(&config.work_dir),
        );
        let cert_service = CertService::new(PathBuf::from(&config.work_dir));
        let message_bus = MessageBusService::new(config);
        let https = HttpsService::new(config.clone());
        let jwt_service = Arc::new(JwtService::default());

        let state = Self::new(
            config.clone(),
            db,
            activation,
            cert_service,
            message_bus,
            https.clone(),
            jwt_service,
        );

        // 3. Late initialization for HttpsService (needs state)
        https.initialize(state.clone());

        state
    }

    /// 启动后台任务
    ///
    /// 必须在 `Server::run()` 之前调用
    ///
    /// 启动的任务：
    /// - 消息总线处理器 (MessageHandler)
    pub async fn start_background_tasks(&self) {
        // Start MessageBus background tasks
        self.message_bus.start_background_tasks(self.clone());
    }

    /// 获取数据库实例
    pub fn get_db(&self) -> Surreal<Db> {
        self.db.clone()
    }

    /// 获取工作目录
    pub fn work_dir(&self) -> PathBuf {
        PathBuf::from(&self.config.work_dir)
    }

    /// 获取 JWT 服务
    pub fn get_jwt_service(&self) -> Arc<JwtService> {
        self.jwt_service.clone()
    }

    /// 获取消息总线
    pub fn message_bus(&self) -> &Arc<crate::message::MessageBus> {
        self.message_bus.bus()
    }

    /// 获取激活服务
    pub fn activation_service(&self) -> &ActivationService {
        &self.activation
    }

    /// 获取证书服务
    pub fn cert_service(&self) -> &CertService {
        &self.cert_service
    }

    /// 获取 HTTPS 服务
    pub fn https_service(&self) -> &HttpsService {
        &self.https
    }

    /// 检查是否已激活
    ///
    /// 激活 = 证书已加载且通过自检
    pub async fn is_activated(&self) -> bool {
        self.activation.is_activated().await
    }

    /// 等待激活信号
    ///
    /// 如果未激活，阻塞等待 `notify.notified()`
    /// 激活成功后返回，继续启动服务
    pub async fn wait_for_activation(&self) {
        self.activation
            .wait_for_activation(&self.cert_service)
            .await
    }

    /// 创建预配服务 (用于边缘激活)
    pub fn provisioning_service(&self, auth_url: String) -> ProvisioningService {
        ProvisioningService::new(self.clone(), auth_url)
    }

    /// 加载 TLS 配置 (mTLS)
    ///
    /// 用于启动 TCP 消息总线和 HTTPS 服务器
    pub fn load_tls_config(
        &self,
    ) -> Result<Option<Arc<rustls::ServerConfig>>, crate::utils::AppError> {
        self.cert_service.load_tls_config()
    }

    /// 保存证书 (边缘激活时由 ProvisioningService 调用)
    ///
    /// 保存到 work_dir/certs/ 目录
    pub async fn save_certificates(
        &self,
        root_ca_pem: &str,
        tenant_ca_pem: &str,
        edge_cert_pem: &str,
        edge_key_pem: &str,
    ) -> Result<(), crate::utils::AppError> {
        self.cert_service
            .save_certificates(root_ca_pem, tenant_ca_pem, edge_cert_pem, edge_key_pem)
            .await
    }

    /// 停用并重置
    ///
    /// 删除证书文件，清理激活状态
    pub async fn deactivate_and_reset(&self) -> Result<(), crate::utils::AppError> {
        self.cert_service.delete_certificates()?;
        self.activation.deactivate_and_reset().await
    }

    /// 打印激活后的横幅内容 (日志)
    pub async fn print_activated_banner_content(&self) {
        let cred = self.activation.get_credential().await.unwrap_or_default();
        if let Some(c) = cred {
            tracing::info!(
                "╔══════════════════════════════════════════════════════════════════════╗"
            );
            tracing::info!(
                "║                    CRAB EDGE SERVER - ACTIVATED                      ║"
            );
            tracing::info!(
                "╚══════════════════════════════════════════════════════════════════════╝"
            );
            tracing::info!("  Server ID    : {}", c.server_id);
            tracing::info!("  Tenant ID    : {}", c.tenant_id);
            if let Some(device_id) = c.device_id {
                tracing::info!("  Device ID    : {}", device_id);
            }
            if let Some(sub) = c.subscription {
                tracing::info!("  Subscription : {:?} ({:?})", sub.status, sub.plan);
            }
            tracing::info!(
                "  HTTPS Server  : https://localhost:{}",
                self.config.http_port
            );
            tracing::info!(
                "  Message Bus  : tcp://localhost:{} (mTLS)",
                self.config.message_tcp_port
            );
            tracing::info!(
                "════════════════════════════════════════════════════════════════════════"
            );
        } else {
            tracing::warn!("Server activated but credential not found in cache!");
        }
    }
}
