use dashmap::DashMap;
use shared::message::{BusMessage, SyncPayload};
use std::path::PathBuf;
use std::sync::Arc;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::auth::JwtService;
use crate::core::Config;
use crate::core::config::migrate_legacy_structure;
use crate::db::DbService;
use crate::orders::OrdersManager;
use crate::pricing::PriceRuleEngine;
use crate::services::{
    ActivationService, CertService, HttpsService, MessageBusService, ProvisioningService,
};

/// 资源版本管理器
///
/// 使用 DashMap 实现无锁并发的版本号管理。
/// 每种资源类型维护独立的版本号，支持原子递增。
///
/// # 使用场景
///
/// 用于 broadcast_sync 时自动生成递增的版本号，
/// 确保客户端可以通过版本号判断数据新旧。
#[derive(Debug)]
pub struct ResourceVersions {
    versions: DashMap<String, u64>,
}

impl ResourceVersions {
    /// 创建空的版本管理器
    pub fn new() -> Self {
        Self {
            versions: DashMap::new(),
        }
    }

    /// 递增指定资源的版本号并返回新值
    ///
    /// 如果资源不存在，从 0 开始递增（返回 1）
    pub fn increment(&self, resource: &str) -> u64 {
        let mut entry = self.versions.entry(resource.to_string()).or_insert(0);
        *entry += 1;
        *entry
    }

    /// 获取指定资源的当前版本号
    ///
    /// 如果资源不存在，返回 0
    pub fn get(&self, resource: &str) -> u64 {
        self.versions.get(resource).map(|v| *v).unwrap_or(0)
    }
}

