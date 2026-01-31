use dashmap::DashMap;
use shared::message::{BusMessage, SyncPayload};
use std::path::PathBuf;
use std::sync::Arc;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use tokio::sync::mpsc;

use crate::audit::{AuditService, AuditWorker};
use crate::auth::JwtService;
use crate::core::tasks::{BackgroundTasks, TaskKind};
use crate::core::Config;

use crate::db::DbService;
use crate::orders::{ArchiveWorker, OrdersManager};
use crate::orders::actions::open_table::load_matching_rules;
use crate::pricing::PriceRuleEngine;
use crate::printing::{KitchenPrintService, PrintStorage};
use crate::services::{
    ActivationService, CatalogService, CertService, HttpsService, MessageBusService,
    ProvisioningService,
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
    /// 厨房/标签打印服务
    pub kitchen_print_service: Arc<KitchenPrintService>,
    /// 产品和分类统一管理 (含内存缓存)
    pub catalog_service: Arc<CatalogService>,
    /// 审计日志服务 (税务级防篡改)
    pub audit_service: Arc<AuditService>,
    /// 服务器实例 epoch (启动时生成的 UUID)
    /// 用于客户端检测服务器重启
    pub epoch: String,
}

impl ServerState {
    /// 创建服务器状态 (手动构造)
    ///
    /// 通常使用 [`initialize()`] 方法代替
    #[allow(clippy::too_many_arguments)]
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
        kitchen_print_service: Arc<KitchenPrintService>,
        catalog_service: Arc<CatalogService>,
        audit_service: Arc<AuditService>,
        epoch: String,
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
            kitchen_print_service,
            catalog_service,
            audit_service,
            epoch,
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

        // 1. Initialize DB
        // Database path: {tenant}/server/data/main.db/
        let db_path = config.database_dir();
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

        // 3. Initialize CatalogService first (OrdersManager and PriceRuleEngine depend on it)
        let images_dir = config.images_dir();
        let catalog_service = Arc::new(CatalogService::new(db.clone(), images_dir));

        // 4. Initialize OrdersManager (event sourcing) with CatalogService
        let orders_db_path = config.orders_db_file();
        let mut orders_manager =
            OrdersManager::new(&orders_db_path).expect("Failed to initialize orders manager");
        orders_manager.set_catalog_service(catalog_service.clone());
        orders_manager.set_archive_service(db.clone());

        // Note: ArchiveWorker is started in start_background_tasks()

        let orders_manager = Arc::new(orders_manager);

        // 5. Initialize PriceRuleEngine
        let price_rule_engine = PriceRuleEngine::new(db.clone(), catalog_service.clone());

        // 6. Initialize KitchenPrintService
        let print_db_path = config.print_db_file();
        let print_storage =
            PrintStorage::open(&print_db_path).expect("Failed to initialize print storage");
        let kitchen_print_service = Arc::new(KitchenPrintService::new(print_storage));

        // 7. Initialize AuditService (税务级审计日志 — SurrealDB)
        let data_dir = config.data_dir();
        let (audit_service, audit_rx) = AuditService::new(db.clone(), &data_dir, 1024);

        // 检测异常关闭和长时间停机（通过 LOCK 文件 + pending-ack.json）
        audit_service.on_startup().await;

        // 启动审计日志 worker
        let audit_worker = AuditWorker::new(audit_service.storage().clone());
        tokio::spawn(async move {
            audit_worker.run(audit_rx).await;
        });

