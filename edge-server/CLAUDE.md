# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Edge Server

分布式餐厅管理系统边缘节点 — 嵌入式数据库 + RESTful API + 实时消息总线 + 订单事件溯源。

## 命令

```bash
cargo check -p edge-server
cargo test -p edge-server --lib
cargo run -p edge-server --example interactive_demo
```

## 模块结构

```
src/
├── core/           # 服务器核心
│   ├── config.rs       # Config (端口、JWT、超时等)
│   ├── state.rs        # ServerState + ResourceVersions
│   ├── server.rs       # Server 启动 + Graceful Shutdown
│   ├── event_router.rs # EventRouter (事件分发到 Archive/Print/Sync)
│   └── tasks.rs        # BackgroundTasks (周期任务管理)
├── api/            # HTTP 路由和处理器 (Axum)
│   ├── auth/           # 登录认证
│   ├── products/       # 商品 CRUD
│   ├── categories/     # 分类 CRUD
│   ├── attributes/     # 属性 CRUD
│   ├── has_attribute/  # 商品-属性绑定 (attribute_binding 边)
│   ├── tags/           # 标签 CRUD
│   ├── zones/          # 区域 CRUD
│   ├── tables/         # 餐桌 CRUD
│   ├── employees/      # 员工 CRUD
│   ├── roles/          # 角色 CRUD
│   ├── price_rules/    # 价格规则 CRUD
│   ├── print_config/   # 打印配置
│   ├── print_destinations/ # 打印目标
│   ├── orders/         # 订单查询 (归档历史)
│   ├── kitchen_orders/ # 厨房订单
│   ├── label_template/ # 标签模板 CRUD
│   ├── shifts/         # 班次 CRUD
│   ├── daily_reports/  # 日报
│   ├── statistics/     # 统计分析 (overview, trends, sales)
│   ├── sync/           # 同步 API (重连同步)
│   ├── system_state/   # 系统状态
│   ├── store_info/     # 门店信息
│   ├── upload/         # 文件上传
│   ├── health/         # 健康检查
│   └── archive_verify/ # 归档验证 API
├── auth/           # 认证与权限
│   ├── jwt.rs          # JwtService (Argon2 + JWT)
│   ├── middleware.rs   # require_auth() + require_permission()
│   └── permissions.rs  # RBAC 权限定义 (Admin/Manager/User)
├── db/             # SurrealDB 数据访问层
│   ├── models/         # 数据模型 (与 shared 对齐)
│   └── repository/     # CRUD 操作
├── message/        # 消息总线 (TCP/TLS/Memory)
├── orders/         # 订单事件溯源 [核心模块]
│   ├── traits.rs       # CommandHandler + EventApplier trait
│   ├── manager.rs      # OrdersManager (命令执行 + 事件分发)
│   ├── reducer.rs      # 价格规则集成
│   ├── money.rs        # 金额计算 (rust_decimal)
│   ├── actions/        # CommandHandler 实现 (12+ 命令)
│   ├── appliers/       # EventApplier 实现 (22+ 事件)
│   ├── storage.rs      # redb 持久化 (events, snapshots, queues)
│   ├── archive.rs      # OrderArchiveService (归档到 SurrealDB 图模型)
│   ├── archive_worker.rs # ArchiveWorker (队列处理, 并发50, 重试3次)
│   ├── verify_scheduler.rs # 哈希链验证调度器 (启动补扫 + 每日定时)
│   └── sync.rs         # 重连同步 API
├── pricing/        # 价格规则引擎
│   ├── engine.rs       # PriceRuleEngine (加载/应用规则)
│   ├── matcher.rs      # 范围匹配 (Product/Category/Tag/Zone/Time)
│   ├── calculator.rs   # 通用计算辅助
│   ├── item_calculator.rs  # 商品级计算 (折扣/附加费叠加)
│   └── order_calculator.rs # 订单级计算
├── printing/       # 厨房/标签打印
│   ├── types.rs        # KitchenOrder, LabelPrintRecord, PrintItemContext
│   ├── service.rs      # KitchenPrintService (事件处理)
│   ├── worker.rs       # KitchenPrintWorker (监听 EventRouter)
│   ├── executor.rs     # PrintExecutor (发送到打印机)
│   ├── renderer.rs     # KitchenTicketRenderer (ESC/POS 渲染)
│   └── storage.rs      # redb 打印记录存储
├── services/       # 业务服务
│   ├── catalog_service.rs  # CatalogService (商品/分类内存缓存)
│   ├── message_bus.rs      # MessageBusService
│   ├── cert.rs             # CertService (mTLS 证书)
│   ├── activation.rs       # ActivationService (激活状态)
│   ├── tenant_binding.rs   # TenantBinding (订阅信息)
│   ├── provisioning.rs     # ProvisioningService (边缘配置)
│   └── https.rs            # HttpsService
├── shifts.rs       # ShiftAutoCloseScheduler (班次自动关闭)
└── utils/          # AppError, Logger, 工具函数
```