impl Default for ResourceVersions {
    fn default() -> Self {
        Self::new()
    }
}

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
/// | resource_versions | Arc<ResourceVersions> | 资源版本管理 |
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
    /// 资源版本管理器 (用于 broadcast_sync 自动递增版本号)
    pub resource_versions: Arc<ResourceVersions>,
    /// 订单管理器 (事件溯源)
    pub orders_manager: Arc<OrdersManager>,
    /// 价格规则引擎
    pub price_rule_engine: PriceRuleEngine,
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
        resource_versions: Arc<ResourceVersions>,
        orders_manager: Arc<OrdersManager>,
        price_rule_engine: PriceRuleEngine,
    ) -> Self {
        Self {
            config,
            db,
            activation,
            cert_service,
            message_bus,
            https,
            jwt_service,
            resource_versions,
            orders_manager,
            price_rule_engine,
        }
    }

    /// 初始化服务器状态
    ///
    /// 按顺序初始化：
    /// 1. 工作目录结构 (确保目录存在，迁移旧结构)
    /// 2. 数据库 (work_dir/database/crab.db)
    /// 3. 各服务 (Activation, Cert, MessageBus, HTTPS, JWT)
    /// 4. HTTPS 服务延迟初始化
    ///
    /// # Panics
    ///
    /// 数据库初始化失败时 panic
    pub async fn initialize(config: &Config) -> Self {
        // 0. Ensure work_dir structure exists
        config
            .ensure_work_dir_structure()
            .expect("Failed to create work directory structure");

        // 0.1 Migrate legacy structure if needed
        let work_dir = PathBuf::from(&config.work_dir);
        migrate_legacy_structure(&work_dir).expect("Failed to migrate legacy directory structure");

        // 1. Initialize DB
        // Use work_dir/database/crab.db for database path
        let db_dir = config.database_dir();
        let db_path = db_dir.join("crab.db");
        let db_path_str = db_path.to_string_lossy();

        let db_service = DbService::new(&db_path_str)
            .await
            .expect("Failed to initialize database");
        let db = db_service.db;

        // 2. Initialize Services
        let activation =
            ActivationService::new(config.auth_server_url.clone(), config.auth_storage_dir());
        let cert_service = CertService::new(PathBuf::from(&config.work_dir));
        let message_bus = MessageBusService::new(config);
        let https = HttpsService::new(config.clone());
        let jwt_service = Arc::new(JwtService::default());
        let resource_versions = Arc::new(ResourceVersions::new());

        // 3. Initialize OrdersManager (event sourcing)
        let orders_db_path = db_dir.join("orders.redb");
        let orders_manager = Arc::new(
            OrdersManager::new(&orders_db_path).expect("Failed to initialize orders manager"),
        );

        // 4. Initialize PriceRuleEngine
        let price_rule_engine = PriceRuleEngine::new(db.clone());

        let state = Self::new(
            config.clone(),
            db,
            activation,
            cert_service,
            message_bus,
            https.clone(),
            jwt_service,
            resource_versions,
            orders_manager,
            price_rule_engine,
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
    /// - 订单事件转发器 (Order Event Forwarder)
    pub async fn start_background_tasks(&self) {
        // Start MessageBus background tasks
        self.message_bus.start_background_tasks(self.clone());

        // Start order event forwarder (OrderEvent -> MessageBus)
        self.start_order_event_forwarder();
    }

    /// 启动订单事件转发器
    ///
    /// 订阅 OrdersManager 的事件流，转发到 MessageBus 以广播给所有客户端
    fn start_order_event_forwarder(&self) {
        let mut event_rx = self.orders_manager.subscribe();
        let message_bus = self.message_bus.bus().clone();

        tokio::spawn(async move {
            tracing::info!("📦 Order event forwarder started");
            loop {
                match event_rx.recv().await {
                    Ok(event) => {
                        // Convert OrderEvent to BusMessage (as Sync)
                        let payload = SyncPayload {
                            resource: "order_event".to_string(),
                            version: event.sequence,
                            action: event.event_type.to_string(),
                            id: event.order_id.clone(),
                            data: serde_json::to_value(&event).ok(),
                        };
                        if let Err(e) = message_bus.publish(BusMessage::sync(&payload)).await {
                            tracing::warn!("Failed to forward order event: {}", e);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Order event forwarder lagged, skipped {} events", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("Order event channel closed, forwarder stopping");
                        break;
                    }
                }
            }
        });
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

    /// 广播同步消息
    ///
    /// 向所有连接的客户端广播资源变更通知。
    /// 版本号由 ResourceVersions 自动递增管理。
    ///
    /// # 参数
    /// - `resource`: 资源类型 (如 "tag", "product", "category")
    /// - `action`: 变更类型 ("created", "updated", "deleted")
    /// - `id`: 资源 ID
    /// - `data`: 资源数据 (deleted 时为 None)
    pub async fn broadcast_sync<T: serde::Serialize>(
        &self,
        resource: &str,
        action: &str,
        id: &str,
        data: Option<&T>,
    ) {
        let version = self.resource_versions.increment(resource);
        let payload = SyncPayload {
            resource: resource.to_string(),
            version,
            action: action.to_string(),
            id: id.to_string(),
            data: data.and_then(|d| serde_json::to_value(d).ok()),
        };
        tracing::info!(resource = %resource, action = %action, id = %id, "Broadcasting sync event");
        match self.message_bus().publish(BusMessage::sync(&payload)).await {
            Ok(_) => tracing::debug!("Sync broadcast successful"),
            Err(e) => tracing::error!("Sync broadcast failed: {}", e),
        }
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

    /// 获取订单管理器
    pub fn orders_manager(&self) -> &Arc<OrdersManager> {
        &self.orders_manager
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

    /// 进入未绑定状态
    ///
    /// 当证书或配置损坏时调用，清理所有状态等待重新激活
    pub async fn enter_unbound_state(&self) {
        self.activation
            .enter_unbound_state_public(&self.cert_service)
            .await;
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
            tracing::info!("  Server ID    : {}", c.binding.entity_id);
            tracing::info!("  Tenant ID    : {}", c.binding.tenant_id);
            tracing::info!("  Device ID    : {}", c.binding.device_id);
            if let Some(sub) = &c.subscription {
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
