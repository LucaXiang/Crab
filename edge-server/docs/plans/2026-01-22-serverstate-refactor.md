# Edge Server 重构设计文档

> 日期: 2026-01-22
> 状态: MVP 规划中

## 1. 背景与问题

### 1.1 当前问题

`ServerState` 是一个"上帝对象"，存在以下问题：

| 问题 | 描述 |
|------|------|
| 职责过重 | 700+ 行代码，包含 12 个字段和大量业务逻辑 |
| 后台任务分散 | 打印监听、同步监听、事件转发都在 `state.rs` 中 |
| 紧耦合 | 各服务依赖整个 `ServerState`，难以单独测试 |
| 预热逻辑混杂 | `warmup_*` 函数与服务初始化混在一起 |

### 1.2 重构目标 (按优先级)

1. **可测试性** - 各组件能独立单元测试
2. **可维护性** - 职责分离，新功能容易添加
3. **性能** - 按需初始化，减少启动开销
4. **代码复用** - 服务可在不同场景复用

### 1.3 设计原则

- **不过度设计** - 边缘服务器，局域网运行，保持简单
- **Builder + 轻量 Trait** - 编译期类型安全，必要时才抽象
- **服务自管理** - 各服务管理自己的后台任务
- **渐进迁移** - 每步可独立编译验证

---

## 2. 架构设计

### 2.1 目录结构 (改造后)

```
src/
├── core/
│   ├── mod.rs
│   ├── config.rs            # 不变
│   ├── server.rs            # 不变
│   ├── context.rs           # 【新】ServerContext (精简版)
│   ├── builder.rs           # 【新】ServerContextBuilder
│   └── error.rs             # 不变
│
├── orders/
│   ├── mod.rs
│   ├── manager.rs           # 扩展: + start_background()
│   ├── warmup.rs            # 【新】预热逻辑
│   ├── storage.rs           # 不变
│   ├── reducer.rs           # 不变
│   └── ...
│
├── printing/
│   ├── mod.rs
│   ├── service.rs           # 扩展: + warmup_cache()
│   ├── background.rs        # 【新】后台任务集中
│   ├── cache.rs             # 不变
│   ├── storage.rs           # 不变
│   └── ...
│
├── services/                # 不变
├── api/                     # 微调: ServerState → ServerContext
├── auth/                    # 不变
├── db/                      # 不变
├── message/                 # 不变
├── pricing/                 # 不变
└── utils/                   # 不变
```

### 2.2 ServerContext 定义

```rust
// src/core/context.rs

use std::sync::Arc;
use surrealdb::{Surreal, engine::local::Db};
use tokio_util::sync::CancellationToken;

use crate::auth::JwtService;
use crate::orders::OrdersManager;
use crate::pricing::PriceRuleEngine;
use crate::printing::KitchenPrintService;
use crate::services::{
    ActivationService, CertService, HttpsService, MessageBusService,
};

use super::config::Config;
use super::state::ResourceVersions;

/// 认证相关服务打包
#[derive(Clone, Debug)]
pub struct AuthServices {
    pub jwt: Arc<JwtService>,
    pub activation: ActivationService,
    pub cert: CertService,
    pub https: HttpsService,
}

/// 服务器上下文 - 持有所有服务的引用
///
/// 与 ServerState 的区别:
/// - 只做组装，不含业务逻辑
/// - 后台任务由各服务自己管理
/// - 预热逻辑移到对应模块
#[derive(Clone, Debug)]
pub struct ServerContext {
    // 基础配置
    pub config: Config,
    pub db: Surreal<Db>,
    pub epoch: String,

    // 基础服务
    pub message_bus: MessageBusService,
    pub resource_versions: Arc<ResourceVersions>,

    // 领域服务
    pub orders: Arc<OrdersManager>,
    pub pricing: PriceRuleEngine,
    pub printing: Arc<KitchenPrintService>,

    // 认证相关 (打包)
    pub auth: AuthServices,

    // 关闭信号
    pub shutdown: CancellationToken,
}

impl ServerContext {
    /// 获取数据库实例
    pub fn db(&self) -> Surreal<Db> {
        self.db.clone()
    }

    /// 获取消息总线
    pub fn message_bus(&self) -> &Arc<crate::message::MessageBus> {
        self.message_bus.bus()
    }

    /// 广播同步消息 (从 ServerState 移过来，逻辑不变)
    pub async fn broadcast_sync<T: serde::Serialize>(
        &self,
        resource: &str,
        action: &str,
        id: &str,
        data: Option<&T>,
    ) {
        use shared::message::{BusMessage, SyncPayload};

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

    /// 打印激活横幅 (从 ServerState 移过来，逻辑不变)
    pub async fn print_activated_banner(&self) {
        let cred = self.auth.activation.get_credential().await.unwrap_or_default();
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
                "  HTTPS Server : https://localhost:{}",
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
```

