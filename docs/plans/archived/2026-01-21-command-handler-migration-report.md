# 订单命令处理架构迁移报告

**迁移目标**: 从 match-based 架构迁移到 Strategy Pattern (trait-based) 架构
**迁移范围**: edge-server/src/orders/
**迁移类型**: 完全重构，不保留兼容性
**预计影响**: 核心订单处理逻辑，无对外API变化

---

## 1. 架构对比

### 1.1 现有架构 (Match-Based)

```rust
// edge-server/src/orders/manager.rs
impl OrdersManager {
    fn process_command(&self, cmd: OrderCommand)
        -> ManagerResult<(CommandResponse, Vec<OrderEvent>)>
    {
        let txn = self.storage.begin_write()?;

        // ❌ 巨大的 match 语句 (14+ 分支)
        let result = match &cmd.payload {
            OrderCommandPayload::OpenTable { .. } => self.handle_open_table(&txn, &cmd),
            OrderCommandPayload::CompleteOrder { order_id, receipt_number } =>
                self.handle_complete_order(&txn, &cmd, order_id, receipt_number),
            OrderCommandPayload::AddItems { order_id, items } =>
                self.handle_add_items(&txn, &cmd, order_id, items),
            OrderCommandPayload::ModifyItem { order_id, instance_id, affected_quantity, changes, .. } =>
                self.handle_modify_item(&txn, &cmd, order_id, instance_id, affected_quantity, changes),
            // ... 11 more branches
        };

        // ... 持久化、提交
    }

    // ❌ 14+ 个 handle_xxx 方法挤在同一个文件
    fn handle_open_table(...) -> ManagerResult<...> { /* 63 lines */ }
    fn handle_complete_order(...) -> ManagerResult<...> { /* 58 lines */ }
    fn handle_add_items(...) -> ManagerResult<...> { /* 32 lines */ }
    fn handle_modify_item(...) -> ManagerResult<...> { /* 134 lines */ }
    // ...
}

// edge-server/src/orders/reducer.rs
impl OrderReducer {
    pub fn apply_event(snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        // ❌ 另一个巨大的 match 语句 (14+ 分支)
        match &event.payload {
            EventPayload::TableOpened { .. } => { /* ... */ }
            EventPayload::ItemsAdded { items } => { /* ... */ }
            EventPayload::ItemModified { .. } => { /* ... */ }
            // ... 11 more branches
        }
    }
}
```

**问题**：
- ❌ OrdersManager 超过 1200 行，难以维护
- ❌ 添加新命令需要修改多个match语句（违反开闭原则）
- ❌ 业务逻辑、状态更新、副作用混在一起
- ❌ 测试困难（无法独立测试单个命令处理逻辑）

---

### 1.2 新架构 (Strategy Pattern + enum_dispatch)

```
┌─────────────────────────────────────────────────────────┐
│          RequestCommandProcessor (message/processor.rs)  │
│               ↓ 接收 OrderCommand                         │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│  OrdersManager::execute_command() (orders/manager.rs)    │
│    1. 幂等性检查                                          │
│    2. OrderCommand → CommandAction (From trait, 1 match) │
│    3. action.execute(&mut snapshot) - enum_dispatch      │
│    4. 持久化 event                                        │
│    5. 持久化 snapshot                                     │
│    6. 提交事务                                            │
│    7. 广播 event                                          │
│    8. action.on_success() - 副作用                        │
└─────────────────────────────────────────────────────────┘
         │                            │
         │                            ▼
         │              ┌──────────────────────────┐
         │              │  SideEffects (可选)      │
         │              │  - 厨房打印机            │
         │              │  - 订单归档到SurrealDB   │
         │              └──────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────┐
│  CommandAction (enum_dispatch)                           │
│    - OpenTableAction                                     │
│    - AddItemsAction                                      │
│    - ModifyItemAction                                    │
│    - ...                                                 │
│  ✅ 每个 Action 独立文件，独立测试                        │
└─────────────────────────────────────────────────────────┘

重放流程 (Replay Events):
┌─────────────────────────────────────────────────────────┐
│  OrdersManager::rebuild_snapshot()                       │
│    1. 加载 events                                         │
│    2. OrderEvent → EventAction (From trait, 1 match)     │
│    3. applier.apply(&mut snapshot, event) - enum_dispatch│
│    ✅ 不调用 execute()，不执行副作用                      │
└─────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────┐
│  EventAction (enum_dispatch)                             │
│    - TableOpenedApplier                                  │
│    - ItemsAddedApplier                                   │
│    - ItemModifiedApplier                                 │
│    - ...                                                 │
│  ✅ 纯数据操作，无业务逻辑                                │
└─────────────────────────────────────────────────────────┘
```

**优势**：
- ✅ 消除巨大 match 语句（只在 From trait 中保留）
- ✅ 每个命令独立文件，单一职责
- ✅ 业务逻辑、状态更新、副作用分离清晰
- ✅ 添加新命令只需增加新文件，不修改现有代码（开闭原则）
- ✅ enum_dispatch 零成本抽象，性能无损
- ✅ 单元测试简单（每个 Handler/Applier 独立测试）

---

## 2. 文件结构变化

### 2.1 现有结构

```
edge-server/src/orders/
├── mod.rs          (exports)
├── manager.rs      (1200+ lines, 14+ handle_xxx methods)
├── reducer.rs      (300+ lines, giant match in apply_event)
└── storage.rs      (redb persistence)
```

### 2.2 新结构

```
edge-server/src/orders/
├── mod.rs                  (exports)
├── manager.rs              (~300 lines, 核心流程编排)
├── storage.rs              (redb persistence, 不变)
├── traits.rs               (CommandHandler, EventApplier, CommandMetadata)
├── actions/                (Command → Event 生成)
│   ├── mod.rs              (enum CommandAction + From<OrderCommand>)
│   ├── open_table.rs       (OpenTableAction)
│   ├── add_items.rs        (AddItemsAction)
│   ├── modify_item.rs      (ModifyItemAction)
│   ├── remove_item.rs      (RemoveItemAction)
│   ├── complete_order.rs   (CompleteOrderAction)
│   ├── void_order.rs       (VoidOrderAction)
│   ├── restore_order.rs    (RestoreOrderAction)
│   ├── restore_item.rs     (RestoreItemAction)
│   ├── add_payment.rs      (AddPaymentAction)
│   ├── cancel_payment.rs   (CancelPaymentAction)
│   ├── split_order.rs      (SplitOrderAction)
│   ├── move_order.rs       (MoveOrderAction)
│   ├── merge_orders.rs     (MergeOrdersAction)
│   └── update_order_info.rs (UpdateOrderInfoAction)
└── appliers/               (Event → Snapshot 投影)
    ├── mod.rs              (enum EventAction + From<&OrderEvent>)
    ├── table_opened.rs     (TableOpenedApplier)
    ├── items_added.rs      (ItemsAddedApplier)
    ├── item_modified.rs    (ItemModifiedApplier)
    ├── item_removed.rs     (ItemRemovedApplier)
    ├── order_completed.rs  (OrderCompletedApplier)
    ├── order_voided.rs     (OrderVoidedApplier)
    ├── order_restored.rs   (OrderRestoredApplier)
    ├── item_restored.rs    (ItemRestoredApplier)
    ├── payment_added.rs    (PaymentAddedApplier)
    ├── payment_cancelled.rs (PaymentCancelledApplier)
    ├── order_split.rs      (OrderSplitApplier)
    ├── order_moved.rs      (OrderMovedApplier)
    ├── orders_merged.rs    (OrdersMergedApplier)
    └── order_info_updated.rs (OrderInfoUpdatedApplier)
```

**文件统计**：
- 现有：4 个文件
- 新架构：32 个文件 (更模块化，但每个文件更小更聚焦)

---

## 3. 金额处理（rust_decimal）

### 3.1 设计原则

**禁止使用 `f64` 处理金额**，必须使用 `rust_decimal::Decimal`：

```toml
# Cargo.toml
[dependencies]
rust_decimal = { version = "1.33", features = ["serde", "serde-with-str"] }
rust_decimal_macros = "1.33"
```

```rust
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

// ✅ 正确
let price: Decimal = dec!(10.99);
let total = price * Decimal::from(quantity);

// ❌ 错误
let price: f64 = 10.99;
let total = price * quantity as f64;
```

### 3.2 类型定义

```rust
// shared/src/order/types.rs

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartItemSnapshot {
    pub instance_id: String,
    pub item_id: String,
    pub item_name: String,
    pub quantity: i32,
    pub unit_price: Decimal,    // ← Decimal
    pub total_price: Decimal,   // ← Decimal
    pub modifiers: Vec<Modifier>,
    pub notes: Option<String>,
    pub is_voided: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentSnapshot {
    pub payment_id: String,
    pub method: String,
    pub amount: Decimal,        // ← Decimal
    pub status: PaymentStatus,
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSnapshot {
    // ...
    pub total_amount: Decimal,  // ← Decimal
    pub paid_amount: Decimal,   // ← Decimal
    // ...
}
```

### 3.3 Hash 计算中的金额处理

```rust
// Decimal 在 Hash 计算中使用 serialize() 保证确定性
fn calculate_hash(&self) -> String {
    let mut hasher = Sha256::new();

    // ✅ 正确：使用 Decimal 的确定性字节表示
    hasher.update(&self.unit_price.serialize());
    hasher.update(&self.total_price.serialize());

    // 或者使用字符串表示（更易读，略慢）
    // hasher.update(self.unit_price.to_string().as_bytes());

    format!("{:x}", hasher.finalize())
}
```

**Decimal.serialize() 优势**：
- ✅ 确定性：相同值总是生成相同字节
- ✅ 精确：无浮点数精度丢失
- ✅ 高效：16 字节固定长度

---

## 4. 核心 Trait 定义

### 4.1 CommandContext - 增强型执行上下文