## 核心概念

### ServerState

```rust
pub struct ServerState {
    pub config: Config,
    pub db: Surreal<Db>,                    // SurrealDB
    pub activation: ActivationService,
    pub cert_service: CertService,
    pub message_bus: MessageBusService,
    pub https: HttpsService,
    pub jwt_service: Arc<JwtService>,
    pub resource_versions: Arc<ResourceVersions>,  // DashMap 版本管理
}
// Clone 成本极低 - 所有字段都是 Arc 包装
```

### 订单事件溯源

**CommandHandler** (async trait):
```rust
async fn execute(&self, ctx: &mut CommandContext, metadata: &CommandMetadata)
    -> Result<Vec<OrderEvent>, OrderError>;
```
- 可做 I/O（数据库查询、外部调用）
- 只在处理新命令时调用

**EventApplier** (纯函数 trait):
```rust
fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent);
```
- **纯函数**: 无 I/O、无副作用
- 用于命令执行和事件回放

**命令流程**:
```
execute_command(cmd)
  ├─ 幂等性检查 (command_id)
  ├─ 开启 redb 写事务
  ├─ CommandAction::from(&cmd).execute()
  ├─ EventApplier.apply() (更新 snapshot)
  ├─ 持久化 events + snapshots
  ├─ 提交事务
  ├─ EventRouter 广播
  └─ 返回 CommandResponse
```

**Commands (12+)**:
OpenTable, AddItems, ModifyItem, RemoveItem, RestoreItem, AddPayment, CancelPayment, CompleteOrder, VoidOrder, MergeOrders, MoveOrder, SplitByItems, SplitByAmount, StartAaSplit, PayAaSplit, UpdateOrderInfo, ToggleRuleSkip

**EventRouter 分发**:
- **Archive** (阻塞): 终结事件 (Completed, Voided, Moved, Merged)
- **Print** (尽力,丢弃): ItemsAdded 事件
- **Sync** (尽力,丢弃): 所有事件

### 归档系统

- **OrderArchiveService**: 归档到 SurrealDB 图模型 (RELATE 边)
- **ArchiveWorker**: 队列处理，并发 50，重试 3 次，指数退避 5s→60s
- **VerifyScheduler**: SHA256 哈希链验证，启动补扫 + business_day_cutoff 定时
- **Dead Letter Queue**: 永久失败的归档任务隔离

### 价格规则引擎

- **Scope 匹配**: Global / Product / Category / Tag / Zone
- **时间匹配**: valid_from/until + active_days + active_start/end_time
- **计算模式**: Percentage / FixedAmount
- **规则类型**: Discount / Surcharge
- **叠加控制**: is_stackable + is_exclusive + priority

### 打印系统

- **KitchenPrintService**: 处理 ItemsAdded 事件，创建厨房单/标签记录
- **KitchenPrintWorker**: 监听 EventRouter，调用 PrintExecutor
- **KitchenTicketRenderer**: ESC/POS 渲染 (58mm=32字符 / 80mm=48字符)
- **PrintStorage**: redb 存储打印记录

### RBAC 权限

**权限组**:
- **Admin**: `["all"]` (超级用户)
- **Manager**: products/categories/attributes/orders/zones/tables/pricing/statistics/receipts 的完整权限
- **User**: 基础读取 + orders:read/write + receipts:print

**权限格式**: `resource:action` (如 `orders:void`, `products:write`, `settings:manage`)

### 添加 API

1. `api/<resource>/` 创建 `mod.rs` + `handler.rs`
2. `api/mod.rs` 添加路由
3. 使用 `ApiResponse::ok()` 返回响应
4. 添加 `require_permission("resource:action")` 中间件

### 添加订单命令

1. `shared/src/order/command.rs` 添加 `OrderCommandPayload` variant
2. `shared/src/order/event.rs` 添加 `OrderEventType` + `EventPayload` variant
3. `orders/actions/` 创建 Action 实现 `CommandHandler`
4. `orders/appliers/` 创建 Applier 实现 `EventApplier`
5. `orders/actions/mod.rs` + `orders/appliers/mod.rs` 注册分发

### CatalogService

内存缓存商品/分类/属性元数据:
- `get_product_meta()` → ProductMeta (category_id, tags, tax_rate)
- `is_kitchen_print_enabled()` / `is_label_print_enabled()`
- 在商品/分类更新时自动失效

## 响应语言

使用中文回答。