        // 8. Generate epoch (UUID for server restart detection)
        let epoch = uuid::Uuid::new_v4().to_string();

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
            kitchen_print_service,
            catalog_service,
            audit_service,
            epoch,
        );

        // 3. Late initialization for HttpsService (needs state)
        https.initialize(state.clone());

        // 9. 记录系统启动审计日志
        state.audit_service.log(
            crate::audit::AuditAction::SystemStartup,
            "system",
            "server:main",
            None,
            None,
            serde_json::json!({"epoch": &state.epoch}),
        ).await;

        state
    }

    /// 启动后台任务
    ///
    /// 必须在 `Server::run()` 之前调用
    ///
    /// 启动的任务：
    /// - **Warmup**: CatalogService 预热, 价格规则缓存预热
    /// - **Worker**: ArchiveWorker, MessageHandler
    /// - **Listener**: 订单事件转发器, 厨房打印事件监听器
    /// - **Periodic**: 打印记录清理任务, 归档验证调度器, 班次自动关闭调度器
    ///
    /// 返回 `BackgroundTasks` 用于 graceful shutdown
    pub async fn start_background_tasks(&self) -> BackgroundTasks {
        use crate::core::EventRouter;

        let mut tasks = BackgroundTasks::new();

        // ═══════════════════════════════════════════════════════════════════
        // Warmup Tasks (同步执行，启动时运行一次)
        // ═══════════════════════════════════════════════════════════════════

        // Warmup: Load all products and categories into CatalogService cache
        if let Err(e) = self.catalog_service.warmup().await {
            tracing::error!("Failed to warmup CatalogService: {:?}", e);
        }

        // Warmup: Load price rules for all active orders
        self.warmup_active_order_rules().await;

        // ═══════════════════════════════════════════════════════════════════
        // Event Router (事件路由，解耦 OrdersManager 和各 Worker)
        // ═══════════════════════════════════════════════════════════════════

        // archive_buffer 较大（关键业务），其他 buffer 适中
        let (router, channels) = EventRouter::new(512, 256);
        let source_rx = self.orders_manager.subscribe();

        tasks.spawn("event_router", TaskKind::Worker, async move {
            router.run(source_rx).await;
        });

        // ═══════════════════════════════════════════════════════════════════
        // Worker Tasks (长期后台工作者)
        // ═══════════════════════════════════════════════════════════════════

        // ArchiveWorker: 归档已完成订单到 SurrealDB
        self.register_archive_worker(&mut tasks, channels.archive_rx);

        // MessageHandler: 处理来自客户端的消息
        self.register_message_handler(&mut tasks);

        // ═══════════════════════════════════════════════════════════════════
        // Listener Tasks (事件监听器)
        // ═══════════════════════════════════════════════════════════════════

        // OrderSyncForwarder: 订单事件 -> MessageBus
        self.register_order_sync_forwarder(&mut tasks, channels.sync_rx);

        // KitchenPrintWorker: ItemsAdded 事件 -> 厨房打印
        self.register_kitchen_print_worker(&mut tasks, channels.print_rx);

        // ═══════════════════════════════════════════════════════════════════
        // Periodic Tasks (定时任务)
        // ═══════════════════════════════════════════════════════════════════

        // PrintRecordCleanup: 清理过期打印记录
        self.register_print_record_cleanup(&mut tasks);

        // VerifyScheduler: 归档哈希链验证（启动补扫 + 每日触发）
        self.register_verify_scheduler(&mut tasks);

        // ShiftAutoCloseScheduler: 自动关闭跨营业日僵尸班次
        self.register_shift_auto_close(&mut tasks);

        // 打印任务摘要
        tasks.log_summary();

        tasks
    }

    /// 启动需要 TLS 的后台任务（激活后调用）
    ///
    /// 这些任务需要 mTLS 配置，必须在设备激活后启动。
    pub fn start_tls_tasks(
        &self,
        tasks: &mut BackgroundTasks,
        tls_config: Arc<rustls::ServerConfig>,
    ) {
        // MessageBus TCP Server (mTLS)
        let message_bus_service = self.message_bus.clone();
        tasks.spawn("message_bus_tcp_server", TaskKind::Worker, async move {
            if let Err(e) = message_bus_service.start_tcp_server(tls_config).await {
                tracing::error!("Message Bus TCP server failed: {}", e);
            }
        });

        tracing::info!("🔐 TLS tasks started (MessageBus TCP Server)");
    }

    /// 预热活跃订单的价格规则缓存
    ///
    /// 服务器启动时调用，确保所有活跃订单都有规则缓存。
    /// 这样 AddItems 命令可以立即使用缓存的规则。
    pub async fn warmup_active_order_rules(&self) {
        let active_orders = match self.orders_manager.get_active_orders() {
            Ok(orders) => orders,
            Err(e) => {
                tracing::error!("Failed to get active orders for rule warmup: {:?}", e);
                return;
            }
        };

        if active_orders.is_empty() {
            tracing::debug!("No active orders, skipping rule warmup");
            return;
        }

        tracing::info!(
            "🔥 Warming up price rules for {} active orders",
            active_orders.len()
        );

        let mut loaded_count = 0;
        for order in &active_orders {
            let rules = load_matching_rules(
                &self.db,
                order.zone_id.as_deref(),
                order.is_retail,
            )
            .await;

            if !rules.is_empty() {
                self.orders_manager.cache_rules(&order.order_id, rules);
                loaded_count += 1;
            }
        }

        tracing::info!(
            "✅ Rule warmup complete: {}/{} orders have cached rules",
            loaded_count,
            active_orders.len()
        );
    }

    /// 为单个订单加载并缓存价格规则
    ///
    /// 用于：
    /// - RestoreOrder 后重新加载规则
    /// - 手动刷新订单规则
    pub async fn load_rules_for_order(&self, order_id: &str) -> bool {
        let snapshot = match self.orders_manager.get_snapshot(order_id) {
            Ok(Some(s)) => s,
            Ok(None) => {
                tracing::warn!("Order {} not found for rule loading", order_id);
                return false;
            }
            Err(e) => {
                tracing::error!("Failed to get order {} for rule loading: {:?}", order_id, e);
                return false;
            }
        };

        let rules = load_matching_rules(
            &self.db,
            snapshot.zone_id.as_deref(),
            snapshot.is_retail,
        )
        .await;

        if !rules.is_empty() {
            tracing::debug!(
                order_id = %order_id,
                rule_count = rules.len(),
                "Loaded rules for order"
            );
            self.orders_manager.cache_rules(order_id, rules);
            true
        } else {
            // No rules to cache, but still valid
            true
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Task Registration Methods
    // ═══════════════════════════════════════════════════════════════════════

    /// 注册 ArchiveWorker
    ///
    /// 归档已完成的订单到 SurrealDB
    /// 接收来自 EventRouter 的 mpsc 通道（已过滤为终端事件）
    fn register_archive_worker(
        &self,
        tasks: &mut BackgroundTasks,
        event_rx: mpsc::Receiver<std::sync::Arc<shared::order::OrderEvent>>,
    ) {
        if let Some(archive_service) = self.orders_manager.archive_service() {
            let worker = ArchiveWorker::new(
                self.orders_manager.storage().clone(),
                archive_service.clone(),
                self.audit_service.clone(),
                self.db.clone(),
            );

            tasks.spawn("archive_worker", TaskKind::Worker, async move {
                worker.run(event_rx).await;
            });
        }
    }

    /// 注册 MessageHandler
    ///
    /// 处理来自客户端的消息
    fn register_message_handler(&self, tasks: &mut BackgroundTasks) {
        let handler_receiver = self.message_bus.bus().subscribe_to_clients();
        let handler_shutdown = self.message_bus.bus().shutdown_token().clone();
        let server_tx = self.message_bus.bus().sender().clone();

        let handler = crate::message::MessageHandler::with_default_processors(
            handler_receiver,
            handler_shutdown,
            self.clone().into(),
        )
        .with_broadcast_tx(server_tx);

        tasks.spawn("message_handler", TaskKind::Worker, async move {
            handler.run().await;
        });
    }

    /// 注册订单同步转发器
    ///
    /// 接收来自 EventRouter 的 mpsc 通道（所有事件），转发到 MessageBus
    fn register_order_sync_forwarder(
        &self,
        tasks: &mut BackgroundTasks,
        mut event_rx: mpsc::Receiver<std::sync::Arc<shared::order::OrderEvent>>,
    ) {
        let message_bus = self.message_bus.bus().clone();
        let orders_manager = self.orders_manager.clone();

        tasks.spawn("order_sync_forwarder", TaskKind::Listener, async move {
            tracing::info!("📦 Order sync forwarder started");

            while let Some(event) = event_rx.recv().await {
                let order_id = event.order_id.clone();
                let sequence = event.sequence;
                let action = event.event_type.to_string();

                // 获取快照，打包 event + snapshot 一起推送
                match orders_manager.get_snapshot(&order_id) {
                    Ok(Some(snapshot)) => {
                        let payload = SyncPayload {
                            resource: "order_sync".to_string(),
                            version: sequence,
                            action,
                            id: order_id,
                            data: serde_json::json!({
                                "event": event,
                                "snapshot": snapshot
                            }).into(),
                        };
                        if let Err(e) = message_bus.publish(BusMessage::sync(&payload)).await {
                            tracing::warn!("Failed to forward order sync: {}", e);
                        }
                    }
                    Ok(None) => {
                        tracing::warn!("Order {} not found after event", order_id);
                    }
                    Err(e) => {
                        tracing::error!("Failed to get snapshot for {}: {}", order_id, e);
                    }
                }
            }

            tracing::info!("Sync channel closed, order sync forwarder stopping");
        });
    }

    /// 注册厨房打印工作者
    ///
    /// 接收来自 EventRouter 的 mpsc 通道（仅 ItemsAdded 事件）
    fn register_kitchen_print_worker(
        &self,
        tasks: &mut BackgroundTasks,
        event_rx: mpsc::Receiver<std::sync::Arc<shared::order::OrderEvent>>,
    ) {
        use crate::printing::KitchenPrintWorker;

        let worker = KitchenPrintWorker::new(
            self.orders_manager.clone(),
            self.kitchen_print_service.clone(),
            self.catalog_service.clone(),
            self.db.clone(),
        );

        tasks.spawn("kitchen_print_worker", TaskKind::Listener, async move {
            worker.run(event_rx).await;
        });
    }


    /// 注册打印记录清理任务
    ///
    /// - 启动时立即执行一次清理
    /// - 之后每小时执行一次
    /// - 清理 3 天以前的记录 (kitchen_order, label_record)
    fn register_print_record_cleanup(&self, tasks: &mut BackgroundTasks) {
        const CLEANUP_INTERVAL_SECS: u64 = 3600; // 1 hour
        const MAX_AGE_SECS: i64 = 3 * 24 * 3600; // 3 days

        let print_service = self.kitchen_print_service.clone();

        tasks.spawn("print_record_cleanup", TaskKind::Periodic, async move {
            tracing::info!("🧹 Print record cleanup task started (interval: 1h, max_age: 3d)");

            // Cleanup immediately on startup
            match print_service.cleanup_old_records(MAX_AGE_SECS) {
                Ok(count) if count > 0 => {
                    tracing::info!("🧹 Cleaned up {} old print records on startup", count);
                }
                Ok(_) => {
                    tracing::debug!("No old print records to cleanup on startup");
                }
                Err(e) => {
                    tracing::error!("Failed to cleanup print records on startup: {:?}", e);
                }
            }

            // Then cleanup periodically
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(CLEANUP_INTERVAL_SECS));
            interval.tick().await; // Skip the first immediate tick (already cleaned up above)

            loop {
                interval.tick().await;
                match print_service.cleanup_old_records(MAX_AGE_SECS) {
                    Ok(count) if count > 0 => {
                        tracing::info!("🧹 Cleaned up {} old print records", count);
                    }
                    Ok(_) => {
                        tracing::debug!("No old print records to cleanup");
                    }
                    Err(e) => {
                        tracing::error!("Failed to cleanup print records: {:?}", e);
                    }
                }
            }
        });
    }

    /// 注册归档验证调度器
    ///
    /// - 启动时补扫未验证的营业日
    /// - 启动时检查是否需要全链扫描（>7 天未执行）
    /// - 运行期间按 business_day_cutoff 每日触发
    fn register_verify_scheduler(&self, tasks: &mut BackgroundTasks) {
        use crate::orders::VerifyScheduler;

        if let Some(archive_service) = self.orders_manager.archive_service() {
            let scheduler = VerifyScheduler::new(
                archive_service.clone(),
                self.db.clone(),
                tasks.shutdown_token(),
            );

            tasks.spawn("verify_scheduler", TaskKind::Periodic, async move {
                scheduler.run().await;
            });
        }
    }

    /// 注册班次自动关闭调度器
    ///
    /// - 启动时立即扫描关闭跨营业日僵尸班次
    /// - 运行期间按 business_day_cutoff 每日触发
    fn register_shift_auto_close(&self, tasks: &mut BackgroundTasks) {
        use crate::shifts::ShiftAutoCloseScheduler;

        let scheduler = ShiftAutoCloseScheduler::new(
            self.clone(),
            tasks.shutdown_token(),
        );

        tasks.spawn("shift_auto_close", TaskKind::Periodic, async move {
            scheduler.run().await;
        });
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Getter Methods
    // ═══════════════════════════════════════════════════════════════════════

    /// 获取数据库实例
    pub fn get_db(&self) -> Surreal<Db> {
        self.db.clone()
    }

    /// 获取工作目录
    pub fn work_dir(&self) -> PathBuf {
        PathBuf::from(&self.config.work_dir)
    }

    /// 获取图片目录: {tenant}/server/images/
    pub fn images_dir(&self) -> PathBuf {
        self.config.images_dir()
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

    /// 获取厨房打印服务
    pub fn kitchen_print_service(&self) -> &Arc<KitchenPrintService> {
        &self.kitchen_print_service
    }

    /// 检查是否已激活
    ///
    /// 激活 = 证书已加载且通过自检
    pub async fn is_activated(&self) -> bool {
        self.activation.is_activated().await
    }

    /// 检查激活状态（非阻塞）
    ///
    /// 返回 Ok(()) 如果已激活且自检通过
    /// 返回 Err 如果未激活或自检失败
    pub async fn check_activation(&self) -> Result<(), crate::utils::AppError> {
        self.activation
            .check_activation(&self.cert_service)
            .await
    }

    /// 等待激活（阻塞，可取消）
    ///
    /// 阻塞直到激活成功且自检通过。
    /// 用于 `Server::run()`，确保 HTTPS 只在激活后启动。
    /// 返回 `Err(())` 表示 shutdown 被请求。
    pub async fn wait_for_activation(
        &self,
        shutdown_token: &tokio_util::sync::CancellationToken,
    ) -> Result<(), ()> {
        self.activation
            .wait_for_activation(&self.cert_service, shutdown_token)
            .await
    }

    /// 检查订阅是否被阻止
    pub async fn is_subscription_blocked(&self) -> bool {
        self.activation.is_subscription_blocked().await
    }

    /// 获取订阅阻止信息 (供 Bridge 使用)
    ///
    /// 返回 None 表示未阻止
    pub async fn get_subscription_blocked_info(
        &self,
    ) -> Option<shared::app_state::SubscriptionBlockedInfo> {
        self.activation.get_subscription_blocked_info().await
    }

    /// 从 auth-server 同步订阅状态
    pub async fn sync_subscription(&self) {
        self.activation.sync_subscription().await;
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
    pub async fn print_subscription_blocked_banner(&self) {
        let cred = self.activation.get_credential().await.unwrap_or_default();
        if let Some(c) = cred {
            tracing::warn!(
                "╔══════════════════════════════════════════════════════════════════════╗"
            );
            tracing::warn!(
                "║              ⛔ SUBSCRIPTION BLOCKED - SERVICES STOPPED             ║"
            );
            tracing::warn!(
                "╚══════════════════════════════════════════════════════════════════════╝"
            );
            tracing::warn!("  Tenant ID    : {}", c.binding.tenant_id);
            if let Some(sub) = &c.subscription {
                tracing::warn!("  Subscription : {:?} ({:?})", sub.status, sub.plan);
            }
            tracing::warn!("  HTTPS Server : NOT STARTED");
            tracing::warn!("  Message Bus  : NOT STARTED");
            tracing::warn!("  Waiting 60s before re-checking...");
            tracing::warn!(
                "════════════════════════════════════════════════════════════════════════"
            );
        }
    }

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