```rust
// edge-server/src/orders/traits.rs

use crate::orders::storage::{WriteTransaction, OrderStorage};
use crate::core::ServerState;
use shared::order::{OrderSnapshot, OrderEvent};
use async_trait::async_trait;
use thiserror::Error;
use std::sync::Arc;
use std::collections::HashMap;

/// 命令执行上下文
///
/// **核心职责**：
/// 1. 管理写事务生命周期
/// 2. 提供 Snapshot 缓存（避免同一事务内重复读取）
/// 3. 提供服务访问（price_rule_engine, db 等）
/// 4. 支持跨订单操作（拆单、合并）
pub struct CommandContext<'a> {
    /// 写事务（私有）
    txn: &'a WriteTransaction,
    /// 存储层（用于加载/保存 snapshot）
    storage: &'a OrderStorage,
    /// Epoch（用于创建新订单）
    epoch: String,
    /// 服务器状态（包含所有服务：price_rule_engine, db, etc.）
    pub state: &'a Arc<ServerState>,
    /// Snapshot 缓存（防止同一事务内重复读取）
    snapshot_cache: HashMap<String, OrderSnapshot>,
}

impl<'a> CommandContext<'a> {
    pub fn new(
        txn: &'a WriteTransaction,
        storage: &'a OrderStorage,
        epoch: String,
        state: &'a Arc<ServerState>,
    ) -> Self {
        Self {
            txn,
            storage,
            epoch,
            state,
            snapshot_cache: HashMap::new(),
        }
    }

    /// 加载订单 Snapshot（支持缓存）
    ///
    /// **用途**：跨订单操作（拆单、合并）时加载其他订单
    pub fn load_snapshot(&mut self, order_id: &str) -> Result<OrderSnapshot, OrderError> {
        // 先查缓存
        if let Some(snapshot) = self.snapshot_cache.get(order_id) {
            return Ok(snapshot.clone());
        }

        // 从存储加载
        let snapshot = self.storage
            .get_snapshot(self.txn, order_id)?
            .ok_or_else(|| OrderError::OrderNotFound(order_id.to_string()))?;

        // 加入缓存
        self.snapshot_cache.insert(order_id.to_string(), snapshot.clone());

        Ok(snapshot)
    }

    /// 创建新订单 Snapshot
    ///
    /// **用途**：OpenTable 等创建新订单的场景
    pub fn create_snapshot(&mut self, order_id: String) -> OrderSnapshot {
        let snapshot = OrderSnapshot::new(order_id.clone(), self.epoch.clone());
        self.snapshot_cache.insert(order_id, snapshot.clone());
        snapshot
    }

    /// 保存 Snapshot 到缓存
    ///
    /// **注意**：实际持久化在 OrdersManager 中统一进行
    pub fn save_snapshot(&mut self, snapshot: OrderSnapshot) {
        self.snapshot_cache.insert(snapshot.order_id.clone(), snapshot);
    }

    /// 获取所有修改过的 Snapshot（用于批量持久化）
    pub fn modified_snapshots(&self) -> Vec<&OrderSnapshot> {
        self.snapshot_cache.values().collect()
    }
}

/// 命令元数据
#[derive(Debug, Clone)]
pub struct CommandMetadata {
    pub command_id: String,
    pub operator_id: String,
    pub operator_name: String,
    pub timestamp: i64,
}

/// 订单错误
#[derive(Debug, Error)]
pub enum OrderError {
    #[error("Order not found: {0}")]
    OrderNotFound(String),

    #[error("Order already completed: {0}")]
    OrderAlreadyCompleted(String),

    #[error("Order already voided: {0}")]
    OrderAlreadyVoided(String),

    #[error("Item not found: {0}")]
    ItemNotFound(String),

    #[error("Payment not found: {0}")]
    PaymentNotFound(String),

    #[error("Insufficient quantity")]
    InsufficientQuantity,

    #[error("Invalid amount")]
    InvalidAmount,

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Storage error: {0}")]
    Storage(String),
}

/// 命令处理器 Trait
///
/// **职责**：
/// 1. 执行业务逻辑（验证、计算、生成ID、应用价格规则）
/// 2. 通过 CommandContext 操作 Snapshot（支持跨订单）
/// 3. 返回完整的 Event 列表（单订单或跨订单操作）
/// 4. (可选) 执行副作用（厨房打印、归档等）
///
/// **设计原则**：
/// - Handler 是"上帝视角"，拥有全部业务逻辑
/// - Event 必须包含完整数据（instance_id、最终价格等）
/// - Applier 只做数据搬运，无业务逻辑
#[async_trait]
pub trait CommandHandler: Send + Sync {
    /// 执行命令，返回事件列表
    ///
    /// **执行时机**: 仅在新命令到达时调用（重放时不调用）
    /// **调用上下文**: 写事务内
    /// **状态修改**: 通过 `ctx.load_snapshot()` / `ctx.save_snapshot()` 操作
    /// **返回值**: `Vec<OrderEvent>` 支持多事件（拆单、合并等）
    ///
    /// **可访问服务**:
    /// - `ctx.state.price_rule_engine` - 价格规则引擎
    /// - `ctx.state.db` - SurrealDB（查询商品信息等）
    /// - `ctx.load_snapshot(id)` - 加载其他订单（跨订单操作）
    /// - `ctx.create_snapshot(id)` - 创建新订单
    /// - `ctx.save_snapshot(snapshot)` - 保存修改
    ///
    /// **示例**（拆单）：
    /// ```rust
    /// async fn execute(&self, ctx: &mut CommandContext<'_>, metadata: &CommandMetadata)
    ///     -> Result<Vec<OrderEvent>, OrderError>
    /// {
    ///     // 1. 加载源订单和目标订单
    ///     let mut source = ctx.load_snapshot(&self.source_order_id)?;
    ///     let mut target = ctx.load_snapshot(&self.target_order_id)?;
    ///
    ///     // 2. 业务逻辑：移动 items
    ///     let moved_items = source.items.drain(filter).collect();
    ///     target.items.extend(moved_items);
    ///
    ///     // 3. 保存修改
    ///     ctx.save_snapshot(source);
    ///     ctx.save_snapshot(target);
    ///
    ///     // 4. 生成两个 Event
    ///     Ok(vec![
    ///         OrderEvent { order_id: source_id, payload: ItemsRemoved { ... } },
    ///         OrderEvent { order_id: target_id, payload: ItemsAdded { ... } },
    ///     ])
    /// }
    /// ```
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError>;

    /// 副作用处理（可选）
    ///
    /// **执行时机**: 事务提交成功后（异步执行，不阻塞主流程）
    /// **调用上下文**: 事务外（已提交）
    /// **用途**: I/O 操作（打印、通知、归档到 SurrealDB）
    /// **重放行为**: 重放 Event 时**不调用**此方法
    ///
    /// **归档示例**：
    /// ```rust
    /// async fn on_success(&self, events: &[OrderEvent], state: &Arc<ServerState>)
    ///     -> Result<(), OrderError>
    /// {
    ///     // 如果订单完成，归档到 SurrealDB
    ///     if let Some(event) = events.iter().find(|e| matches!(e.payload, EventPayload::OrderCompleted { .. })) {
    ///         let snapshot = state.orders_manager().get_snapshot(&event.order_id)?;
    ///
    ///         // 归档到 SurrealDB
    ///         state.db.create("archived_orders").content(&snapshot).await?;
    ///
    ///         // 物理删除 Redb 数据
    ///         state.orders_manager().unload_order(&event.order_id)?;
    ///     }
    ///     Ok(())
    /// }
    /// ```
    async fn on_success(
        &self,
        _events: &[OrderEvent],
        _state: &Arc<ServerState>,
    ) -> Result<(), OrderError> {
        Ok(()) // 默认无副作用
    }
}
```

### 3.2 EventApplier Trait

```rust
/// 事件应用器 Trait
///
/// 职责：
/// 1. 从 Event 提取数据
/// 2. 应用到 snapshot（纯数据操作）
/// 3. 无业务逻辑，无副作用
pub trait EventApplier: Send + Sync {
    /// 应用事件到 snapshot
    ///
    /// **执行时机**:
    /// - 重放历史事件重建 snapshot
    /// - 从归档恢复订单
    ///
    /// **原则**:
    /// - 只从 Event 读取数据
    /// - 不重新执行业务逻辑（不生成新ID、不重新计算）
    /// - 不访问 DB、不执行 I/O
    /// - 幂等操作
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent);
}
```

---

## 4. enum_dispatch 实现

### 4.1 CommandAction Enum

```rust
// edge-server/src/orders/actions/mod.rs

use enum_dispatch::enum_dispatch;
use crate::orders::traits::CommandHandler;
use shared::order::{OrderCommand, OrderCommandPayload};

mod open_table;
mod add_items;
mod modify_item;
mod remove_item;
mod complete_order;
mod void_order;
mod restore_order;
mod restore_item;
mod add_payment;
mod cancel_payment;
mod split_order;
mod move_order;
mod merge_orders;
mod update_order_info;

pub use open_table::OpenTableAction;
pub use add_items::AddItemsAction;
pub use modify_item::ModifyItemAction;
pub use remove_item::RemoveItemAction;
pub use complete_order::CompleteOrderAction;
pub use void_order::VoidOrderAction;
pub use restore_order::RestoreOrderAction;
pub use restore_item::RestoreItemAction;
pub use add_payment::AddPaymentAction;
pub use cancel_payment::CancelPaymentAction;
pub use split_order::SplitOrderAction;
pub use move_order::MoveOrderAction;
pub use merge_orders::MergeOrdersAction;
pub use update_order_info::UpdateOrderInfoAction;

/// CommandAction enum - enum_dispatch wrapper
#[enum_dispatch(CommandHandler)]
pub enum CommandAction {
    OpenTable(OpenTableAction),
    AddItems(AddItemsAction),
    ModifyItem(ModifyItemAction),
    RemoveItem(RemoveItemAction),
    CompleteOrder(CompleteOrderAction),
    VoidOrder(VoidOrderAction),
    RestoreOrder(RestoreOrderAction),
    RestoreItem(RestoreItemAction),
    AddPayment(AddPaymentAction),
    CancelPayment(CancelPaymentAction),
    SplitOrder(SplitOrderAction),
    MoveOrder(MoveOrderAction),
    MergeOrders(MergeOrdersAction),
    UpdateOrderInfo(UpdateOrderInfoAction),
}

/// OrderCommand → CommandAction 转换
///
/// ⚠️ 唯一保留 match 的地方
impl From<OrderCommand> for CommandAction {
    fn from(cmd: OrderCommand) -> Self {
        match cmd.payload {
            OrderCommandPayload::OpenTable { table_id, table_name, zone_id, zone_name, guest_count, is_retail } => {
                CommandAction::OpenTable(OpenTableAction {
                    table_id,
                    table_name,
                    zone_id,
                    zone_name,
                    guest_count,
                    is_retail,
                })
            }
            OrderCommandPayload::AddItems { order_id, items } => {
                CommandAction::AddItems(AddItemsAction { order_id, items })
            }
            OrderCommandPayload::ModifyItem { order_id, instance_id, affected_quantity, changes, authorizer_id, authorizer_name } => {
                CommandAction::ModifyItem(ModifyItemAction {
                    order_id,
                    instance_id,
                    affected_quantity,
                    changes,
                    authorizer_id,
                    authorizer_name,
                })
            }
            OrderCommandPayload::RemoveItem { order_id, instance_id, quantity, reason, authorizer_id, authorizer_name } => {
                CommandAction::RemoveItem(RemoveItemAction {
                    order_id,
                    instance_id,
                    quantity,
                    reason,
                    authorizer_id,
                    authorizer_name,
                })
            }
            OrderCommandPayload::CompleteOrder { order_id, receipt_number } => {
                CommandAction::CompleteOrder(CompleteOrderAction { order_id, receipt_number })
            }
            OrderCommandPayload::VoidOrder { order_id, reason } => {
                CommandAction::VoidOrder(VoidOrderAction { order_id, reason })
            }
            OrderCommandPayload::RestoreOrder { order_id } => {
                CommandAction::RestoreOrder(RestoreOrderAction { order_id })
            }
            OrderCommandPayload::RestoreItem { order_id, instance_id } => {
                CommandAction::RestoreItem(RestoreItemAction { order_id, instance_id })
            }
            OrderCommandPayload::AddPayment { order_id, payment } => {
                CommandAction::AddPayment(AddPaymentAction { order_id, payment })
            }
            OrderCommandPayload::CancelPayment { order_id, payment_id, reason, authorizer_id, authorizer_name } => {
                CommandAction::CancelPayment(CancelPaymentAction {
                    order_id,
                    payment_id,
                    reason,
                    authorizer_id,
                    authorizer_name,
                })
            }
            OrderCommandPayload::SplitOrder { order_id, split_amount, payment_method, items } => {
                CommandAction::SplitOrder(SplitOrderAction {
                    order_id,
                    split_amount,
                    payment_method,
                    items,
                })
            }
            OrderCommandPayload::MoveOrder { order_id, target_table_id, target_table_name, target_zone_name } => {
                CommandAction::MoveOrder(MoveOrderAction {
                    order_id,
                    target_table_id,
                    target_table_name,
                    target_zone_name,
                })
            }
            OrderCommandPayload::MergeOrders { source_order_id, target_order_id } => {
                CommandAction::MergeOrders(MergeOrdersAction {
                    source_order_id,
                    target_order_id,
                })
            }
            OrderCommandPayload::UpdateOrderInfo { order_id, receipt_number, guest_count, table_name, is_pre_payment } => {
                CommandAction::UpdateOrderInfo(UpdateOrderInfoAction {
                    order_id,
                    receipt_number,
                    guest_count,
                    table_name,
                    is_pre_payment,
                })
            }
        }
    }
}
```

### 4.2 EventAction Enum

```rust
// edge-server/src/orders/appliers/mod.rs

use enum_dispatch::enum_dispatch;
use crate::orders::traits::EventApplier;
use shared::order::{OrderEvent, EventPayload};

mod table_opened;
mod items_added;
mod item_modified;
mod item_removed;
mod order_completed;
mod order_voided;
mod order_restored;
mod item_restored;
mod payment_added;
mod payment_cancelled;
mod order_split;
mod order_moved;
mod orders_merged;
mod order_info_updated;

pub use table_opened::TableOpenedApplier;
pub use items_added::ItemsAddedApplier;
pub use item_modified::ItemModifiedApplier;
pub use item_removed::ItemRemovedApplier;
pub use order_completed::OrderCompletedApplier;
pub use order_voided::OrderVoidedApplier;
pub use order_restored::OrderRestoredApplier;
pub use item_restored::ItemRestoredApplier;
pub use payment_added::PaymentAddedApplier;
pub use payment_cancelled::PaymentCancelledApplier;
pub use order_split::OrderSplitApplier;
pub use order_moved::OrderMovedApplier;
pub use orders_merged::OrdersMergedApplier;
pub use order_info_updated::OrderInfoUpdatedApplier;

/// EventAction enum - enum_dispatch wrapper
#[enum_dispatch(EventApplier)]
pub enum EventAction {
    TableOpened(TableOpenedApplier),
    ItemsAdded(ItemsAddedApplier),
    ItemModified(ItemModifiedApplier),
    ItemRemoved(ItemRemovedApplier),
    OrderCompleted(OrderCompletedApplier),
    OrderVoided(OrderVoidedApplier),
    OrderRestored(OrderRestoredApplier),
    ItemRestored(ItemRestoredApplier),
    PaymentAdded(PaymentAddedApplier),
    PaymentCancelled(PaymentCancelledApplier),
    OrderSplit(OrderSplitApplier),
    OrderMoved(OrderMovedApplier),
    OrdersMerged(OrdersMergedApplier),
    OrderInfoUpdated(OrderInfoUpdatedApplier),
}

/// OrderEvent → EventAction 转换
///
/// ⚠️ 唯一保留 match 的地方
impl From<&OrderEvent> for EventAction {
    fn from(event: &OrderEvent) -> Self {
        match &event.payload {
            EventPayload::TableOpened { .. } => EventAction::TableOpened(TableOpenedApplier),
            EventPayload::ItemsAdded { .. } => EventAction::ItemsAdded(ItemsAddedApplier),
            EventPayload::ItemModified { .. } => EventAction::ItemModified(ItemModifiedApplier),
            EventPayload::ItemRemoved { .. } => EventAction::ItemRemoved(ItemRemovedApplier),
            EventPayload::OrderCompleted { .. } => EventAction::OrderCompleted(OrderCompletedApplier),
            EventPayload::OrderVoided { .. } => EventAction::OrderVoided(OrderVoidedApplier),
            EventPayload::OrderRestored { .. } => EventAction::OrderRestored(OrderRestoredApplier),
            EventPayload::ItemRestored { .. } => EventAction::ItemRestored(ItemRestoredApplier),
            EventPayload::PaymentAdded { .. } => EventAction::PaymentAdded(PaymentAddedApplier),
            EventPayload::PaymentCancelled { .. } => EventAction::PaymentCancelled(PaymentCancelledApplier),
            EventPayload::OrderSplit { .. } => EventAction::OrderSplit(OrderSplitApplier),
            EventPayload::OrderMoved { .. } => EventAction::OrderMoved(OrderMovedApplier),
            EventPayload::OrdersMerged { .. } => EventAction::OrdersMerged(OrdersMergedApplier),
            EventPayload::OrderInfoUpdated { .. } => EventAction::OrderInfoUpdated(OrderInfoUpdatedApplier),
        }
    }
}
```

---

## 5. 具体实现示例

### 5.1 AddItemsAction (Command Handler) - 完整版

```rust
// edge-server/src/orders/actions/add_items.rs

use crate::orders::traits::{CommandHandler, CommandContext, CommandMetadata, OrderError};
use crate::core::ServerState;
use shared::order::{OrderSnapshot, OrderEvent, EventPayload, CartItemInput, CartItemSnapshot};
use async_trait::async_trait;
use uuid::Uuid;
use std::sync::Arc;