### 2.3 ServerContextBuilder 定义

```rust
// src/core/builder.rs

use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::auth::JwtService;
use crate::db::DbService;
use crate::orders::OrdersManager;
use crate::pricing::PriceRuleEngine;
use crate::printing::{KitchenPrintService, PrintConfigCache, PrintStorage};
use crate::services::{
    ActivationService, CertService, HttpsService, MessageBusService,
};

use super::config::{Config, migrate_legacy_structure};
use super::context::{AuthServices, ServerContext};
use super::state::ResourceVersions;
use super::error::{Result, ServerError};

pub struct ServerContextBuilder {
    config: Config,
}

impl ServerContextBuilder {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn build(self) -> Result<ServerContext> {
        let config = self.config;

        // 0. 确保工作目录结构
        config
            .ensure_work_dir_structure()
            .map_err(|e| ServerError::Internal(e.into()))?;

        // 0.1 迁移旧目录结构
        let work_dir = PathBuf::from(&config.work_dir);
        migrate_legacy_structure(&work_dir)
            .map_err(|e| ServerError::Internal(e.into()))?;

        // 1. 初始化数据库
        let db_dir = config.database_dir();
        let db_path = db_dir.join("crab.db");
        let db_service = DbService::new(&db_path.to_string_lossy())
            .await
            .map_err(|e| ServerError::Internal(e.into()))?;
        let db = db_service.db;

        // 2. 初始化基础服务
        let activation = ActivationService::new(
            config.auth_server_url.clone(),
            config.auth_storage_dir(),
        );
        let cert_service = CertService::new(work_dir.clone());
        let message_bus = MessageBusService::new(&config);
        let https = HttpsService::new(config.clone());
        let jwt_service = Arc::new(JwtService::default());
        let resource_versions = Arc::new(ResourceVersions::new());

        // 3. 初始化领域服务
        let orders_db_path = db_dir.join("orders.redb");
        let orders = Arc::new(
            OrdersManager::new(&orders_db_path)
                .map_err(|e| ServerError::Internal(e.into()))?,
        );

        let pricing = PriceRuleEngine::new(db.clone());

        let print_db_path = db_dir.join("print.redb");
        let print_storage = PrintStorage::open(&print_db_path)
            .map_err(|e| ServerError::Internal(e.into()))?;
        let print_config_cache = PrintConfigCache::new();
        let printing = Arc::new(KitchenPrintService::new(
            print_storage,
            print_config_cache,
        ));

        // 4. 生成 epoch
        let epoch = uuid::Uuid::new_v4().to_string();

        // 5. 组装上下文
        let ctx = ServerContext {
            config: config.clone(),
            db,
            epoch,
            message_bus,
            resource_versions,
            orders,
            pricing,
            printing,
            auth: AuthServices {
                jwt: jwt_service,
                activation,
                cert: cert_service,
                https: https.clone(),
            },
            shutdown: CancellationToken::new(),
        };

        // 6. HttpsService 延迟初始化
        https.initialize(ctx.clone());

        Ok(ctx)
    }
}
```

### 2.4 启动流程

```rust
// src/core/context.rs (续)

impl ServerContext {
    /// 启动所有后台服务
    pub async fn start(&self) {
        // 1. 预热缓存
        self.printing.warmup_cache(&self.db).await;
        crate::orders::warmup_active_order_rules(&self.orders, &self.db).await;

        // 2. 启动各服务的后台任务
        self.message_bus.start_background_tasks(self.clone());

        self.orders.start_background(
            self.shutdown.clone(),
            self.message_bus.bus().clone(),
        );

        self.printing.start_background(
            self.shutdown.clone(),
            self.db.clone(),
            self.orders.subscribe(),
            self.message_bus.bus().subscribe(),
        );
    }

    // === 委托方法 (兼容现有 API 调用) ===

    pub async fn is_activated(&self) -> bool {
        self.auth.activation.is_activated().await
    }

    pub async fn wait_for_activation(&self) {
        self.auth.activation
            .wait_for_activation(&self.auth.cert)
            .await
    }

    pub fn load_tls_config(&self) -> std::result::Result<Option<Arc<rustls::ServerConfig>>, crate::utils::AppError> {
        self.auth.cert.load_tls_config()
    }

    pub async fn enter_unbound_state(&self) {
        self.auth.activation
            .enter_unbound_state_public(&self.auth.cert)
            .await;
    }
}
```

