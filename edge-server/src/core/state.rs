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
use crate::db::repository::{CategoryRepository, ProductRepository};
use crate::orders::OrdersManager;
use crate::orders::actions::open_table::load_matching_rules;
use crate::pricing::PriceRuleEngine;
use crate::printing::{
    CategoryPrintConfig, KitchenPrintService, PrintConfigCache, PrintStorage, ProductPrintConfig,
};
use crate::services::{
    ActivationService, CertService, HttpsService, MessageBusService, ProvisioningService,
};
use shared::order::OrderEventType;

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

        // 5. Initialize KitchenPrintService
        let print_db_path = db_dir.join("print.redb");
        let print_storage =
            PrintStorage::open(&print_db_path).expect("Failed to initialize print storage");
        let print_config_cache = PrintConfigCache::new();
        let kitchen_print_service = Arc::new(KitchenPrintService::new(
            print_storage,
            print_config_cache,
        ));

        // 6. Generate epoch (UUID for server restart detection)
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
            epoch,
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
    /// - 价格规则缓存预热 (为活跃订单加载规则)
    /// - 打印配置缓存预热 (加载商品/分类打印配置)
    /// - 消息总线处理器 (MessageHandler)
    /// - 订单事件转发器 (Order Event Forwarder)
    /// - 厨房打印事件监听器 (Kitchen Print Event Listener)
    /// - 同步事件监听器 (Sync Event -> Cache Update)
    pub async fn start_background_tasks(&self) {
        // Warmup: Load price rules for all active orders
        self.warmup_active_order_rules().await;

        // Warmup: Load print config for all products/categories
        self.warmup_print_config_cache().await;

        // Warmup: Load product metadata (category_id, tags) for rule matching
        self.warmup_product_metadata_cache().await;

        // Start MessageBus background tasks
        self.message_bus.start_background_tasks(self.clone());

        // Start order event forwarder (OrderEvent -> MessageBus)
        self.start_order_event_forwarder();

        // Start kitchen print event listener (ItemsAdded -> Print)
        self.start_kitchen_print_event_listener();

        // Start sync event listener (product/category changes -> cache update)
        self.start_sync_event_listener();

        // Start print record cleanup task (cleanup records older than 3 days)
        self.start_print_record_cleanup_task();
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

    /// 预热打印配置缓存
    ///
    /// 服务器启动时调用，加载所有商品和分类的打印配置到内存缓存。
    pub async fn warmup_print_config_cache(&self) {
        let product_repo = ProductRepository::new(self.db.clone());
        let category_repo = CategoryRepository::new(self.db.clone());

        // Load categories first (products reference category_id)
        match category_repo.find_all_with_destinations().await {
            Ok(categories) => {
                for cat in &categories {
                    let id = cat
                        .id
                        .as_ref()
                        .map(|t| t.to_string())
                        .unwrap_or_default();

                    // Get destination IDs from the Thing references (full "table:id" format)
                    let kitchen_destinations: Vec<String> = cat
                        .kitchen_print_destinations
                        .iter()
                        .map(|t| t.to_string())
                        .collect();
                    let label_destinations: Vec<String> = cat
                        .label_print_destinations
                        .iter()
                        .map(|t| t.to_string())
                        .collect();

                    let config = CategoryPrintConfig {
                        category_id: id,
                        category_name: cat.name.clone(),
                        kitchen_print_destinations: kitchen_destinations,
                        label_print_destinations: label_destinations,
                        is_kitchen_print_enabled: cat.is_kitchen_print_enabled,
                        is_label_print_enabled: cat.is_label_print_enabled,
                    };
                    self.kitchen_print_service
                        .config_cache()
                        .update_category(config)
                        .await;
                }
                tracing::info!(
                    "🖨️ Loaded {} category print configs",
                    categories.len()
                );
            }
            Err(e) => {
                tracing::error!("Failed to load categories for print config: {:?}", e);
            }
        }

        // Load products
        match product_repo.find_all_with_destinations().await {
            Ok(products) => {
                for prod in &products {
                    let id = prod
                        .id
                        .as_ref()
                        .map(|t| t.to_string())
                        .unwrap_or_default();

                    // Get destination IDs (full "table:id" format)
                    let kitchen_destinations: Vec<String> = prod
                        .kitchen_print_destinations
                        .iter()
                        .map(|t| t.to_string())
                        .collect();
                    let label_destinations: Vec<String> = prod
                        .label_print_destinations
                        .iter()
                        .map(|t| t.to_string())
                        .collect();

                    // Get category ID from the Thing reference (full "table:id" format)
                    let category_id = prod.category.to_string();

                    // Get root spec external_id (find spec where is_root == true)
                    let root_spec_external_id = prod
                        .specs
                        .iter()
                        .find(|s| s.is_root)
                        .and_then(|s| s.external_id);

                    let config = ProductPrintConfig {
                        product_id: id,
                        product_name: prod.name.clone(),
                        kitchen_name: prod
                            .kitchen_print_name
                            .clone()
                            .unwrap_or_else(|| prod.name.clone()),
                        kitchen_print_destinations: kitchen_destinations,
                        label_print_destinations: label_destinations,
                        is_kitchen_print_enabled: prod.is_kitchen_print_enabled,
                        is_label_print_enabled: prod.is_label_print_enabled,
                        root_spec_external_id,
                        category_id,
                    };
                    self.kitchen_print_service
                        .config_cache()
                        .update_product(config)
                        .await;
                }
                tracing::info!(
                    "🖨️ Loaded {} product print configs",
                    products.len()
                );
            }
            Err(e) => {
                tracing::error!("Failed to load products for print config: {:?}", e);
            }
        }
    }

    /// 预热产品元数据缓存
    ///
    /// 服务器启动时调用，加载所有产品的 category_id 和 tags 到 OrdersManager 的缓存。
    /// 这样价格规则匹配时可以使用 Category 和 Tag 作用域。
    pub async fn warmup_product_metadata_cache(&self) {
        use crate::orders::manager::ProductMeta;
        use std::collections::HashMap;

        let product_repo = ProductRepository::new(self.db.clone());

        match product_repo.find_all().await {
            Ok(products) => {
                let mut metadata: HashMap<String, ProductMeta> = HashMap::new();

                for prod in &products {
                    let product_id = prod
                        .id
                        .as_ref()
                        .map(|t| t.to_string())
                        .unwrap_or_default();

                    if product_id.is_empty() {
                        continue;
                    }

                    // Extract category_id as String (Thing format: "category:xxx")
                    let category_id = prod.category.to_string();

                    // Extract tags as Vec<String> (Thing format: "tag:xxx")
                    let tags: Vec<String> = prod.tags.iter().map(|t| t.to_string()).collect();

                    metadata.insert(
                        product_id.clone(),
                        ProductMeta { category_id, tags },
                    );
                }

                if !metadata.is_empty() {
                    self.orders_manager.cache_product_metadata_batch(metadata.clone());
                    tracing::info!(
                        "📦 Loaded {} product metadata entries for rule matching",
                        metadata.len()
                    );
                } else {
                    tracing::debug!("No products found for metadata warmup");
                }
            }
            Err(e) => {
                tracing::error!("Failed to load products for metadata cache: {:?}", e);
            }
        }
    }

    /// 启动厨房打印事件监听器
    ///
    /// 订阅 OrdersManager 的事件流，处理 ItemsAdded 事件：
    /// - 检查打印是否启用
    /// - 创建 KitchenOrder 和 LabelPrintRecord
    fn start_kitchen_print_event_listener(&self) {
        use crate::db::repository::PrintDestinationRepository;
        use crate::printing::PrintExecutor;
        use std::collections::HashMap;

        let mut event_rx = self.orders_manager.subscribe();
        let kitchen_print_service = self.kitchen_print_service.clone();
        let orders_manager = self.orders_manager.clone();
        let db = self.db.clone();

        tokio::spawn(async move {
            tracing::info!("🖨️ Kitchen print event listener started");
            let executor = PrintExecutor::new();

            loop {
                match event_rx.recv().await {
                    Ok(event) => {
                        // Only process ItemsAdded events
                        if event.event_type != OrderEventType::ItemsAdded {
                            continue;
                        }

                        // Get table name from order snapshot
                        let table_name = orders_manager
                            .get_snapshot(&event.order_id)
                            .ok()
                            .flatten()
                            .and_then(|s| s.table_name);

                        // Process the event (create KitchenOrder record)
                        match kitchen_print_service
                            .process_items_added(&event, table_name)
                            .await
                        {
                            Ok(Some(kitchen_order_id)) => {
                                tracing::info!(
                                    order_id = %event.order_id,
                                    kitchen_order_id = %kitchen_order_id,
                                    "🖨️ Created kitchen order"
                                );

                                // Execute actual printing
                                if let Ok(Some(order)) = kitchen_print_service.get_kitchen_order(&kitchen_order_id) {
                                    // Load print destinations
                                    let repo = PrintDestinationRepository::new(db.clone());
                                    match repo.find_all().await {
                                        Ok(destinations) => {
                                            let dest_map: HashMap<String, _> = destinations
                                                .into_iter()
                                                .filter_map(|d| {
                                                    d.id.as_ref()
                                                        .map(|id| (id.to_string(), d.clone()))
                                                })
                                                .collect();

                                            if let Err(e) = executor.print_kitchen_order(&order, &dest_map).await {
                                                tracing::error!(
                                                    kitchen_order_id = %kitchen_order_id,
                                                    error = %e,
                                                    "Failed to execute print job"
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                error = ?e,
                                                "Failed to load print destinations"
                                            );
                                        }
                                    }
                                }
                            }
                            Ok(None) => {
                                // Printing not enabled or no items to print
                            }
                            Err(e) => {
                                tracing::error!(
                                    order_id = %event.order_id,
                                    "Failed to process ItemsAdded for printing: {:?}",
                                    e
                                );
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Kitchen print listener lagged, skipped {} events", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("Order event channel closed, kitchen print listener stopping");
                        break;
                    }
                }
            }
        });
    }

    /// 启动同步事件监听器
    ///
    /// 订阅 MessageBus 的广播流，处理 product/category 变更：
    /// - 更新 PrintConfigCache 以保持缓存与数据库同步
    fn start_sync_event_listener(&self) {
        use crate::db::models::{Category, Product};
        use crate::orders::manager::ProductMeta;
        use shared::message::EventType;

        let mut sync_rx = self.message_bus.bus().subscribe();
        let kitchen_print_service = self.kitchen_print_service.clone();
        let orders_manager = self.orders_manager.clone();

        tokio::spawn(async move {
            tracing::info!("🔄 Sync event listener started (for print config & product metadata cache)");
            loop {
                match sync_rx.recv().await {
                    Ok(msg) => {
                        // Only process Sync events
                        if msg.event_type != EventType::Sync {
                            continue;
                        }

                        // Parse SyncPayload
                        let payload: SyncPayload = match serde_json::from_slice(&msg.payload) {
                            Ok(p) => p,
                            Err(_) => continue,
                        };

                        // Handle product changes
                        if payload.resource == "product" {
                            if let Some(data) = &payload.data {
                                if let Ok(product) = serde_json::from_value::<Product>(data.clone()) {
                                    let product_id = product.id.as_ref().map(|t| t.to_string()).unwrap_or_default();

                                    // Find root spec external_id
                                    let root_spec_external_id = product
                                        .specs
                                        .iter()
                                        .find(|s| s.is_root)
                                        .and_then(|s| s.external_id);

                                    let config = ProductPrintConfig {
                                        product_id,
                                        product_name: product.name.clone(),
                                        kitchen_name: product
                                            .kitchen_print_name
                                            .clone()
                                            .unwrap_or_else(|| product.name.clone()),
                                        kitchen_print_destinations: product.kitchen_print_destinations.iter().map(|t| t.to_string()).collect(),
                                        label_print_destinations: product.label_print_destinations.iter().map(|t| t.to_string()).collect(),
                                        is_kitchen_print_enabled: product.is_kitchen_print_enabled,
                                        is_label_print_enabled: product.is_label_print_enabled,
                                        root_spec_external_id,
                                        category_id: product.category.to_string(),
                                    };
                                    kitchen_print_service.config_cache().update_product(config).await;

                                    // Also update product metadata cache for rule matching
                                    let product_id = product.id.as_ref().map(|t| t.to_string()).unwrap_or_default();
                                    if !product_id.is_empty() {
                                        let meta = ProductMeta {
                                            category_id: product.category.to_string(),
                                            tags: product.tags.iter().map(|t| t.to_string()).collect(),
                                        };
                                        orders_manager.cache_product_meta(&product_id, meta);
                                    }

                                    tracing::debug!(
                                        product_id = %payload.id,
                                        action = %payload.action,
                                        "Updated product print config and metadata from sync"
                                    );
                                }
                            }
                        }

                        // Handle category changes
                        if payload.resource == "category" {
                            if let Some(data) = &payload.data {
                                if let Ok(category) = serde_json::from_value::<Category>(data.clone()) {
                                    let category_id = category.id.as_ref().map(|t| t.to_string()).unwrap_or_default();
                                    let config = CategoryPrintConfig {
                                        category_id,
                                        category_name: category.name.clone(),
                                        kitchen_print_destinations: category.kitchen_print_destinations.iter().map(|t| t.to_string()).collect(),
                                        label_print_destinations: category.label_print_destinations.iter().map(|t| t.to_string()).collect(),
                                        is_kitchen_print_enabled: category.is_kitchen_print_enabled,
                                        is_label_print_enabled: category.is_label_print_enabled,
                                    };
                                    kitchen_print_service.config_cache().update_category(config).await;
                                    tracing::debug!(
                                        category_id = %payload.id,
                                        action = %payload.action,
                                        "Updated category print config from sync"
                                    );
                                }
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Sync event listener lagged, skipped {} events", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("Server broadcast channel closed, sync listener stopping");
                        break;
                    }
                }
            }
        });
    }

    /// 启动打印记录清理任务
    ///
    /// - 启动时立即执行一次清理
    /// - 之后每小时执行一次
    /// - 清理 3 天以前的记录 (kitchen_order, label_record)
    fn start_print_record_cleanup_task(&self) {
        const CLEANUP_INTERVAL_SECS: u64 = 3600; // 1 hour
        const MAX_AGE_SECS: i64 = 3 * 24 * 3600; // 3 days

        let print_service = self.kitchen_print_service.clone();

        tokio::spawn(async move {
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

    /// 启动订单同步转发器
    ///
    /// 订阅 OrdersManager 的事件流，转发到 MessageBus：
    /// - order_sync: 包含 event (时间线) + snapshot (状态)
    fn start_order_event_forwarder(&self) {
        let mut event_rx = self.orders_manager.subscribe();
        let message_bus = self.message_bus.bus().clone();
        let orders_manager = self.orders_manager.clone();

        tokio::spawn(async move {
            tracing::info!("📦 Order sync forwarder started");
            loop {
                match event_rx.recv().await {
                    Ok(event) => {
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
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Order forwarder lagged, skipped {} events", n);
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