/// AddItems 命令处理器
pub struct AddItemsAction {
    pub order_id: String,
    pub items: Vec<CartItemInput>,
}

#[async_trait]
impl CommandHandler for AddItemsAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. 加载订单 Snapshot
        let mut snapshot = ctx.load_snapshot(&self.order_id)?;

        // 2. 验证订单状态
        if snapshot.status == shared::order::OrderStatus::Completed {
            return Err(OrderError::OrderAlreadyCompleted(self.order_id.clone()));
        }
        if snapshot.status == shared::order::OrderStatus::Voided {
            return Err(OrderError::OrderAlreadyVoided(self.order_id.clone()));
        }

        // 3. 应用价格规则（访问 ctx.state.price_rule_engine）
        let zone_id = snapshot.zone_id.as_deref();
        let is_retail = snapshot.is_retail;

        let rules = ctx.state
            .price_rule_engine
            .load_rules_for_zone(zone_id, is_retail)
            .await;

        let current_time = chrono::Utc::now().timestamp_millis();

        let items_with_rules = if !rules.is_empty() {
            ctx.state
                .price_rule_engine
                .apply_rules(self.items.clone(), &rules, current_time)
                .await
        } else {
            self.items.clone()
        };

        // 4. 业务逻辑：生成 instance_id、转换为 CartItemSnapshot
        let processed_items: Vec<CartItemSnapshot> = items_with_rules
            .iter()
            .map(|input| CartItemSnapshot {
                instance_id: Uuid::new_v4().to_string(), // 生成唯一ID
                item_id: input.item_id.clone(),
                item_name: input.item_name.clone(),
                item_name_zh: input.item_name_zh.clone(),
                category_id: input.category_id.clone(),
                quantity: input.quantity,
                unit_price: input.unit_price, // 已应用价格规则
                total_price: input.unit_price * input.quantity as f64,
                modifiers: input.modifiers.clone(),
                notes: input.notes.clone(),
                is_voided: false,
            })
            .collect();

        // 5. 修改 snapshot
        snapshot.items.extend(processed_items.clone());
        snapshot.sequence += 1;

        // 6. 重算聚合字段和 Hash
        snapshot.recalculate();

        // 7. 保存到 Context
        ctx.save_snapshot(snapshot.clone());

        // 8. 构造 Event（包含完整的 processed_items）
        Ok(vec![OrderEvent {
            event_id: Uuid::new_v4().to_string(),
            event_type: shared::order::OrderEventType::ItemsAdded,
            order_id: self.order_id.clone(),
            sequence: snapshot.sequence,
            timestamp: metadata.timestamp,
            operator_id: metadata.operator_id.clone(),
            operator_name: metadata.operator_name.clone(),
            payload: EventPayload::ItemsAdded {
                items: processed_items, // ← 包含 instance_id 和最终价格
            },
        }])
    }

    async fn on_success(
        &self,
        events: &[OrderEvent],
        state: &Arc<ServerState>,
    ) -> Result<(), OrderError> {
        // TODO: 发送到厨房打印机
        // for event in events {
        //     state.kitchen_printer.print(event).await?;
        // }
        Ok(())
    }
}
```

### 5.2 ItemsAddedApplier (Event Applier) - 完整版

```rust
// edge-server/src/orders/appliers/items_added.rs

use crate::orders::traits::EventApplier;
use shared::order::{OrderSnapshot, OrderEvent, EventPayload};

/// ItemsAdded 事件应用器
///
/// **职责**：纯数据操作，从 Event 提取数据更新 Snapshot
/// **原则**：无业务逻辑，无 I/O，无副作用
pub struct ItemsAddedApplier;

impl EventApplier for ItemsAddedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::ItemsAdded { items } = &event.payload {
            // 1. 直接添加（Event 中已包含 instance_id、最终价格）
            snapshot.items.extend(items.clone());

            // 2. 更新 sequence
            snapshot.sequence = event.sequence;

            // 3. 重算聚合字段和 Hash
            snapshot.recalculate(); // ← 必须调用，更新 total_amount 和 content_hash
        }
    }
}
```

### 5.3 ModifyItemAction (复杂场景)

```rust
// edge-server/src/orders/actions/modify_item.rs

use crate::orders::traits::{CommandHandler, CommandMetadata, OrderError};
use crate::orders::storage::WriteTransaction;
use crate::core::ServerState;
use shared::order::{
    OrderSnapshot, OrderEvent, EventPayload, ItemChanges,
    ItemModificationResult, CartItemSnapshot,
};
use async_trait::async_trait;
use uuid::Uuid;

pub struct ModifyItemAction {
    pub order_id: String,
    pub instance_id: String,
    pub affected_quantity: Option<i32>,
    pub changes: ItemChanges,
    pub authorizer_id: Option<String>,
    pub authorizer_name: Option<String>,
}

#[async_trait]
impl CommandHandler for ModifyItemAction {
    async fn execute(
        &self,
        snapshot: &mut OrderSnapshot,
        metadata: &CommandMetadata,
        _txn: &WriteTransaction,
    ) -> Result<OrderEvent, OrderError> {
        // 1. 找到源 item
        let source_item = snapshot.items.iter()
            .find(|item| item.instance_id == self.instance_id && !item.is_voided)
            .ok_or_else(|| OrderError::ItemNotFound(self.instance_id.clone()))?
            .clone();

        // 2. 确定影响数量
        let affected_qty = self.affected_quantity.unwrap_or(source_item.quantity);
        if affected_qty > source_item.quantity {
            return Err(OrderError::InsufficientQuantity);
        }

        // 3. 应用修改，生成新 item
        let mut modified_item = source_item.clone();
        modified_item.instance_id = Uuid::new_v4().to_string(); // 新 instance_id
        modified_item.quantity = affected_qty;

        // 应用 changes
        if let Some(new_price) = self.changes.unit_price {
            modified_item.unit_price = new_price;
        }
        if let Some(new_modifiers) = &self.changes.modifiers {
            modified_item.modifiers = new_modifiers.clone();
        }
        if let Some(new_notes) = &self.changes.notes {
            modified_item.notes = new_notes.clone();
        }

        modified_item.total_price = modified_item.unit_price * modified_item.quantity as f64;

        // 4. 修改 snapshot
        // 4.1 减少源 item 数量（如果部分修改）
        if affected_qty < source_item.quantity {
            let remaining_item = snapshot.items.iter_mut()
                .find(|item| item.instance_id == self.instance_id)
                .unwrap();
            remaining_item.quantity -= affected_qty;
            remaining_item.total_price = remaining_item.unit_price * remaining_item.quantity as f64;
        } else {
            // 完全替换，标记源 item 为 voided
            let old_item = snapshot.items.iter_mut()
                .find(|item| item.instance_id == self.instance_id)
                .unwrap();
            old_item.is_voided = true;
        }

        // 4.2 添加新 item
        snapshot.items.push(modified_item.clone());
        snapshot.sequence += 1;

        // 重算总额
        snapshot.total_amount = snapshot.items.iter()
            .filter(|item| !item.is_voided)
            .map(|item| item.total_price)
            .sum();

        // 5. 构造 Event
        let results = vec![ItemModificationResult {
            source_instance_id: self.instance_id.clone(),
            new_instance_id: modified_item.instance_id.clone(),
            new_item: modified_item.clone(),
            remaining_quantity: if affected_qty < source_item.quantity {
                Some(source_item.quantity - affected_qty)
            } else {
                None
            },
        }];

        Ok(OrderEvent {
            event_id: Uuid::new_v4().to_string(),
            event_type: shared::order::OrderEventType::ItemModified,
            order_id: self.order_id.clone(),
            sequence: snapshot.sequence,
            timestamp: metadata.timestamp,
            operator_id: metadata.operator_id.clone(),
            operator_name: metadata.operator_name.clone(),
            payload: EventPayload::ItemModified {
                source: self.instance_id.clone(),
                affected_quantity: affected_qty,
                changes: self.changes.clone(),
                results,
                authorizer_id: self.authorizer_id.clone(),
                authorizer_name: self.authorizer_name.clone(),
            },
        })
    }
}
```

---

## 6. OrdersManager 重构

### 6.1 新的 execute_command 流程（完整版）

```rust
// edge-server/src/orders/manager.rs

use crate::orders::actions::CommandAction;
use crate::orders::appliers::EventAction;
use crate::orders::traits::{CommandHandler, CommandContext, EventApplier, CommandMetadata};
use crate::core::ServerState;
use std::sync::Arc;

pub struct OrdersManager {
    storage: OrderStorage,
    event_tx: broadcast::Sender<OrderEvent>,
    epoch: String,
    state: Arc<ServerState>,
}

impl OrdersManager {
    pub fn execute_command(&self, cmd: OrderCommand) -> CommandResponse {
        // 1. 幂等性检查
        if let Ok(Some(_)) = self.storage.get_processed_command(&cmd.command_id) {
            tracing::debug!("Command {} already processed (idempotent)", cmd.command_id);
            return CommandResponse::success(cmd.command_id.clone(), None);
        }

        // 2. 执行命令
        match self.process_command_internal(cmd.clone()) {
            Ok((response, events)) => {
                // 3. 广播所有 events
                for event in &events {
                    if let Err(e) = self.event_tx.send(event.clone()) {
                        tracing::warn!("Failed to broadcast event {}: {}", event.event_id, e);
                    }
                }

                // 4. 执行副作用（异步，不阻塞）
                let action: CommandAction = cmd.into();
                let events_clone = events.clone();
                let state_clone = self.state.clone();

                tokio::spawn(async move {
                    if let Err(e) = action.on_success(&events_clone, &state_clone).await {
                        tracing::warn!("Side effect failed: {}", e);
                    }
                });

                response
            }
            Err(err) => CommandResponse::error(cmd.command_id, err.into()),
        }
    }

    async fn process_command_internal(&self, cmd: OrderCommand)
        -> ManagerResult<(CommandResponse, Vec<OrderEvent>)>
    {
        // 1. 开启写事务
        let txn = self.storage.begin_write()?;

        // 2. 创建 CommandContext
        let mut ctx = CommandContext::new(
            &txn,
            &self.storage,
            self.epoch.clone(),
            &self.state,
        );

        // 3. 构造元数据
        let metadata = CommandMetadata {
            command_id: cmd.command_id.clone(),
            operator_id: cmd.operator_id.clone(),
            operator_name: cmd.operator_name.clone(),
            timestamp: cmd.timestamp,
        };

        // 4. 转换为 Action 并执行（enum_dispatch，无 match）
        let action: CommandAction = cmd.clone().into();
        let events = action.execute(&mut ctx, &metadata).await?;

        // 5. 持久化所有 events（可能跨多个订单）
        for event in &events {
            self.storage.persist_event(&txn, event)?;
        }

        // 6. 更新所有修改过的 snapshots
        for snapshot in ctx.modified_snapshots() {
            // 验证 Hash
            if !snapshot.verify_hash() {
                tracing::error!(
                    "❌ Hash verification failed before persist: order {}",
                    snapshot.order_id
                );
                return Err(ManagerError::Internal(
                    format!("Hash mismatch for order {}", snapshot.order_id)
                ));
            }

            self.storage.update_snapshot(&txn, snapshot)?;
        }

        // 7. 标记命令已处理
        self.storage.mark_command_processed(&txn, &cmd.command_id)?;

        // 8. 提交事务
        txn.commit()?;

        // 9. 返回主订单的 Snapshot（第一个 event 的 order_id）
        let main_order_id = events.first()
            .map(|e| e.order_id.as_str())
            .ok_or_else(|| ManagerError::Internal("No events generated".into()))?;

        let final_snapshot = ctx.modified_snapshots()
            .into_iter()
            .find(|s| s.order_id == main_order_id)
            .cloned();

        Ok((
            CommandResponse::success(cmd.command_id, final_snapshot),
            events,
        ))
    }