---

## 3. 模块改造详情

### 3.1 Printing 模块

#### 3.1.1 新建 `background.rs`

```rust
// src/printing/background.rs

use std::collections::HashMap;
use std::sync::Arc;
use surrealdb::{Surreal, engine::local::Db};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use shared::message::BusMessage;
use shared::order::{OrderEvent, OrderEventType};

use crate::db::repository::PrintDestinationRepository;
use crate::printing::{
    KitchenPrintService, PrintExecutor,
    CategoryPrintConfig, ProductPrintConfig,
};

impl KitchenPrintService {
    /// 启动所有后台任务
    pub fn start_background(
        self: &Arc<Self>,
        shutdown: CancellationToken,
        db: Surreal<Db>,
        orders_rx: broadcast::Receiver<OrderEvent>,
        sync_rx: broadcast::Receiver<BusMessage>,
    ) {
        self.spawn_event_listener(shutdown.clone(), orders_rx, db);
        self.spawn_sync_listener(shutdown.clone(), sync_rx);
        self.spawn_cleanup_task(shutdown);
    }

    /// 厨房打印事件监听器
    fn spawn_event_listener(
        self: &Arc<Self>,
        shutdown: CancellationToken,
        mut orders_rx: broadcast::Receiver<OrderEvent>,
        db: Surreal<Db>,
    ) {
        let service = self.clone();
        let executor = PrintExecutor::new();

        tokio::spawn(async move {
            tracing::info!("🖨️ Kitchen print event listener started");

            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => {
                        tracing::info!("Kitchen print listener shutting down");
                        break;
                    }
                    result = orders_rx.recv() => {
                        match result {
                            Ok(event) => {
                                if event.event_type != OrderEventType::ItemsAdded {
                                    continue;
                                }

                                // 处理打印逻辑 (从 state.rs 移过来，逻辑不变)
                                match service.process_items_added(&event, None).await {
                                    Ok(Some(kitchen_order_id)) => {
                                        tracing::info!(
                                            order_id = %event.order_id,
                                            kitchen_order_id = %kitchen_order_id,
                                            "🖨️ Created kitchen order"
                                        );

                                        // 执行打印
                                        if let Ok(Some(order)) = service.get_kitchen_order(&kitchen_order_id) {
                                            let repo = PrintDestinationRepository::new(db.clone());
                                            if let Ok(destinations) = repo.find_all().await {
                                                let dest_map: HashMap<String, _> = destinations
                                                    .into_iter()
                                                    .filter_map(|d| {
                                                        d.id.as_ref().map(|id| (id.id.to_string(), d.clone()))
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
                                        }
                                    }
                                    Ok(None) => {}
                                    Err(e) => {
                                        tracing::error!(
                                            order_id = %event.order_id,
                                            "Failed to process ItemsAdded for printing: {:?}",
                                            e
                                        );
                                    }
                                }
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                tracing::warn!("Kitchen print listener lagged, skipped {} events", n);
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                tracing::info!("Order event channel closed");
                                break;
                            }
                        }
                    }
                }
            }
        });
    }

    /// 同步事件监听器 (更新打印配置缓存)
    fn spawn_sync_listener(
        self: &Arc<Self>,
        shutdown: CancellationToken,
        mut sync_rx: broadcast::Receiver<BusMessage>,
    ) {
        let service = self.clone();

        tokio::spawn(async move {
            tracing::info!("🔄 Print config sync listener started");

            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => {
                        tracing::info!("Print config sync listener shutting down");
                        break;
                    }
                    result = sync_rx.recv() => {
                        match result {
                            Ok(msg) => {
                                // 解析并更新缓存 (从 state.rs 移过来，逻辑不变)
                                service.handle_sync_message(msg).await;
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                tracing::warn!("Sync listener lagged, skipped {} events", n);
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                tracing::info!("Sync channel closed");
                                break;
                            }
                        }
                    }
                }
            }
        });
    }

    /// 定时清理任务
    fn spawn_cleanup_task(self: &Arc<Self>, shutdown: CancellationToken) {
        const CLEANUP_INTERVAL_SECS: u64 = 3600; // 1 hour
        const MAX_AGE_SECS: i64 = 3 * 24 * 3600;  // 3 days

        let service = self.clone();

        tokio::spawn(async move {
            tracing::info!("🧹 Print record cleanup task started (interval: 1h, max_age: 3d)");

            // 启动时立即清理一次
            if let Ok(count) = service.cleanup_old_records(MAX_AGE_SECS) {
                if count > 0 {
                    tracing::info!("🧹 Cleaned up {} old print records on startup", count);
                }
            }

            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(CLEANUP_INTERVAL_SECS)
            );
            interval.tick().await; // 跳过第一次

            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => {
                        tracing::info!("Cleanup task shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        if let Ok(count) = service.cleanup_old_records(MAX_AGE_SECS) {
                            if count > 0 {
                                tracing::info!("🧹 Cleaned up {} old print records", count);
                            }
                        }
                    }
                }
            }
        });
    }

    /// 处理同步消息 (内部方法)
    async fn handle_sync_message(&self, msg: BusMessage) {
        use shared::message::{EventType, SyncPayload};
        use shared::models::{
            category::Category as SharedCategory,
            product::Product as SharedProduct,
        };

        if msg.event_type != EventType::Sync {
            return;
        }

        let payload: SyncPayload = match serde_json::from_slice(&msg.payload) {
            Ok(p) => p,
            Err(_) => return,
        };

        // 处理 product 变更
        if payload.resource == "product" {
            if let Some(data) = &payload.data {
                if let Ok(product) = serde_json::from_value::<SharedProduct>(data.clone()) {
                    let product_id = product.id.clone().unwrap_or_default();
                    let root_spec_external_id = product
                        .specs
                        .iter()
                        .find(|s| s.is_root)
                        .and_then(|s| s.external_id);

                    let config = ProductPrintConfig {
                        product_id,
                        product_name: product.name.clone(),
                        kitchen_name: product.kitchen_print_name
                            .clone()
                            .unwrap_or_else(|| product.name.clone()),
                        kitchen_print_destinations: product.kitchen_print_destinations,
                        label_print_destinations: product.label_print_destinations,
                        is_kitchen_print_enabled: product.is_kitchen_print_enabled,
                        is_label_print_enabled: product.is_label_print_enabled,
                        root_spec_external_id,
                        category_id: product.category,
                    };
                    self.config_cache().update_product(config).await;
                }
            }
        }

        // 处理 category 变更
        if payload.resource == "category" {
            if let Some(data) = &payload.data {
                if let Ok(category) = serde_json::from_value::<SharedCategory>(data.clone()) {
                    let category_id = category.id.clone().unwrap_or_default();
                    let config = CategoryPrintConfig {
                        category_id,
                        category_name: category.name.clone(),
                        kitchen_print_destinations: category.kitchen_print_destinations,
                        label_print_destinations: category.label_print_destinations,
                        is_kitchen_print_enabled: category.is_kitchen_print_enabled,
                        is_label_print_enabled: category.is_label_print_enabled,
                    };
                    self.config_cache().update_category(config).await;
                }
            }
        }
    }
}
```