    /// 卸载订单（物理删除 Redb 数据）
    pub fn unload_order(&self, order_id: &str) -> Result<(), ManagerError> {
        let txn = self.storage.begin_write()?;

        self.storage.delete_events_for_order(&txn, order_id)?;
        self.storage.delete_snapshot(&txn, order_id)?;
        self.storage.delete_command_records(&txn, order_id)?;

        txn.commit()?;

        tracing::info!("Order {} unloaded from Redb", order_id);

        Ok(())
    }
}
```

### 6.2 Event 重放

```rust
impl OrdersManager {
    /// 从事件流重建 snapshot
    pub fn rebuild_snapshot(&self, order_id: &str) -> ManagerResult<OrderSnapshot> {
        let txn = self.storage.begin_read()?;

        // 1. 加载所有 events
        let events = self.storage.get_events_for_order(&txn, order_id)?;

        // 2. 创建空 snapshot
        let mut snapshot = OrderSnapshot::new(order_id.to_string(), self.epoch.clone());

        // 3. 依次应用 events（enum_dispatch，无 match）
        for event in events {
            let applier: EventAction = (&event).into();
            applier.apply(&mut snapshot, &event);
        }

        Ok(snapshot)
    }
}
```

---

## 7. 迁移步骤

### Phase 1: 基础设施准备 (1-2 天)

**任务**：
1. ✅ 添加 `enum_dispatch` 依赖到 `Cargo.toml`
2. ✅ 创建 `edge-server/src/orders/traits.rs`
   - 定义 `CommandHandler` trait
   - 定义 `EventApplier` trait
   - 定义 `CommandMetadata` struct
   - 定义 `OrderError` enum
3. ✅ 创建文件夹结构
   - `edge-server/src/orders/actions/`
   - `edge-server/src/orders/appliers/`

**验证**：
- `cargo check` 通过
- 文件结构就绪

---

### Phase 2: 实现 Actions (3-5 天)

**优先级顺序**（按使用频率）：
1. 🔴 **高优先级**（核心流程）:
   - `OpenTableAction` / `TableOpenedApplier`
   - `AddItemsAction` / `ItemsAddedApplier`
   - `CompleteOrderAction` / `OrderCompletedApplier`
   - `AddPaymentAction` / `PaymentAddedApplier`

2. 🟡 **中优先级**（常用功能）:
   - `ModifyItemAction` / `ItemModifiedApplier`
   - `RemoveItemAction` / `ItemRemovedApplier`
   - `VoidOrderAction` / `OrderVoidedApplier`
   - `UpdateOrderInfoAction` / `OrderInfoUpdatedApplier`

3. 🟢 **低优先级**（辅助功能）:
   - `CancelPaymentAction` / `PaymentCancelledApplier`
   - `MoveOrderAction` / `OrderMovedApplier`
   - `MergeOrdersAction` / `OrdersMergedApplier`
   - `SplitOrderAction` / `OrderSplitApplier`
   - `RestoreOrderAction` / `OrderRestoredApplier`
   - `RestoreItemAction` / `ItemRestoredApplier`

**实现策略**：
- 每次实现一对 (Action + Applier)
- 从现有 `handle_xxx` 方法迁移业务逻辑
- Event payload 需包含完整数据（instance_id、计算后的值）

**验证**：
- 每个 Action 单元测试
- 对比新旧实现生成的 Event 是否一致

---

### Phase 3: enum_dispatch 集成 (1 天)

**任务**：
1. ✅ 实现 `CommandAction` enum (actions/mod.rs)
2. ✅ 实现 `EventAction` enum (appliers/mod.rs)
3. ✅ 实现 `From<OrderCommand> for CommandAction`
4. ✅ 实现 `From<&OrderEvent> for EventAction`

**验证**：
- `cargo check` 通过
- enum_dispatch 宏展开正确

---

### Phase 4: OrdersManager 重构 (2 天)

**任务**：
1. ✅ 重构 `execute_command()` 使用 `CommandAction`
2. ✅ 重构 `process_command_internal()` 移除 match
3. ✅ 实现 `rebuild_snapshot()` 使用 `EventAction`
4. ✅ 添加 `on_success` 异步调用
5. ❌ 删除所有旧的 `handle_xxx` 方法
6. ❌ 删除 `reducer.rs` 中的 `apply_event` match

**验证**：
- 集成测试通过
- 与旧版本行为一致性测试

---

### Phase 5: 测试与验证 (2-3 天)

**测试范围**：
1. **单元测试**:
   - 每个 Action 的 `execute()` 测试
   - 每个 Applier 的 `apply()` 测试
   - 错误路径测试（订单不存在、状态错误等）

2. **集成测试**:
   - 完整命令流程（Command → Event → Broadcast）
   - Event 重放测试（rebuild_snapshot）
   - 并发命令测试
   - 幂等性测试

3. **性能测试**:
   - enum_dispatch vs match 性能对比
   - 内存占用测试

4. **回归测试**:
   - 现有测试套件必须全部通过
   - processor.rs 集成测试

**验证**：
- `cargo test --workspace` 全部通过
- 性能无退化

---

### Phase 6: 清理与文档 (1 天)

**任务**：
1. ✅ 删除旧代码（manager.rs 中的 handle_xxx 方法）
2. ✅ 删除 reducer.rs 或改为仅导出 EventAction
3. ✅ 更新 mod.rs exports
4. ✅ 编写架构文档
5. ✅ 编写迁移指南（如何添加新命令）

**验证**：
- 代码审查通过
- 文档完整

---

## 8. 风险评估

### 8.1 高风险项

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| **Event payload 数据不完整** | 重放失败，snapshot 错误 | ✅ 每个 Event 严格 code review<br>✅ 添加重放测试验证 |
| **业务逻辑迁移错误** | 产生错误的订单数据 | ✅ 对比新旧实现的 Event 输出<br>✅ 并行运行测试 |
| **副作用重复执行** | 厨房打印机重复出单 | ✅ on_success 只在新命令时调用<br>✅ 重放时不调用 on_success |
| **并发安全问题** | redb 事务冲突 | ✅ 保持现有事务隔离级别<br>✅ 并发测试 |

### 8.2 中风险项

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| **enum_dispatch 编译错误** | 无法构建 | ✅ 小步迭代，频繁编译验证 |
| **性能退化** | 吞吐量下降 | ✅ 性能基准测试<br>✅ enum_dispatch 通常无性能损失 |
| **snapshot 重算逻辑遗漏** | 总额错误 | ✅ recalculate_totals() 统一调用<br>✅ 添加总额验证测试 |

### 8.3 低风险项

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| **文件数量增加** | 导航不便 | ✅ 统一命名规范<br>✅ IDE 快速跳转 |
| **From 转换 match 遗漏** | 编译错误 | ✅ exhaustive match 编译检查 |

---

## 9. 测试策略

### 9.1 单元测试示例

```rust
// edge-server/src/orders/actions/add_items.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orders::traits::CommandMetadata;
    use shared::order::{OrderSnapshot, CartItemInput};

    #[tokio::test]
    async fn test_add_items_execute() {
        // Arrange
        let mut snapshot = OrderSnapshot::new("order-123".into(), "2024-01".into());
        snapshot.status = OrderStatus::Active;

        let action = AddItemsAction {
            order_id: "order-123".into(),
            items: vec![
                CartItemInput {
                    item_id: "item-1".into(),
                    item_name: "Coffee".into(),
                    quantity: 2,
                    unit_price: 5.0,
                    ..Default::default()
                }
            ],
        };

        let metadata = CommandMetadata {
            command_id: "cmd-1".into(),
            operator_id: "user-1".into(),
            operator_name: "Alice".into(),
            timestamp: 1234567890,
        };

        // Act
        let event = action.execute(&mut snapshot, &metadata, &mock_txn()).await.unwrap();

        // Assert
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].quantity, 2);
        assert_eq!(snapshot.total_amount, 10.0);
        assert!(!snapshot.items[0].instance_id.is_empty()); // 确保生成了 ID

        // 验证 Event
        if let EventPayload::ItemsAdded { items } = event.payload {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].instance_id, snapshot.items[0].instance_id);
        } else {
            panic!("Wrong event payload");
        }
    }

    #[tokio::test]
    async fn test_add_items_to_completed_order_fails() {
        let mut snapshot = OrderSnapshot::new("order-123".into(), "2024-01".into());
        snapshot.status = OrderStatus::Completed; // ❌ 已完成

        let action = AddItemsAction {
            order_id: "order-123".into(),
            items: vec![],
        };

        let result = action.execute(&mut snapshot, &mock_metadata(), &mock_txn()).await;

        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }
}
```

### 9.2 集成测试示例

```rust
// edge-server/tests/orders_integration_test.rs

#[tokio::test]
async fn test_complete_order_flow() {
    // 1. 创建 OrdersManager
    let manager = OrdersManager::new(...);

    // 2. OpenTable
    let open_cmd = OrderCommand {
        command_id: "cmd-1".into(),
        payload: OrderCommandPayload::OpenTable {
            table_id: Some("T1".into()),
            guest_count: 2,
            is_retail: false,
            ..Default::default()
        },
        ..Default::default()
    };

    let resp1 = manager.execute_command(open_cmd);
    assert!(resp1.success);
    let order_id = resp1.data.unwrap().order_id;

    // 3. AddItems
    let add_cmd = OrderCommand {
        command_id: "cmd-2".into(),
        payload: OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![CartItemInput { ... }],
        },
        ..Default::default()
    };

    let resp2 = manager.execute_command(add_cmd);
    assert!(resp2.success);

    // 4. 重放 Events 验证
    let rebuilt = manager.rebuild_snapshot(&order_id).unwrap();
    let current = manager.get_snapshot(&order_id).unwrap().unwrap();

    assert_eq!(rebuilt.items.len(), current.items.len());
    assert_eq!(rebuilt.total_amount, current.total_amount);
}
```

---

## 10. 性能影响分析

### 10.1 enum_dispatch vs match

**enum_dispatch 优势**：
```rust
// 传统 match (动态分发)
match payload {
    Variant1 => handler1(),  // 每次都要匹配
    Variant2 => handler2(),
    ...
}

// enum_dispatch (静态分发)
// 编译期生成类似以下代码：
impl CommandHandler for CommandAction {
    fn execute(...) {
        match self {
            Self::AddItems(h) => h.execute(...),  // 编译期已确定
            Self::ModifyItem(h) => h.execute(...),
            ...
        }
    }
}
```

**性能对比**：
- ✅ 零成本抽象（编译期单态化）
- ✅ 无虚函数表开销
- ✅ 内联优化更激进
- ⚠️ 二进制大小稍增（每个 variant 生成独立代码）

**预期影响**：
- 延迟：**无变化**（可能略微改善）
- 吞吐量：**无变化**
- 内存：**轻微增加**（+50KB 左右，可忽略）

---

## 11. 断电重播与生命周期管理

### 11.1 断电重启恢复流程

**场景**：Edge Server 异常关闭（断电、崩溃），重启后需要恢复所有活跃订单状态。

**核心原则**：
- ✅ Redb 持久化保证事务原子性（ACID）
- ✅ 从 Snapshot + 增量 Events 重建状态
- ✅ 重放时**不执行**业务逻辑（不生成新 ID、不调用 on_success）

**恢复步骤**：

```rust
// edge-server/src/orders/manager.rs

impl OrdersManager {
    /// 启动时恢复所有活跃订单
    pub fn recover_on_startup(&self) -> Result<usize, ManagerError> {
        let txn = self.storage.begin_read()?;

        // 1. 加载所有活跃订单的 Snapshot
        let snapshots = self.storage.list_active_snapshots(&txn)?;
        tracing::info!("Found {} active orders to recover", snapshots.len());

        // 2. 对每个订单，检查是否有增量 Events
        for mut snapshot in snapshots {
            let order_id = &snapshot.order_id;

            // 3. 获取该订单的所有 Events (sequence > snapshot.sequence)
            let incremental_events = self.storage.get_events_since(
                &txn,
                order_id,
                snapshot.sequence,
            )?;

            if incremental_events.is_empty() {
                tracing::debug!("Order {} snapshot is up-to-date", order_id);
                continue;
            }

            tracing::info!(
                "Replaying {} incremental events for order {}",
                incremental_events.len(),
                order_id
            );

            // 4. 依次应用增量 Events（纯函数，无副作用）
            for event in &incremental_events {
                let applier: EventAction = event.into();
                applier.apply(&mut snapshot, event);
            }

            // 5. 更新 Snapshot（写回最新状态）
            drop(txn); // 结束读事务
            let write_txn = self.storage.begin_write()?;
            self.storage.update_snapshot(&write_txn, &snapshot)?;
            write_txn.commit()?;
            let txn = self.storage.begin_read()?; // 重新开启读事务
        }

        Ok(snapshots.len())
    }

    /// 完全重建 Snapshot（用于测试或修复）
    pub fn rebuild_snapshot(&self, order_id: &str) -> Result<OrderSnapshot, ManagerError> {
        let txn = self.storage.begin_read()?;

        // 1. 加载所有 Events
        let events = self.storage.get_all_events_for_order(&txn, order_id)?;

        // 2. 创建空 Snapshot
        let mut snapshot = OrderSnapshot::new(order_id.to_string(), self.epoch.clone());

        // 3. 依次应用所有 Events
        for event in &events {
            let applier: EventAction = event.into();
            applier.apply(&mut snapshot, event);
        }

        Ok(snapshot)
    }
}
```

**关键要点**：
- ✅ **Applier 必须是纯函数**：只读取 Event 数据，不访问 DB、不生成 ID
- ✅ **重放时不调用 on_success**：避免副作用重复执行（重复打印、重复归档）
- ✅ **Snapshot 定期持久化**：减少重放的 Event 数量

---

### 11.2 归档与卸载机制

**设计目标**：
- Redb 只存储活跃订单（内存压力小）
- 已完成订单归档到 SurrealDB（长期存储）
- 归档成功后物理删除 Redb 数据

**实现策略**：

```rust
// edge-server/src/orders/actions/complete_order.rs

pub struct CompleteOrderAction {
    pub order_id: String,
    pub receipt_number: String,
}

#[async_trait]
impl CommandHandler for CompleteOrderAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. 加载订单
        let mut snapshot = ctx.load_snapshot(&self.order_id)?;

        // 2. 验证状态
        if snapshot.status == OrderStatus::Completed {
            return Err(OrderError::OrderAlreadyCompleted(self.order_id.clone()));
        }

        // 3. 修改状态
        snapshot.status = OrderStatus::Completed;
        snapshot.receipt_number = Some(self.receipt_number.clone());
        snapshot.completed_at = Some(metadata.timestamp);
        snapshot.sequence += 1;

        // 4. 保存
        ctx.save_snapshot(snapshot.clone());

        // 5. 生成 Event
        Ok(vec![OrderEvent {
            event_id: Uuid::new_v4().to_string(),
            event_type: OrderEventType::OrderCompleted,
            order_id: self.order_id.clone(),
            sequence: snapshot.sequence,
            timestamp: metadata.timestamp,
            operator_id: metadata.operator_id.clone(),
            operator_name: metadata.operator_name.clone(),
            payload: EventPayload::OrderCompleted {
                receipt_number: self.receipt_number.clone(),
            },
        }])
    }

    async fn on_success(
        &self,
        events: &[OrderEvent],
        state: &Arc<ServerState>,
    ) -> Result<(), OrderError> {
        // ✅ 副作用：归档到 SurrealDB + 卸载 Redb 数据
        if let Some(event) = events.iter().find(|e| {
            matches!(e.payload, EventPayload::OrderCompleted { .. })
        }) {
            let order_id = &event.order_id;

            tracing::info!("Archiving completed order: {}", order_id);

            // 1. 获取最终 Snapshot
            let snapshot = state.orders_manager()
                .get_snapshot(order_id)
                .map_err(|e| OrderError::Storage(e.to_string()))?
                .ok_or_else(|| OrderError::OrderNotFound(order_id.clone()))?;

            // 2. 归档到 SurrealDB
            state.db
                .create::<Option<serde_json::Value>>("archived_orders")
                .content(&snapshot)
                .await
                .map_err(|e| OrderError::Storage(format!("Archive failed: {}", e)))?;

            tracing::info!("Order {} archived to SurrealDB", order_id);

            // 3. 物理删除 Redb 数据（Events + Snapshot）
            state.orders_manager()
                .unload_order(order_id)
                .map_err(|e| OrderError::Storage(format!("Unload failed: {}", e)))?;

            tracing::info!("Order {} unloaded from Redb", order_id);
        }

        Ok(())
    }
}
```

**OrdersManager 卸载方法**：

```rust
impl OrdersManager {
    /// 卸载订单（物理删除 Redb 数据）
    ///
    /// **前置条件**：订单已归档到 SurrealDB
    pub fn unload_order(&self, order_id: &str) -> Result<(), ManagerError> {
        let txn = self.storage.begin_write()?;

        // 1. 删除所有 Events
        self.storage.delete_events_for_order(&txn, order_id)?;

        // 2. 删除 Snapshot
        self.storage.delete_snapshot(&txn, order_id)?;

        // 3. 删除处理记录
        self.storage.delete_command_records(&txn, order_id)?;

        txn.commit()?;

        tracing::info!("Order {} physically deleted from Redb", order_id);

        Ok(())
    }
}
```

---

### ⚠️ 关键边界情况：CompleteOrder 重放与幽灵订单

**问题场景**：
```
1. CompleteOrder 执行成功
2. Event 已持久化到 Redb ✅
3. 事务提交 ✅
4. on_success 开始执行归档...
5. 💥 崩溃（归档未完成，Redb 未删除）
6. 重启 → 重放 CompleteOrder Event
7. ❓ 订单状态 = Completed，但没归档到 SurrealDB
```

**设计原则**：
- ✅ **Event 重放时不调用 on_success**（避免重复打印、重复归档）
- ✅ **Event 本身是正确的**（CompleteOrder 确实发生了）
- ⚠️ **需要补偿逻辑**：检查 Completed 订单是否已归档

**解决方案：启动时补偿检查**

```rust
impl OrdersManager {
    /// 启动时完整恢复流程
    pub async fn recover_on_startup(&self) -> Result<(), ManagerError> {
        // 1. 恢复 OrderNumberGenerator
        self.order_number_gen.recover()?;

        // 2. 恢复所有活跃订单的 Snapshot
        let recovered_count = self.replay_incremental_events()?;
        tracing::info!("Recovered {} orders from events", recovered_count);

        // 3. 🔑 补偿检查：处理未完成归档的订单
        self.compensate_pending_archives().await?;

        Ok(())
    }

    /// 补偿检查：处理崩溃时未完成归档的订单
    async fn compensate_pending_archives(&self) -> Result<(), ManagerError> {
        let txn = self.storage.begin_read()?;

        // 1. 查找所有 Completed 状态但还在 Redb 中的订单
        let snapshots = self.storage.list_active_snapshots(&txn)?;
        let completed_orders: Vec<_> = snapshots
            .into_iter()
            .filter(|s| s.status == OrderStatus::Completed)
            .collect();

        if completed_orders.is_empty() {
            tracing::debug!("No pending archives to compensate");
            return Ok(());
        }

        tracing::warn!(
            "Found {} completed orders still in Redb, running compensation",
            completed_orders.len()
        );

        // 2. 对每个已完成但未归档的订单，执行补偿
        for snapshot in completed_orders {
            let order_id = &snapshot.order_id;

            // 检查是否已在 SurrealDB 中
            let already_archived = self.state.db
                .query("SELECT * FROM archived_orders WHERE order_id = $id")
                .bind(("id", order_id))
                .await
                .map(|mut result| {
                    result.take::<Vec<serde_json::Value>>(0)
                        .map(|v| !v.is_empty())
                        .unwrap_or(false)
                })
                .unwrap_or(false);

            if already_archived {
                // 已归档，只需删除 Redb 数据
                tracing::info!("Order {} already archived, unloading from Redb", order_id);
                self.unload_order(order_id)?;
            } else {
                // 未归档，执行完整归档流程
                tracing::info!("Order {} not archived, running archive compensation", order_id);

                // 归档到 SurrealDB
                self.state.db
                    .create::<Option<serde_json::Value>>("archived_orders")
                    .content(&snapshot)
                    .await
                    .map_err(|e| ManagerError::Internal(format!("Archive failed: {}", e)))?;

                // 从 Redb 卸载
                self.unload_order(order_id)?;

                tracing::info!("Order {} archive compensation completed", order_id);
            }
        }

        Ok(())
    }
}
```

**补偿流程图**：

```
┌─────────────────────────────────────────────────────────────────────┐
│  recover_on_startup()                                                │
│    1. 恢复 OrderNumberGenerator                                      │
│    2. 重放增量 Events → 重建 Snapshots                               │
│    3. compensate_pending_archives()                                  │
│       ├─ 查找所有 status=Completed 但还在 Redb 中的订单              │
│       │                                                              │
│       ▼  For each completed order:                                   │
│       ├─ 检查是否已在 SurrealDB                                      │
│       ├─ 如果已归档 → 直接 unload_order()                            │
│       └─ 如果未归档 → 归档到 SurrealDB → unload_order()              │
└─────────────────────────────────────────────────────────────────────┘
```

**为什么不会产生幽灵订单？**

| 场景 | 处理方式 | 结果 |
|------|----------|------|
| **正常完成** | CompleteOrder → on_success 归档 → unload | ✅ 订单在 SurrealDB，Redb 已清空 |
| **崩溃在归档前** | 重启 → 重放 Event → 补偿归档 | ✅ 补偿逻辑完成归档 |
| **崩溃在卸载前** | 重启 → 检测已归档 → 直接卸载 | ✅ 只删除 Redb |
| **重复重放** | Event 重放不调用 on_success | ✅ 不会重复归档 |

**关键保证**：
- ✅ Event 重放只修改 Snapshot 状态，不执行副作用
- ✅ 补偿逻辑在启动时检查并修复不一致状态
- ✅ 幂等归档：SurrealDB 用 order_id 作为唯一键，重复插入会失败或更新

---

### 11.3 跨订单操作的事务闭环

**场景**：拆单、合并订单等跨订单操作。

**核心挑战**：
1. 两个订单的状态必须在同一事务内原子更新
2. 生成的 Events 必须关联到各自的订单
3. 重放时每个订单独立重建，不能依赖对方

**解决方案**：一个 Command 生成多个 Event，每个 Event 归属各自订单。

**示例：MergeOrders（合并订单）**

```rust
// edge-server/src/orders/actions/merge_orders.rs

pub struct MergeOrdersAction {
    pub source_order_id: String,
    pub target_order_id: String,
}

#[async_trait]
impl CommandHandler for MergeOrdersAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. 加载两个订单
        let mut source = ctx.load_snapshot(&self.source_order_id)?;
        let mut target = ctx.load_snapshot(&self.target_order_id)?;

        // 2. 验证状态
        if source.status != OrderStatus::Active {
            return Err(OrderError::InvalidOperation(
                format!("Source order {} is not active", self.source_order_id)
            ));
        }

        // 3. 业务逻辑：移动所有 items 和 payments
        let moved_items = source.items.drain(..).collect::<Vec<_>>();
        let moved_payments = source.payments.drain(..).collect::<Vec<_>>();

        target.items.extend(moved_items.clone());
        target.payments.extend(moved_payments.clone());

        // 重算总额
        target.total_amount = target.items.iter()
            .filter(|item| !item.is_voided)
            .map(|item| item.total_price)
            .sum();

        // 源订单标记为 Voided
        source.status = OrderStatus::Voided;
        source.sequence += 1;
        target.sequence += 1;

        // 4. 保存修改
        ctx.save_snapshot(source.clone());
        ctx.save_snapshot(target.clone());

        // 5. 生成两个 Event（各归属各自订单）
        let source_event = OrderEvent {
            event_id: Uuid::new_v4().to_string(),
            event_type: OrderEventType::OrdersMerged,
            order_id: self.source_order_id.clone(), // ← 源订单的 Event
            sequence: source.sequence,
            timestamp: metadata.timestamp,
            operator_id: metadata.operator_id.clone(),
            operator_name: metadata.operator_name.clone(),
            payload: EventPayload::OrdersMerged {
                role: "source".to_string(),
                target_order_id: self.target_order_id.clone(),
                moved_items: moved_items.clone(),
                moved_payments: moved_payments.clone(),
            },
        };

        let target_event = OrderEvent {
            event_id: Uuid::new_v4().to_string(),
            event_type: OrderEventType::OrdersMerged,
            order_id: self.target_order_id.clone(), // ← 目标订单的 Event
            sequence: target.sequence,
            timestamp: metadata.timestamp,
            operator_id: metadata.operator_id.clone(),
            operator_name: metadata.operator_name.clone(),
            payload: EventPayload::OrdersMerged {
                role: "target".to_string(),
                source_order_id: self.source_order_id.clone(),
                moved_items: moved_items.clone(),
                moved_payments: moved_payments.clone(),
            },
        };

        Ok(vec![source_event, target_event])
    }
}
```

**Applier 实现**（独立重放）：

```rust
// edge-server/src/orders/appliers/orders_merged.rs

pub struct OrdersMergedApplier;

impl EventApplier for OrdersMergedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::OrdersMerged { role, moved_items, moved_payments, .. } = &event.payload {
            match role.as_str() {
                "source" => {
                    // 源订单：清空 items/payments，标记 Voided
                    snapshot.items.clear();
                    snapshot.payments.clear();
                    snapshot.status = OrderStatus::Voided;
                }
                "target" => {
                    // 目标订单：添加 items/payments
                    snapshot.items.extend(moved_items.clone());
                    snapshot.payments.extend(moved_payments.clone());

                    // 重算总额
                    snapshot.total_amount = snapshot.items.iter()
                        .filter(|item| !item.is_voided)
                        .map(|item| item.total_price)
                        .sum();
                }
                _ => {}
            }

            snapshot.sequence = event.sequence;
        }
    }
}
```

**关键设计点**：
- ✅ **Event 包含完整数据**：`moved_items` 和 `moved_payments` 在两个 Event 中都有
- ✅ **独立重放**：重放源订单时只看 `role="source"`，重放目标订单时只看 `role="target"`
- ✅ **事务原子性**：两个 Event 在同一 Redb 事务内提交

---

## 12. Order Number 序列号分配机制

### 12.1 需求分析

**问题**：`order_number`（原 receipt_number）需要：
- ✅ 全局唯一，不能重复
- ✅ 持久化索引，断电重启后继续递增
- ✅ 线程安全，支持并发分配
- ✅ 格式可配置（如 `2024012100001`）

**分配时机**：
- `OpenTable` 时分配（订单创建即获得号码）
- 或 `CompleteOrder` 时分配（结账时才分配）
- **建议**：`OpenTable` 时分配，避免结账时序号冲突

---

### 12.2 设计方案

```rust
// edge-server/src/orders/sequence.rs

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use parking_lot::Mutex;

/// 订单号生成器
///
/// **格式**: `{prefix}{date}{sequence}`
/// **示例**: `ORD-20260121-00001`
///
/// **持久化策略**：
/// - 每次分配后立即写入 Redb
/// - 启动时从 Redb 加载最新值
pub struct OrderNumberGenerator {
    /// 当前序列号（原子操作，线程安全）
    current: AtomicU64,
    /// 当前日期（YYYYMMDD）
    current_date: Mutex<String>,
    /// 存储引用（用于持久化）
    storage: Arc<SequenceStorage>,
    /// 前缀（可配置）
    prefix: String,
}

impl OrderNumberGenerator {
    /// 从存储恢复
    pub fn recover(storage: Arc<SequenceStorage>, prefix: String) -> Result<Self, ManagerError> {
        let today = Self::today_str();

        // 从 Redb 加载当前日期的序列号
        let (stored_date, stored_seq) = storage.load_sequence()?;

        let (date, seq) = if stored_date == today {
            // 同一天，继续递增
            (today, stored_seq)
        } else {
            // 新的一天，重置为 0
            (today, 0)
        };

        Ok(Self {
            current: AtomicU64::new(seq),
            current_date: Mutex::new(date),
            storage,
            prefix,
        })
    }

    /// 分配下一个订单号（线程安全）
    ///
    /// **原子性保证**：
    /// 1. 递增序列号
    /// 2. 持久化到 Redb
    /// 3. 返回格式化的订单号
    pub fn next(&self) -> Result<String, ManagerError> {
        let today = Self::today_str();

        // 检查日期是否变化
        {
            let mut current_date = self.current_date.lock();
            if *current_date != today {
                // 新的一天，重置序列号
                self.current.store(0, Ordering::SeqCst);
                *current_date = today.clone();
            }
        }

        // 原子递增
        let seq = self.current.fetch_add(1, Ordering::SeqCst) + 1;

        // 持久化（必须在返回前完成）
        self.storage.save_sequence(&today, seq)?;

        // 格式化
        Ok(format!("{}-{}-{:05}", self.prefix, today, seq))
    }

    /// 获取当前序列号（只读，用于调试）
    pub fn current_value(&self) -> u64 {
        self.current.load(Ordering::SeqCst)
    }

    fn today_str() -> String {
        chrono::Local::now().format("%Y%m%d").to_string()
    }
}

/// 序列号存储（Redb）
pub struct SequenceStorage {
    db: redb::Database,
}

impl SequenceStorage {
    const TABLE: redb::TableDefinition<&str, (String, u64)> =
        redb::TableDefinition::new("sequence");

    /// 加载序列号
    pub fn load_sequence(&self) -> Result<(String, u64), ManagerError> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(Self::TABLE)?;