#### 3.1.2 扩展 `service.rs`

```rust
// src/printing/service.rs (添加预热方法)

impl KitchenPrintService {
    /// 预热打印配置缓存
    pub async fn warmup_cache(&self, db: &Surreal<Db>) {
        use crate::db::repository::{CategoryRepository, ProductRepository};

        let product_repo = ProductRepository::new(db.clone());
        let category_repo = CategoryRepository::new(db.clone());

        // 加载分类配置
        match category_repo.find_all_with_destinations().await {
            Ok(categories) => {
                for cat in &categories {
                    let id = cat.id.as_ref()
                        .map(|t| t.id.to_string())
                        .unwrap_or_default();

                    let kitchen_destinations: Vec<String> = cat
                        .kitchen_print_destinations
                        .iter()
                        .map(|t| t.id.to_string())
                        .collect();
                    let label_destinations: Vec<String> = cat
                        .label_print_destinations
                        .iter()
                        .map(|t| t.id.to_string())
                        .collect();

                    let config = CategoryPrintConfig {
                        category_id: id,
                        category_name: cat.name.clone(),
                        kitchen_print_destinations: kitchen_destinations,
                        label_print_destinations: label_destinations,
                        is_kitchen_print_enabled: cat.is_kitchen_print_enabled,
                        is_label_print_enabled: cat.is_label_print_enabled,
                    };
                    self.config_cache().update_category(config).await;
                }
                tracing::info!("🖨️ Loaded {} category print configs", categories.len());
            }
            Err(e) => {
                tracing::error!("Failed to load categories for print config: {:?}", e);
            }
        }

        // 加载商品配置
        match product_repo.find_all_with_destinations().await {
            Ok(products) => {
                for prod in &products {
                    let id = prod.id.as_ref()
                        .map(|t| t.id.to_string())
                        .unwrap_or_default();

                    let kitchen_destinations: Vec<String> = prod
                        .kitchen_print_destinations
                        .iter()
                        .map(|t| t.id.to_string())
                        .collect();
                    let label_destinations: Vec<String> = prod
                        .label_print_destinations
                        .iter()
                        .map(|t| t.id.to_string())
                        .collect();

                    let category_id = prod.category.id.to_string();
                    let root_spec_external_id = prod
                        .specs
                        .iter()
                        .find(|s| s.is_root)
                        .and_then(|s| s.external_id);

                    let config = ProductPrintConfig {
                        product_id: id,
                        product_name: prod.name.clone(),
                        kitchen_name: prod.kitchen_print_name
                            .clone()
                            .unwrap_or_else(|| prod.name.clone()),
                        kitchen_print_destinations: kitchen_destinations,
                        label_print_destinations: label_destinations,
                        is_kitchen_print_enabled: prod.is_kitchen_print_enabled,
                        is_label_print_enabled: prod.is_label_print_enabled,
                        root_spec_external_id,
                        category_id,
                    };
                    self.config_cache().update_product(config).await;
                }
                tracing::info!("🖨️ Loaded {} product print configs", products.len());
            }
            Err(e) => {
                tracing::error!("Failed to load products for print config: {:?}", e);
            }
        }
    }
}
```