        match table.get("order_number")? {
            Some(value) => Ok(value.value()),
            None => Ok((String::new(), 0)),
        }
    }

    /// 保存序列号（写入 Redb）
    pub fn save_sequence(&self, date: &str, seq: u64) -> Result<(), ManagerError> {
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(Self::TABLE)?;
            table.insert("order_number", (date.to_string(), seq))?;
        }
        txn.commit()?;
        Ok(())
    }
}
```

---

### 12.3 集成到 CommandContext

```rust
// edge-server/src/orders/traits.rs

pub struct CommandContext<'a> {
    txn: &'a WriteTransaction,
    storage: &'a OrderStorage,
    epoch: String,
    pub state: &'a Arc<ServerState>,
    snapshot_cache: HashMap<String, OrderSnapshot>,

    /// 订单号生成器
    order_number_gen: &'a OrderNumberGenerator,
}

impl<'a> CommandContext<'a> {
    // ... 其他方法

    /// 分配订单号（用于 OpenTable）
    pub fn allocate_order_number(&self) -> Result<String, OrderError> {
        self.order_number_gen
            .next()
            .map_err(|e| OrderError::Storage(e.to_string()))
    }
}
```

---

### 12.4 OpenTableAction 使用

```rust
// edge-server/src/orders/actions/open_table.rs

pub struct OpenTableAction {
    pub table_id: Option<String>,
    pub table_name: Option<String>,
    pub zone_id: Option<String>,
    pub zone_name: Option<String>,
    pub guest_count: i32,
    pub is_retail: bool,
}

#[async_trait]
impl CommandHandler for OpenTableAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. 生成订单 ID
        let order_id = Uuid::new_v4().to_string();

        // 2. 分配订单号（持久化，不会重复）
        let order_number = ctx.allocate_order_number()?;

        // 3. 创建 Snapshot
        let mut snapshot = ctx.create_snapshot(order_id.clone());
        snapshot.order_number = Some(order_number.clone()); // ← 使用分配的号码
        snapshot.table_id = self.table_id.clone();
        snapshot.table_name = self.table_name.clone();
        snapshot.zone_id = self.zone_id.clone();
        snapshot.zone_name = self.zone_name.clone();
        snapshot.guest_count = self.guest_count;
        snapshot.is_retail = self.is_retail;
        snapshot.sequence = 1;

        // 4. 重算 Hash
        snapshot.recalculate();

        // 5. 保存
        ctx.save_snapshot(snapshot.clone());

        // 6. 生成 Event
        Ok(vec![OrderEvent {
            event_id: Uuid::new_v4().to_string(),
            event_type: OrderEventType::TableOpened,
            order_id: order_id.clone(),
            sequence: 1,
            timestamp: metadata.timestamp,
            operator_id: metadata.operator_id.clone(),
            operator_name: metadata.operator_name.clone(),
            payload: EventPayload::TableOpened {
                order_number: order_number.clone(), // ← Event 包含订单号
                table_id: self.table_id.clone(),
                table_name: self.table_name.clone(),
                zone_id: self.zone_id.clone(),
                zone_name: self.zone_name.clone(),
                guest_count: self.guest_count,
                is_retail: self.is_retail,
            },
        }])
    }
}
```

---

### 12.5 断电恢复

```rust
impl OrdersManager {
    /// 启动时初始化
    pub async fn initialize(config: &Config) -> Result<Self, ManagerError> {
        let storage = OrderStorage::open(&config.redb_path)?;
        let sequence_storage = Arc::new(SequenceStorage::new(&config.redb_path)?);

        // 恢复订单号生成器
        let order_number_gen = OrderNumberGenerator::recover(
            sequence_storage,
            config.order_number_prefix.clone(), // 如 "ORD"
        )?;

        tracing::info!(
            "Order number generator recovered: current={}",
            order_number_gen.current_value()
        );

        // ... 其他初始化
    }
}
```

---

### 12.6 订单号格式配置

```toml
# edge-server/config.toml

[orders]
# 订单号前缀
order_number_prefix = "ORD"

# 订单号格式示例:
# - "ORD-20260121-00001"
# - "ORD-20260121-00002"
# - ...
# 每天重置序列号
```

---

### 12.7 并发安全性分析

| 场景 | 处理方式 |
|------|----------|
| **多线程并发分配** | `AtomicU64::fetch_add` 保证原子性 |
| **持久化失败** | 分配后立即写入 Redb，失败则整个命令失败 |
| **断电重启** | 从 Redb 加载最新序列号继续递增 |
| **跨日期** | 检测日期变化，自动重置为 0 |
| **重复分配** | 不可能，每次调用 `next()` 都是原子递增 |

---

## 13. Hash 一致性验证机制

### 12.1 设计原则

**核心理念**：OrderSnapshot 是 OrderItems 的**投影结果**，其状态应完全由 items 确定性计算得出。

**一致性保证**：
- ✅ OrderSnapshot 的 `content_hash` 由 items 集合计算
- ✅ 每次修改 items 后重新计算 hash
- ✅ 重放 Events 后验证 hash 是否一致
- ✅ 检测数据损坏或篡改

**Hash 计算范围**：
```rust
content_hash = SHA256(
    items (sorted by instance_id)
    + payments (sorted by payment_id)
    + status
    + receipt_number
)
```

---

### 12.2 OrderSnapshot 扩展

```rust
// shared/src/order/snapshot.rs

use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSnapshot {
    pub order_id: String,
    pub epoch: String,
    pub sequence: u64,

    // ========== 核心数据 ==========
    pub items: Vec<CartItemSnapshot>,
    pub payments: Vec<PaymentSnapshot>,
    pub status: OrderStatus,

    // ========== 元数据 ==========
    pub table_id: Option<String>,
    pub table_name: Option<String>,
    pub zone_id: Option<String>,
    pub zone_name: Option<String>,
    pub guest_count: i32,
    pub receipt_number: Option<String>,
    pub is_retail: bool,

    // ========== 时间戳 ==========
    pub created_at: i64,
    pub updated_at: i64,
    pub completed_at: Option<i64>,

    // ========== 聚合字段（从 items/payments 计算）==========
    /// 总金额（从 items 计算）
    pub total_amount: f64,
    /// 已支付金额（从 payments 计算）
    pub paid_amount: f64,

    // ========== Hash 验证 ==========
    /// 内容哈希（基于 items + payments + status）
    pub content_hash: String,
}

impl OrderSnapshot {
    /// 创建新订单
    pub fn new(order_id: String, epoch: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        let mut snapshot = Self {
            order_id,
            epoch,
            sequence: 0,
            items: Vec::new(),
            payments: Vec::new(),
            status: OrderStatus::Active,
            table_id: None,
            table_name: None,
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            receipt_number: None,
            is_retail: false,
            created_at: now,
            updated_at: now,
            completed_at: None,
            total_amount: 0.0,
            paid_amount: 0.0,
            content_hash: String::new(), // 初始化后计算
        };

        snapshot.recalculate(); // 计算 hash 和聚合字段
        snapshot
    }

    /// 重新计算聚合字段和 Hash
    ///
    /// **调用时机**：
    /// - Event 应用后
    /// - Handler 修改 snapshot 后
    pub fn recalculate(&mut self) {
        // 1. 计算总金额（从 items）
        self.total_amount = self.items.iter()
            .filter(|item| !item.is_voided)
            .map(|item| item.total_price)
            .sum();

        // 2. 计算已支付金额（从 payments）
        self.paid_amount = self.payments.iter()
            .filter(|p| p.status == PaymentStatus::Confirmed)
            .map(|p| p.amount)
            .sum();

        // 3. 计算 content_hash
        self.content_hash = self.calculate_hash();

        // 4. 更新时间戳
        self.updated_at = chrono::Utc::now().timestamp_millis();
    }

    /// 计算内容哈希
    ///
    /// **确定性要求**：
    /// - items 按 instance_id 排序
    /// - payments 按 payment_id 排序
    /// - 使用稳定的序列化格式
    fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();

        // 1. Items（排序后）
        let mut sorted_items = self.items.clone();
        sorted_items.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));

        for item in &sorted_items {
            // 只哈希关键字段（避免浮点数精度问题）
            hasher.update(item.instance_id.as_bytes());
            hasher.update(item.item_id.as_bytes());
            hasher.update(&item.quantity.to_le_bytes());
            hasher.update(&item.unit_price.serialize()); // Decimal 确定性字节
            hasher.update(&(item.is_voided as u8).to_le_bytes());
        }

        // 2. Payments（排序后）
        let mut sorted_payments = self.payments.clone();
        sorted_payments.sort_by(|a, b| a.payment_id.cmp(&b.payment_id));

        for payment in &sorted_payments {
            hasher.update(payment.payment_id.as_bytes());
            hasher.update(payment.method.as_bytes());
            hasher.update(&payment.amount.serialize()); // 转为分
            hasher.update(&(payment.status as u8).to_le_bytes());
        }

        // 3. Status
        hasher.update(&(self.status as u8).to_le_bytes());

        // 4. Receipt Number
        if let Some(ref receipt) = self.receipt_number {
            hasher.update(receipt.as_bytes());
        }

        // 生成哈希
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    /// 验证 Hash 是否一致
    ///
    /// **用途**：
    /// - 重放后验证
    /// - 检测数据损坏
    pub fn verify_hash(&self) -> bool {
        let computed_hash = {
            let mut temp = self.clone();
            temp.content_hash = String::new(); // 清空后重新计算
            temp.calculate_hash()
        };

        computed_hash == self.content_hash
    }
}
```

---

### 12.3 EventApplier 自动重算

**所有 Applier 必须在修改 snapshot 后调用 `recalculate()`**：

```rust
// edge-server/src/orders/appliers/items_added.rs

pub struct ItemsAddedApplier;

impl EventApplier for ItemsAddedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::ItemsAdded { items } = &event.payload {
            // 1. 修改数据
            snapshot.items.extend(items.clone());
            snapshot.sequence = event.sequence;

            // 2. 重算聚合字段和 Hash
            snapshot.recalculate(); // ← 必须调用
        }
    }
}
```

---

### 12.4 重放验证流程

```rust
impl OrdersManager {
    /// 恢复订单并验证 Hash
    pub fn recover_with_verification(&self, order_id: &str) -> Result<OrderSnapshot, ManagerError> {
        let txn = self.storage.begin_read()?;

        // 1. 加载 Snapshot
        let mut snapshot = self.storage
            .get_snapshot(&txn, order_id)?
            .ok_or_else(|| ManagerError::OrderNotFound(order_id.to_string()))?;

        // 2. 验证 Hash
        if !snapshot.verify_hash() {
            tracing::error!(
                "❌ Hash mismatch for order {}: stored={}, computed={}",
                order_id,
                snapshot.content_hash,
                {
                    let mut temp = snapshot.clone();
                    temp.content_hash = String::new();
                    temp.calculate_hash()
                }
            );
            return Err(ManagerError::Internal(
                format!("Hash verification failed for order {}", order_id)
            ));
        }

        tracing::debug!("✅ Hash verified for order {}", order_id);

        // 3. 应用增量 Events
        let incremental_events = self.storage.get_events_since(&txn, order_id, snapshot.sequence)?;

        for event in &incremental_events {
            let applier: EventAction = event.into();
            applier.apply(&mut snapshot, event);

            // 4. 每次应用后验证 Hash
            if !snapshot.verify_hash() {
                tracing::error!(
                    "❌ Hash mismatch after applying event {}: {}",
                    event.sequence,
                    event.event_id
                );
                return Err(ManagerError::Internal(
                    format!("Hash verification failed after event {}", event.sequence)
                ));
            }
        }

        Ok(snapshot)
    }
}
```

---

### 12.5 Hash 用途总结

| 用途 | 说明 |
|------|------|
| **数据完整性** | 检测 Redb 数据损坏 |
| **重放验证** | 确保 Event 重放结果一致 |
| **跨订单一致性** | 合并/拆单时验证两边数据一致 |
| **调试工具** | 对比两个 Snapshot 是否相同 |
| **审计日志** | 记录每个版本的 Hash |

**示例：合并订单验证**

```rust
impl MergeOrdersAction {
    async fn execute(&self, ctx: &mut CommandContext<'_>, metadata: &CommandMetadata)
        -> Result<Vec<OrderEvent>, OrderError>
    {
        // ... 合并逻辑

        // 验证：源订单 Hash 应该变化
        let source_hash_before = source.content_hash.clone();
        ctx.save_snapshot(source.clone());
        source.recalculate();

        assert_ne!(source.content_hash, source_hash_before, "Source hash should change");

        // 验证：目标订单 Hash 应该变化
        let target_hash_before = target.content_hash.clone();
        ctx.save_snapshot(target.clone());
        target.recalculate();

        assert_ne!(target.content_hash, target_hash_before, "Target hash should change");

        Ok(vec![source_event, target_event])
    }
}
```

---

## 14. Hash 链保护机制（防篡改）

### 14.1 双层 Hash 架构

```
┌─────────────────────────────────────────────────────────────────────┐
│  OrdersManager（活跃订单）                                           │
│  ────────────────────────────────────────────────────────────────── │
│  职责：动态订单处理、提交                                            │
│                                                                      │
│  OrderSnapshot.content_hash = 可靠性 Hash                           │
│    └─ 目的：检测数据损坏、验证重放正确性                             │
│    └─ 计算：SHA256(items + payments + status)                       │
│                                                                      │
│  OrderEvent.hash = 事件链 Hash（订单内）                             │
│    └─ 目的：保护事件顺序，防止插入/删除/篡改                         │
│    └─ 计算：SHA256(prev_hash + 敏感数据)                            │
│    └─ 首个 Event 的 prev_hash = order.order_number                  │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                │ CompleteOrder 后移交
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  OrderService（归档服务）                                          │
│  ────────────────────────────────────────────────────────────────── │
│  职责：订单归档、全局链维护                                          │
│                                                                      │
│  ArchivedOrder.hash = 不可篡改性 Hash（全局链）                      │
│    └─ 目的：审计追踪、防篡改证明、法律效力                           │
│    └─ 计算：SHA256(prev_order_hash + last_event.hash + 敏感数据)    │
│    └─ 首个 Order 的 prev_hash = system_state.genesis_hash           │
│    └─ 按归档顺序形成全局链（非创建顺序）                             │
└─────────────────────────────────────────────────────────────────────┘
```

---

### 14.2 OrderEvent Hash 链（订单内）

**设计**：每个 Event 产生时立即计算 hash，形成订单内的链式保护。

```rust
// shared/src/order/event.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderEvent {
    pub event_id: String,
    pub event_type: OrderEventType,
    pub order_id: String,
    pub sequence: u64,
    pub timestamp: i64,
    pub operator_id: String,
    pub operator_name: String,
    pub payload: EventPayload,

    // ========== Hash 链 ==========
    /// 上一个 Event 的 hash（首个 Event 使用 order_number）
    pub prev_hash: String,
    /// 当前 Event 的 hash
    pub hash: String,
}

impl OrderEvent {
    /// 创建新 Event 并计算 Hash
    pub fn new(
        order_id: String,
        sequence: u64,
        timestamp: i64,
        operator_id: String,
        operator_name: String,
        payload: EventPayload,
        prev_hash: String, // 上一个 Event 的 hash 或 order_number
    ) -> Self {
        let event_id = Uuid::new_v4().to_string();

        let mut event = Self {
            event_id,
            event_type: payload.event_type(),
            order_id,
            sequence,
            timestamp,
            operator_id,
            operator_name,
            payload,
            prev_hash,
            hash: String::new(), // 先占位
        };

        // 计算 hash
        event.hash = event.calculate_hash();
        event
    }

    /// 计算 Event Hash
    ///
    /// hash = SHA256(prev_hash + 敏感数据)
    fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();

        // 1. prev_hash
        hasher.update(self.prev_hash.as_bytes());

        // 2. 敏感数据
        hasher.update(self.event_id.as_bytes());
        hasher.update(self.order_id.as_bytes());
        hasher.update(&self.sequence.to_le_bytes());
        hasher.update(&self.timestamp.to_le_bytes());
        hasher.update(self.operator_id.as_bytes());

        // 3. Payload 关键数据（根据类型）
        match &self.payload {
            EventPayload::ItemsAdded { items } => {
                for item in items {
                    hasher.update(item.instance_id.as_bytes());
                    hasher.update(&item.total_price.serialize());
                }
            }
            EventPayload::PaymentAdded { payment } => {
                hasher.update(payment.payment_id.as_bytes());
                hasher.update(&payment.amount.serialize());
            }
            EventPayload::OrderCompleted { receipt_number } => {
                hasher.update(receipt_number.as_bytes());
            }
            // ... 其他 payload 类型
            _ => {}
        }

        format!("{:x}", hasher.finalize())
    }

    /// 验证 Hash 链完整性
    pub fn verify(&self, expected_prev_hash: &str) -> bool {
        // 1. 验证 prev_hash
        if self.prev_hash != expected_prev_hash {
            return false;
        }

        // 2. 重新计算 hash 验证
        let computed = {
            let mut temp = self.clone();
            temp.hash = String::new();
            temp.calculate_hash()
        };

        computed == self.hash
    }
}
```

**Handler 中使用**：

```rust
// edge-server/src/orders/actions/add_items.rs

impl CommandHandler for AddItemsAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        let mut snapshot = ctx.load_snapshot(&self.order_id)?;

        // ... 业务逻辑

        // 获取 prev_hash（上一个 Event 的 hash 或 order_number）
        let prev_hash = snapshot.last_event_hash
            .clone()
            .unwrap_or_else(|| snapshot.order_number.clone().unwrap_or_default());

        // 创建 Event（自动计算 hash）
        let event = OrderEvent::new(
            self.order_id.clone(),
            snapshot.sequence + 1,
            metadata.timestamp,
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            EventPayload::ItemsAdded { items: processed_items.clone() },
            prev_hash,
        );

        // 更新 snapshot 的 last_event_hash
        snapshot.last_event_hash = Some(event.hash.clone());
        snapshot.sequence += 1;
        snapshot.items.extend(processed_items);
        snapshot.recalculate();

        ctx.save_snapshot(snapshot);

        Ok(vec![event])
    }
}
```

---

### 14.3 OrderSnapshot 扩展

```rust
// shared/src/order/snapshot.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSnapshot {
    // ... 现有字段

    // ========== Hash 链支持 ==========
    /// 最后一个 Event 的 hash（用于链接下一个 Event）
    pub last_event_hash: Option<String>,
}
```

---

### 14.4 ArchivedOrder Hash 链（全局）

**设计**：订单归档时计算不可篡改 hash，按归档顺序形成全局链。

```rust
// edge-server/src/orders/service.rs

use sha2::{Sha256, Digest};

/// 归档订单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivedOrder {
    // ========== 订单数据 ==========
    pub order_id: String,
    pub order_number: String,
    pub items: Vec<CartItemSnapshot>,
    pub payments: Vec<PaymentSnapshot>,
    pub total_amount: f64,
    pub paid_amount: f64,
    pub status: OrderStatus,
    pub created_at: i64,
    pub completed_at: i64,

    // ========== 操作记录 ==========
    pub operator_id: String,
    pub operator_name: String,

    // ========== Hash 链 ==========
    /// 上一个归档订单的 hash（首个使用 genesis_hash）
    pub prev_order_hash: String,
    /// 该订单最后一个 Event 的 hash
    pub last_event_hash: String,
    /// 当前订单的 hash
    pub hash: String,

    // ========== 归档元数据 ==========
    pub archived_at: i64,
    pub archive_sequence: u64, // 全局归档序号
}

impl ArchivedOrder {
    /// 从 OrderSnapshot 创建归档订单
    pub fn from_snapshot(
        snapshot: OrderSnapshot,
        prev_order_hash: String,
        archive_sequence: u64,
    ) -> Self {
        let mut archived = Self {
            order_id: snapshot.order_id,
            order_number: snapshot.order_number.unwrap_or_default(),
            items: snapshot.items,
            payments: snapshot.payments,
            total_amount: snapshot.total_amount,
            paid_amount: snapshot.paid_amount,
            status: snapshot.status,
            created_at: snapshot.created_at,
            completed_at: snapshot.completed_at.unwrap_or(0),
            operator_id: String::new(), // 从最后一个 Event 获取
            operator_name: String::new(),
            prev_order_hash,
            last_event_hash: snapshot.last_event_hash.unwrap_or_default(),
            hash: String::new(), // 先占位
            archived_at: chrono::Utc::now().timestamp_millis(),
            archive_sequence,
        };

        // 计算 hash
        archived.hash = archived.calculate_hash();
        archived
    }

    /// 计算订单 Hash
    ///
    /// hash = SHA256(prev_order_hash + last_event_hash + 敏感数据)
    fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();

        // 1. prev_order_hash（链接上一个订单）
        hasher.update(self.prev_order_hash.as_bytes());

        // 2. last_event_hash（链接订单内的事件链）
        hasher.update(self.last_event_hash.as_bytes());

        // 3. 敏感数据
        hasher.update(self.order_id.as_bytes());
        hasher.update(self.order_number.as_bytes());
        hasher.update(&self.total_amount.serialize());
        hasher.update(&self.paid_amount.serialize());
        hasher.update(&self.created_at.to_le_bytes());
        hasher.update(&self.completed_at.to_le_bytes());
        hasher.update(&self.archive_sequence.to_le_bytes());

        // 4. Items 摘要
        for item in &self.items {
            hasher.update(item.instance_id.as_bytes());
            hasher.update(&item.total_price.serialize());
        }

        // 5. Payments 摘要
        for payment in &self.payments {
            hasher.update(payment.payment_id.as_bytes());
            hasher.update(&payment.amount.serialize());
        }

        format!("{:x}", hasher.finalize())
    }

    /// 验证 Hash
    pub fn verify(&self, expected_prev_hash: &str) -> bool {
        if self.prev_order_hash != expected_prev_hash {
            return false;
        }

        let computed = {
            let mut temp = self.clone();
            temp.hash = String::new();
            temp.calculate_hash()
        };

        computed == self.hash
    }
}
```

---

### 14.5 OrderService 实现

```rust
// edge-server/src/orders/service.rs

use shared::models::system_state::{SystemState, SystemStateUpdate};

pub struct OrderService {
    db: Surreal<Db>,
    config: Arc<Config>,
    /// 内存缓存：当前归档链尾 hash
    last_order_hash: Mutex<String>,
    /// 内存缓存：当前归档序号
    archive_sequence: AtomicU64,
}

impl OrderService {
    /// 初始化（从 SystemState 加载状态）
    pub async fn initialize(db: Surreal<Db>, config: Arc<Config>) -> Result<Self, Error> {
        // 确保 genesis_hash 存在
        let genesis_hash = Self::ensure_genesis_static(&db, &config).await?;
        
        // 从 SystemState 加载最后状态
        let state: Option<SystemState> = db
            .select("system_state:main")
            .await?;
        
        let last_hash = state
            .as_ref()
            .and_then(|s| s.last_order_hash.clone())
            .unwrap_or(genesis_hash);
        
        // 查询最后一个归档订单获取序号
        let last_archived: Option<ArchivedOrder> = db
            .query("SELECT * FROM archived_orders ORDER BY archive_sequence DESC LIMIT 1")
            .await?
            .take(0)?;
        
        let last_seq = last_archived.map(|o| o.archive_sequence).unwrap_or(0);
        
        Ok(Self {
            db,
            config,
            last_order_hash: Mutex::new(last_hash),
            archive_sequence: AtomicU64::new(last_seq),
        })
    }
    
    /// 确保 genesis_hash 存在
    async fn ensure_genesis_static(db: &Surreal<Db>, config: &Config) -> Result<String, Error> {
        let state: Option<SystemState> = db.select("system_state:main").await?;
        
        if let Some(hash) = state.and_then(|s| s.genesis_hash) {
            return Ok(hash);
        }
        
        // 生成创世哈希：店铺ID + 激活时间戳
        let genesis_input = format!(
            "genesis:{}:{}",
            config.store_id.as_deref().unwrap_or("default"),
            chrono::Utc::now().timestamp()
        );
        let genesis_hash = sha256_hex(&genesis_input);
        
        // 初始化 SystemState
        db.query(
            "UPDATE system_state:main SET genesis_hash = $hash, order_count = 0"
        ).bind(("hash", &genesis_hash)).await?;
        
        tracing::info!("🌱 Genesis hash initialized: {}...", &genesis_hash[..16]);
        Ok(genesis_hash)
    }

    /// 归档订单（原子操作：归档 + 更新 SystemState）
    pub async fn archive(&self, snapshot: OrderSnapshot) -> Result<ArchivedOrder, Error> {
        // 1. 获取 prev_hash 和序号（加锁保证顺序）
        let (prev_hash, sequence) = {
            let last_hash = self.last_order_hash.lock().await;
            let seq = self.archive_sequence.fetch_add(1, Ordering::SeqCst) + 1;
            (last_hash.clone(), seq)
        };

        // 2. 创建归档订单（计算 hash）
        let archived = ArchivedOrder::from_snapshot(snapshot, prev_hash, sequence);
        let order_id = archived.id.clone();
        let order_hash = archived.hash.clone();

        // 3. 事务：保存归档订单 + 更新 SystemState
        self.db.query(
            "BEGIN TRANSACTION;
             CREATE archived_orders CONTENT $order;
             UPDATE system_state:main SET 
                 last_order = $order_id,
                 last_order_hash = $hash,
                 order_count += 1,
                 updated_at = $time;
             COMMIT TRANSACTION;"
        )
        .bind(("order", &archived))
        .bind(("order_id", &order_id))
        .bind(("hash", &order_hash))
        .bind(("time", chrono::Utc::now().to_rfc3339()))
        .await?;

        // 4. 更新内存缓存
        {
            let mut last_hash = self.last_order_hash.lock().await;
            *last_hash = order_hash.clone();
        }

        tracing::info!(
            "📦 Order {} archived: seq={}, hash={}...",
            archived.order_id,
            sequence,
            &order_hash[..16]
        );

        Ok(archived)
    }

    /// 验证归档链完整性
    pub async fn verify_chain(&self) -> Result<VerifyResult, Error> {
        // 从 SystemState 获取 genesis_hash
        let state: SystemState = self.db
            .select("system_state:main")
            .await?
            .ok_or_else(|| Error::NotInitialized)?;
        
        let genesis_hash = state.genesis_hash
            .ok_or_else(|| Error::NotInitialized)?;
        
        let orders: Vec<ArchivedOrder> = self.db
            .query("SELECT * FROM archived_orders ORDER BY archive_sequence ASC")
            .await?
            .take(0)?;

        let mut expected_prev = genesis_hash;
        let mut verified_count = 0;

        for order in &orders {
            if !order.verify(&expected_prev) {
                return Ok(VerifyResult {
                    valid: false,
                    verified_count,
                    total_count: orders.len(),
                    error: Some(format!(
                        "Chain broken at order {}: expected prev={}, got={}",
                        order.order_id, expected_prev, order.prev_order_hash
                    )),
                });
            }
            expected_prev = order.hash.clone();
            verified_count += 1;
        }

        // 验证 SystemState 的 last_order_hash 是否一致
        if let Some(last) = orders.last() {
            if state.last_order_hash.as_ref() != Some(&last.hash) {
                return Ok(VerifyResult {
                    valid: false,
                    verified_count,
                    total_count: orders.len(),
                    error: Some("SystemState.last_order_hash mismatch".to_string()),
                });
            }
        }

        tracing::info!("✅ Archive chain verified: {} orders", orders.len());
        Ok(VerifyResult {
            valid: true,
            verified_count,
            total_count: orders.len(),
            error: None,
        })
    }
    