### 3.2 Orders 模块

#### 3.2.1 新建 `warmup.rs`

```rust
// src/orders/warmup.rs

use surrealdb::{Surreal, engine::local::Db};
use crate::orders::OrdersManager;
use crate::orders::actions::open_table::load_matching_rules;

/// 预热活跃订单的价格规则缓存
pub async fn warmup_active_order_rules(orders: &OrdersManager, db: &Surreal<Db>) {
    let active_orders = match orders.get_active_orders() {
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
            db,
            order.zone_id.as_deref(),
            order.is_retail,
        ).await;

        if !rules.is_empty() {
            orders.cache_rules(&order.order_id, rules);
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
pub async fn load_rules_for_order(
    orders: &OrdersManager,
    db: &Surreal<Db>,
    order_id: &str,
) -> bool {
    let snapshot = match orders.get_snapshot(order_id) {
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
        db,
        snapshot.zone_id.as_deref(),
        snapshot.is_retail,
    ).await;

    if !rules.is_empty() {
        tracing::debug!(
            order_id = %order_id,
            rule_count = rules.len(),
            "Loaded rules for order"
        );
        orders.cache_rules(order_id, rules);
    }

    true
}
```

#### 3.2.2 扩展 `manager.rs`

```rust
// src/orders/manager.rs (添加后台任务方法)

use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use shared::message::{BusMessage, SyncPayload};

impl OrdersManager {
    /// 启动后台任务
    pub fn start_background(
        self: &Arc<Self>,
        shutdown: CancellationToken,
        message_bus: Arc<crate::message::MessageBus>,
    ) {
        self.spawn_event_forwarder(shutdown, message_bus);
    }

    /// 订单事件转发器 (OrderEvent -> MessageBus)
    fn spawn_event_forwarder(
        self: &Arc<Self>,
        shutdown: CancellationToken,
        message_bus: Arc<crate::message::MessageBus>,
    ) {
        let mut event_rx = self.subscribe();
        let manager = self.clone();

        tokio::spawn(async move {
            tracing::info!("📦 Order sync forwarder started");

            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => {
                        tracing::info!("Order forwarder shutting down");
                        break;
                    }
                    result = event_rx.recv() => {
                        match result {
                            Ok(event) => {
                                let order_id = event.order_id.clone();
                                let sequence = event.sequence;
                                let action = event.event_type.to_string();

                                // 获取快照，打包推送
                                match manager.get_snapshot(&order_id) {
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
                                tracing::info!("Order event channel closed");
                                break;
                            }
                        }
                    }
                }
            }
        });
    }
}
```