    // ========== 远程同步（预留，暂不实现） ==========
    // 
    // SystemState 已预留以下字段用于未来税务级同步：
    // - synced_up_to: 已同步到远程的最后订单
    // - synced_up_to_hash: 已同步订单的 hash（验证完整性）
    // - last_sync_time: 最后同步时间
    //
    // 未来实现时，需要：
    // 1. get_pending_sync_orders() - 获取待同步订单
    // 2. mark_synced() - 标记同步完成
    // 3. verify_remote_sync() - 验证远程同步一致性
}

#[derive(Debug)]
pub struct VerifyResult {
    pub valid: bool,
    pub verified_count: usize,
    pub total_count: usize,
    pub error: Option<String>,
}
```

---

### 14.6 SystemState 集成（税务级审计）

**现有模型**：`shared/src/models/system_state.rs`

```rust
/// System state entity (哈希链状态缓存)
/// 
/// 职责：
/// 1. 本地归档链追踪（genesis_hash → last_order_hash）
/// 2. 远程同步状态（synced_up_to_hash）- 税务级审计需求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub id: Option<Thing>,
    
    // ========== 归档链状态 ==========
    /// 创世哈希（首个归档订单的 prev_hash）
    pub genesis_hash: Option<String>,
    /// 最后归档订单引用
    pub last_order: Option<Thing>,
    /// 最后归档订单的 hash（链尾）
    pub last_order_hash: Option<String>,
    
    // ========== 远程同步状态（税务审计） ==========
    /// 已同步到远程的最后订单
    pub synced_up_to: Option<Thing>,
    /// 已同步订单的 hash（验证同步完整性）
    pub synced_up_to_hash: Option<String>,
    pub last_sync_time: Option<String>,
    
    // ========== 统计 ==========
    pub order_count: i32,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}
```

**初始化流程**：

```rust
// 首次启动时初始化 genesis_hash
impl OrderService {
    pub async fn ensure_genesis(&self) -> Result<String, Error> {
        let state = self.db.select::<Option<SystemState>>("system_state:main").await?;
        
        match state.and_then(|s| s.genesis_hash) {
            Some(hash) => Ok(hash),
            None => {
                // 生成创世哈希：店铺ID + 激活时间
                let genesis = format!(
                    "{}:{}",
                    self.config.store_id,
                    chrono::Utc::now().timestamp()
                );
                let genesis_hash = sha256_hex(&genesis);
                
                // 初始化 SystemState
                self.db.query(
                    "UPDATE system_state:main SET genesis_hash = $hash, order_count = 0"
                ).bind(("hash", &genesis_hash)).await?;
                
                Ok(genesis_hash)
            }
        }
    }
}
```

**远程同步（预留）**：

> ⚠️ 现阶段不实现同步逻辑，仅预留字段。

```rust
// SystemState 预留字段（供未来税务级同步使用）：
//
// synced_up_to: Option<Thing>      - 已同步到远程的最后订单
// synced_up_to_hash: Option<String> - 已同步订单的 hash
// last_sync_time: Option<String>    - 最后同步时间
//
// 未来实现需求：
// - 所有归档订单必须上传到中央服务器
// - 使用 hash 链验证数据完整性
// - 断点续传支持
```

---

### 14.7 Hash 链验证流程

```
┌─────────────────────────────────────────────────────────────────────┐
│  订单内 Event 链验证                                                 │
│                                                                      │
│  order_number ──▶ E1.prev ──▶ E1.hash ──▶ E2.prev ──▶ E2.hash ──▶  │
│                                                                      │
│  验证方式：                                                          │
│  1. E1.prev_hash == order_number ✓                                  │
│  2. E1.hash == SHA256(E1.prev + E1.data) ✓                         │
│  3. E2.prev_hash == E1.hash ✓                                       │
│  4. E2.hash == SHA256(E2.prev + E2.data) ✓                         │
│  ...                                                                 │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│  全局 Order 链验证                                                   │
│                                                                      │
│  genesis ──▶ O1.prev ──▶ O1.hash ──▶ O2.prev ──▶ O2.hash ──▶       │
│                  │                        │                          │
│                  └── O1.last_event_hash   └── O2.last_event_hash    │
│                           │                        │                 │
│                           ▼                        ▼                 │
│                     (订单内链)                (订单内链)             │
│                                                                      │
│  验证方式：                                                          │
│  1. O1.prev_order_hash == genesis_hash ✓                            │
│  2. O1.hash == SHA256(O1.prev + O1.last_event + O1.data) ✓         │
│  3. O2.prev_order_hash == O1.hash ✓                                 │
│  4. O2.hash == SHA256(O2.prev + O2.last_event + O2.data) ✓         │
│  ...                                                                 │
└─────────────────────────────────────────────────────────────────────┘
```

---

### 14.8 三种 Hash 对比

| Hash 类型 | 位置 | 目的 | 链式 | 计算时机 |
|----------|------|------|------|----------|
| **content_hash** | OrderSnapshot | 可靠性（数据完整性） | ❌ | 每次修改后 |
| **event.hash** | OrderEvent | 事件链保护（订单内） | ✅ | Event 产生时 |
| **order.hash** | ArchivedOrder | 不可篡改性（审计） | ✅ | 归档时 |

---

## 15. 添加新命令的流程

**示例：添加 `TransferItems` 命令**

### Step 1: 定义 Command Payload
```rust
// shared/src/order/command.rs
pub enum OrderCommandPayload {
    // ... existing variants

    /// Transfer items between orders
    TransferItems {
        source_order_id: String,
        target_order_id: String,
        items: Vec<TransferItemInput>,
    },
}
```

### Step 2: 定义 Event Payload
```rust
// shared/src/order/event.rs
pub enum EventPayload {
    // ... existing variants

    ItemsTransferred {
        source_order_id: String,
        target_order_id: String,
        transferred_items: Vec<CartItemSnapshot>,
    },
}
```

### Step 3: 实现 Action
```rust
// edge-server/src/orders/actions/transfer_items.rs
pub struct TransferItemsAction {
    pub source_order_id: String,
    pub target_order_id: String,
    pub items: Vec<TransferItemInput>,
}

#[async_trait]
impl CommandHandler for TransferItemsAction {
    async fn execute(...) -> Result<OrderEvent, OrderError> {
        // 业务逻辑
    }
}
```

### Step 4: 实现 Applier
```rust
// edge-server/src/orders/appliers/items_transferred.rs
pub struct ItemsTransferredApplier;

impl EventApplier for ItemsTransferredApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        // 数据应用
    }
}
```

### Step 5: 注册到 enum
```rust
// actions/mod.rs
pub enum CommandAction {
    // ... existing variants
    TransferItems(TransferItemsAction),
}

impl From<OrderCommand> for CommandAction {
    fn from(cmd: OrderCommand) -> Self {
        match cmd.payload {
            // ... existing arms
            OrderCommandPayload::TransferItems { source_order_id, target_order_id, items } => {
                CommandAction::TransferItems(TransferItemsAction {
                    source_order_id,
                    target_order_id,
                    items,
                })
            }
        }
    }
}

// appliers/mod.rs
pub enum EventAction {
    // ... existing variants
    ItemsTransferred(ItemsTransferredApplier),
}

impl From<&OrderEvent> for EventAction {
    fn from(event: &OrderEvent) -> Self {
        match &event.payload {
            // ... existing arms
            EventPayload::ItemsTransferred { .. } => {
                EventAction::ItemsTransferred(ItemsTransferredApplier)
            }
        }
    }
}
```

### Step 6: 编写测试
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_transfer_items() { ... }
}
```

**总结**：
- ✅ 只需新增 2 个文件 + 修改 2 个 enum
- ✅ 不修改 OrdersManager
- ✅ 不影响其他命令

---

## 16. 迁移检查清单

### 开发阶段
- [ ] 所有 14 个 Action 实现完成
- [ ] 所有 14 个 Applier 实现完成
- [ ] CommandAction enum 完整
- [ ] EventAction enum 完整
- [ ] OrdersManager 重构完成
- [ ] 旧代码删除（handle_xxx 方法）
- [ ] reducer.rs 清理
- [ ] OrderEvent Hash 链实现
- [ ] OrderSnapshot.last_event_hash 字段添加
- [ ] OrderService 实现
- [ ] ArchivedOrder Hash 链实现
- [ ] OrderNumberGenerator 实现
- [ ] SystemState 集成（genesis_hash、last_order_hash、同步状态）

### 测试阶段
- [ ] 所有单元测试通过
- [ ] 集成测试通过
- [ ] 重放测试通过（rebuild_snapshot）
- [ ] 并发测试通过
- [ ] 幂等性测试通过
- [ ] 性能基准测试（无退化）
- [ ] Event Hash 链验证测试
- [ ] Order Hash 链验证测试
- [ ] 归档补偿逻辑测试

### 文档阶段
- [ ] 架构文档更新
- [ ] API 文档更新
- [ ] 迁移指南编写
- [ ] 示例代码更新

### 部署准备
- [ ] Code review 完成
- [ ] Clippy warnings 清零
- [ ] `cargo fmt` 检查通过
- [ ] Release notes 编写
- [ ] SystemState 初始化确认（genesis_hash 自动生成）

---

## 最终总结

### 核心设计决策

| 决策点 | 方案 | 理由 |
|--------|------|------|
| **Command 分发** | enum_dispatch | 零成本抽象，消除 match |
| **Event 分发** | enum_dispatch | 重放时无 match |
| **Handler 权限** | CommandContext | 统一访问 State、Storage、Snapshot、价格规则 |
| **多订单操作** | `Vec<OrderEvent>` 输出 | 支持拆单、合并等跨订单场景 |
| **状态一致性** | SHA256 content_hash | 检测数据损坏，验证重放正确性 |
| **订单号分配** | 原子序列号 + Redb 持久化 | 不重复，断电恢复 |
| **归档卸载** | on_success 异步归档 | SurrealDB 长期存储，Redb 保持轻量 |
| **Event Hash 链** | prev_hash → hash 链式 | 订单内事件防篡改，首个用 order_number |
| **Order Hash 链** | 归档时计算，全局链 | 审计追踪，首个用 genesis_hash |
| **职责分离** | OrdersManager / OrderService | 动态订单 vs 归档不可篡改 |

---

### 数据流总览

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Command 到达                                 │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  OrdersManager::execute_command()                                    │
│    1. 幂等性检查 (command_id)                                        │
│    2. OrderCommand → CommandAction (From trait)                      │
│    3. 创建 CommandContext                                            │
│    4. action.execute(&mut ctx, &metadata)                            │
│       ├─ ctx.load_snapshot() / ctx.create_snapshot()                │
│       ├─ ctx.state.price_rule_engine (访问服务)                      │
│       ├─ ctx.allocate_order_number() (分配订单号)                    │
│       ├─ snapshot.recalculate() (更新 Hash)                         │
│       └─ ctx.save_snapshot()                                        │
│    5. 持久化 Events (按 order_id 分组)                               │
│    6. 持久化 Snapshots (验证 Hash)                                   │
│    7. 提交 Redb 事务                                                 │
│    8. 广播 Events                                                    │
│    9. 异步执行 on_success (副作用)                                   │
└─────────────────────────────────────────────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
┌───────────────┐   ┌───────────────────┐   ┌───────────────────┐
│  MessageBus   │   │  on_success()     │   │  Redb 持久化       │
│  广播 Events  │   │  - 厨房打印       │   │  - Events          │
│               │   │  - 归档 SurrealDB │   │  - Snapshots       │
│               │   │  - 卸载 Redb      │   │  - Sequence Index  │
└───────────────┘   └───────────────────┘   └───────────────────┘
```

---

### 断电重启恢复流程

```
┌─────────────────────────────────────────────────────────────────────┐
│  OrdersManager::recover_on_startup()                                 │
│    1. 恢复 OrderNumberGenerator (从 Redb 加载序列号)                 │
│    2. 加载所有活跃 Snapshots                                         │
│    3. 对每个订单:                                                    │
│       a. 验证 content_hash                                          │
│       b. 加载增量 Events (sequence > snapshot.sequence)              │
│       c. 应用 Events (EventApplier，无业务逻辑)                      │
│       d. 重新计算 Hash，验证一致性                                   │
│       e. 更新 Snapshot                                               │
│    4. 完成恢复，接受新命令                                           │
└─────────────────────────────────────────────────────────────────────┘
```

---

### 迁移收益对比

| 维度 | 现状 | 迁移后 |
|------|------|--------|
| **代码组织** | 1200+ 行 manager.rs | 32 个独立文件，每个 < 100 行 |
| **添加新命令** | 修改 3+ 处 match | 只需新增 2 个文件 |
| **测试覆盖** | 难以 Mock | 每个 Handler/Applier 独立测试 |
| **跨订单操作** | 复杂嵌套逻辑 | 统一的 `Vec<OrderEvent>` 输出 |
| **数据一致性** | 无验证 | SHA256 Hash 保证 |
| **订单号** | 可能重复 | 原子分配 + 持久化 |
| **重放可靠性** | 依赖中间状态 | 纯函数 Applier，100% 确定性 |

---

### 迁移成本

- **开发时间**：约 10-14 工作日
- **风险等级**：中等（核心模块重构）
- **回滚策略**：使用 git feature branch，可随时回滚

---

### 实施建议

1. **Phase 1**：基础设施
   - traits.rs (CommandHandler, EventApplier, CommandContext)
   - sequence.rs (OrderNumberGenerator)
   - Hash 验证机制

2. **Phase 2**：核心命令迁移
   - OpenTableAction (含订单号分配)
   - AddItemsAction (含价格规则)
   - CompleteOrderAction (含归档)

3. **Phase 3**：其他命令迁移
   - ModifyItem, RemoveItem
   - AddPayment, CancelPayment
   - MoveOrder, MergeOrders

4. **Phase 4**：测试与验证
   - 单元测试
   - 重放测试
   - 性能基准

---

**审批签字**：
- [ ] 架构师审批
- [ ] 技术负责人审批
- [ ] QA 负责人审批

**预计开始日期**：2026-01-22
**预计完成日期**：2026-02-05