---

## 4. API 层适配

### 4.1 全局替换清单

| 旧代码 | 新代码 |
|-------|-------|
| `State<ServerState>` | `State<ServerContext>` |
| `state.get_db()` | `ctx.db.clone()` |
| `state.message_bus()` | `ctx.message_bus()` |
| `state.orders_manager()` | `&ctx.orders` |
| `state.kitchen_print_service()` | `&ctx.printing` |
| `state.get_jwt_service()` | `ctx.auth.jwt.clone()` |
| `state.cert_service()` | `&ctx.auth.cert` |
| `state.activation_service()` | `&ctx.auth.activation` |
| `state.https_service()` | `&ctx.auth.https` |
| `state.price_rule_engine` | `ctx.pricing` |
| `state.resource_versions` | `ctx.resource_versions` |
| `state.epoch` | `ctx.epoch` |

### 4.2 Handler 示例

```rust
// 改造前
async fn get_order(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<OrderSnapshot>> {
    let snapshot = state.orders_manager().get_snapshot(&id)?;
    // ...
}

// 改造后
async fn get_order(
    State(ctx): State<ServerContext>,
    Path(id): Path<String>,
) -> AppResult<Json<OrderSnapshot>> {
    let snapshot = ctx.orders.get_snapshot(&id)?;
    // ...
}
```

---

## 5. 迁移步骤

### 阶段 1：新建文件 (不改现有代码)

```bash
# 创建新文件
touch src/core/context.rs
touch src/core/builder.rs
touch src/printing/background.rs
touch src/orders/warmup.rs
```

### 阶段 2：实现新模块

| 顺序 | 文件 | 内容 |
|-----|------|------|
| 2.1 | `core/context.rs` | ServerContext + AuthServices 定义 |
| 2.2 | `core/builder.rs` | ServerContextBuilder 实现 |
| 2.3 | `printing/background.rs` | 3 个 spawn 函数 |
| 2.4 | `printing/service.rs` | 添加 `warmup_cache()` |
| 2.5 | `orders/warmup.rs` | 预热函数 |
| 2.6 | `orders/manager.rs` | 添加 `start_background()` |

### 阶段 3：更新导出

```rust
// src/core/mod.rs
pub mod context;
pub mod builder;
pub use context::{ServerContext, AuthServices};
pub use builder::ServerContextBuilder;

// src/lib.rs
pub use core::{ServerContext, ServerContextBuilder};

// src/orders/mod.rs
pub mod warmup;
pub use warmup::{warmup_active_order_rules, load_rules_for_order};

// src/printing/mod.rs
mod background;  // 私有，只通过 service 方法暴露
```

### 阶段 4：切换使用

| 顺序 | 操作 |
|-----|------|
| 4.1 | `Server::run()` 使用 `ServerContextBuilder` |
| 4.2 | `services/https.rs` 改用 `ServerContext` |
| 4.3 | `services/message_bus.rs` 改用 `ServerContext` |
| 4.4 | 全局替换 API handlers |

### 阶段 5：清理

| 顺序 | 操作 |
|-----|------|
| 5.1 | 删除 `state.rs` 中已迁移的代码 |
| 5.2 | 保留 `ResourceVersions` (移到 `context.rs` 或独立文件) |
| 5.3 | 删除或重命名 `state.rs` |
| 5.4 | `cargo clippy` 清理 |

---

## 6. 字段变更记录 (前端需同步)

**本次重构不涉及 API 接口变更，前端无需修改。**

内部字段变更：

| 变更类型 | 旧 | 新 | 影响范围 |
|---------|----|----|---------|
| 重命名 | `ServerState` | `ServerContext` | 仅后端内部 |
| 打包 | 分散的 auth 字段 | `AuthServices` | 仅后端内部 |

---

## 7. 验证清单

- [ ] `cargo check --workspace` 通过
- [ ] `cargo test --workspace --lib` 通过
- [ ] `cargo clippy --workspace` 无警告
- [ ] 服务器正常启动
- [ ] 打印功能正常 (ItemsAdded -> 厨房打印)
- [ ] 订单同步正常 (OrderEvent -> MessageBus)
- [ ] 预热日志正常输出
- [ ] 优雅关闭正常 (Ctrl+C)
