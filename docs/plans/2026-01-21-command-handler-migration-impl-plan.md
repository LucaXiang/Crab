# CommandHandler 架构迁移实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将订单处理从 match-based 架构迁移到 Strategy Pattern (trait-based) 架构，实现可维护、可扩展的代码结构。

**Architecture:** 使用 `enum_dispatch` 实现零成本抽象的策略模式。每个命令拆分为独立的 Action 文件实现 `CommandHandler` trait，每个事件拆分为独立的 Applier 文件实现 `EventApplier` trait。通过 `From` trait 集中转换，消除分散的 match 语句。

**Tech Stack:** Rust, enum_dispatch, async_trait, thiserror, redb

---

## Phase 1: 基础设施准备

### Task 1.1: 添加 enum_dispatch 依赖

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `edge-server/Cargo.toml`

**Step 1: 添加依赖到 workspace Cargo.toml**

在 `[workspace.dependencies]` 部分添加：
```toml
enum_dispatch = "0.3"
```

**Step 2: 在 edge-server 中引用依赖**

在 `edge-server/Cargo.toml` 的 `[dependencies]` 部分添加：
```toml
enum_dispatch.workspace = true
```

**Step 3: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译通过，无错误

**Step 4: Commit**

```bash
git add Cargo.toml edge-server/Cargo.toml
git commit -m "deps: add enum_dispatch for strategy pattern migration"
```

---

### Task 1.2: 创建 traits.rs 定义核心 Trait

**Files:**
- Create: `edge-server/src/orders/traits.rs`
- Modify: `edge-server/src/orders/mod.rs`

**Step 1: 创建 traits.rs**

```rust
//! Core traits for the CommandHandler architecture
//!
//! This module defines the traits that enable the Strategy Pattern for order command processing:
//! - `CommandHandler`: Executes commands and generates events
//! - `EventApplier`: Applies events to snapshots (pure function, no side effects)

use crate::orders::storage::OrderStorage;
use async_trait::async_trait;
use redb::WriteTransaction;
use shared::order::{OrderEvent, OrderSnapshot};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during order operations
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

/// Command metadata extracted from OrderCommand
#[derive(Debug, Clone)]
pub struct CommandMetadata {
    pub command_id: String,
    pub operator_id: String,
    pub operator_name: String,
    pub timestamp: i64,
}

/// Command execution context
///
/// Provides:
/// - Access to the write transaction
/// - Snapshot cache to avoid redundant reads
/// - Order creation utilities
pub struct CommandContext<'a> {
    txn: &'a WriteTransaction,
    storage: &'a OrderStorage,
    snapshot_cache: HashMap<String, OrderSnapshot>,
    next_sequence: u64,
}

impl<'a> CommandContext<'a> {
    pub fn new(txn: &'a WriteTransaction, storage: &'a OrderStorage, current_sequence: u64) -> Self {
        Self {
            txn,
            storage,
            snapshot_cache: HashMap::new(),
            next_sequence: current_sequence + 1,
        }
    }

    /// Load a snapshot, using cache if available
    pub fn load_snapshot(&mut self, order_id: &str) -> Result<OrderSnapshot, OrderError> {
        if let Some(snapshot) = self.snapshot_cache.get(order_id) {
            return Ok(snapshot.clone());
        }

        let snapshot = self
            .storage
            .get_snapshot_txn(self.txn, order_id)
            .map_err(|e| OrderError::Storage(e.to_string()))?
            .ok_or_else(|| OrderError::OrderNotFound(order_id.to_string()))?;

        self.snapshot_cache.insert(order_id.to_string(), snapshot.clone());
        Ok(snapshot)
    }

    /// Create a new snapshot and add to cache
    pub fn create_snapshot(&mut self, order_id: String) -> OrderSnapshot {
        let snapshot = OrderSnapshot::new(order_id.clone());
        self.snapshot_cache.insert(order_id, snapshot.clone());
        snapshot
    }

    /// Save a snapshot to the cache (actual persistence happens in manager)
    pub fn save_snapshot(&mut self, snapshot: OrderSnapshot) {
        self.snapshot_cache.insert(snapshot.order_id.clone(), snapshot);
    }

    /// Get all modified snapshots for persistence
    pub fn modified_snapshots(&self) -> impl Iterator<Item = &OrderSnapshot> {
        self.snapshot_cache.values()
    }

    /// Get the write transaction
    pub fn txn(&self) -> &WriteTransaction {
        self.txn
    }

    /// Get the storage
    pub fn storage(&self) -> &OrderStorage {
        self.storage
    }

    /// Allocate a new sequence number
    pub fn next_sequence(&mut self) -> u64 {
        let seq = self.next_sequence;
        self.next_sequence += 1;
        seq
    }
}

/// Command handler trait
///
/// Implementations execute business logic and generate events.
/// This trait is called when processing NEW commands, not when replaying events.
#[async_trait]
pub trait CommandHandler: Send + Sync {
    /// Execute the command and return generated events
    ///
    /// # Arguments
    /// - `ctx`: Execution context with transaction and snapshot cache
    /// - `metadata`: Command metadata (operator, timestamp, etc.)
    ///
    /// # Returns
    /// - `Ok(Vec<OrderEvent>)`: Events generated by this command
    /// - `Err(OrderError)`: If the command cannot be executed
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError>;
}

/// Event applier trait
///
/// Implementations apply event data to snapshots.
/// This is a PURE function - no business logic, no side effects, no I/O.
/// Used for both command execution and event replay.
pub trait EventApplier: Send + Sync {
    /// Apply the event to the snapshot
    ///
    /// # Guarantees
    /// - Pure function: same input always produces same output
    /// - No I/O operations
    /// - No ID generation (IDs come from the event)
    /// - No business logic validation
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent);
}
```

**Step 2: 添加到 mod.rs**

在 `edge-server/src/orders/mod.rs` 中添加：
```rust
pub mod traits;
```

**Step 3: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译通过

**Step 4: Commit**

```bash
git add edge-server/src/orders/traits.rs edge-server/src/orders/mod.rs
git commit -m "feat(orders): add CommandHandler and EventApplier traits"
```

---

### Task 1.3: 创建 actions 和 appliers 目录结构

**Files:**
- Create: `edge-server/src/orders/actions/mod.rs`
- Create: `edge-server/src/orders/appliers/mod.rs`
- Modify: `edge-server/src/orders/mod.rs`

**Step 1: 创建 actions/mod.rs 骨架**

```rust
//! Command action implementations
//!
//! Each action implements the `CommandHandler` trait and handles
//! one specific command type.

use enum_dispatch::enum_dispatch;

use crate::orders::traits::CommandHandler;
use shared::order::{OrderCommand, OrderCommandPayload};

// Action modules will be added as we implement them
// mod open_table;
// mod add_items;
// ...

// Re-exports will be added as we implement them
// pub use open_table::OpenTableAction;
// pub use add_items::AddItemsAction;
// ...

/// CommandAction enum - dispatches to concrete action implementations
///
/// Uses enum_dispatch for zero-cost static dispatch.
#[enum_dispatch(CommandHandler)]
pub enum CommandAction {
    // Variants will be added as we implement them
    // OpenTable(OpenTableAction),
    // AddItems(AddItemsAction),
    // ...
    /// Placeholder variant (remove when first action is added)
    #[allow(dead_code)]
    Placeholder(PlaceholderAction),
}

/// Placeholder action (remove when first action is added)
pub struct PlaceholderAction;

#[async_trait::async_trait]
impl CommandHandler for PlaceholderAction {
    async fn execute(
        &self,
        _ctx: &mut crate::orders::traits::CommandContext<'_>,
        _metadata: &crate::orders::traits::CommandMetadata,
    ) -> Result<Vec<shared::order::OrderEvent>, crate::orders::traits::OrderError> {
        unreachable!("PlaceholderAction should never be executed")
    }
}

/// Convert OrderCommand to CommandAction
///
/// This is the ONLY place with a match on OrderCommandPayload.
impl From<&OrderCommand> for CommandAction {
    fn from(_cmd: &OrderCommand) -> Self {
        // Implementation will be added as we implement actions
        todo!("Implement command conversion")
    }
}
```

**Step 2: 创建 appliers/mod.rs 骨架**

```rust
//! Event applier implementations
//!
//! Each applier implements the `EventApplier` trait and handles
//! one specific event type. Appliers are PURE functions.

use enum_dispatch::enum_dispatch;

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

// Applier modules will be added as we implement them
// mod table_opened;
// mod items_added;
// ...

// Re-exports will be added as we implement them
// pub use table_opened::TableOpenedApplier;
// pub use items_added::ItemsAddedApplier;
// ...

/// EventAction enum - dispatches to concrete applier implementations
///
/// Uses enum_dispatch for zero-cost static dispatch.
#[enum_dispatch(EventApplier)]
pub enum EventAction {
    // Variants will be added as we implement them
    // TableOpened(TableOpenedApplier),
    // ItemsAdded(ItemsAddedApplier),
    // ...
    /// Placeholder variant (remove when first applier is added)
    #[allow(dead_code)]
    Placeholder(PlaceholderApplier),
}

/// Placeholder applier (remove when first applier is added)
pub struct PlaceholderApplier;

impl EventApplier for PlaceholderApplier {
    fn apply(&self, _snapshot: &mut OrderSnapshot, _event: &OrderEvent) {
        unreachable!("PlaceholderApplier should never be called")
    }
}

/// Convert OrderEvent reference to EventAction
///
/// This is the ONLY place with a match on EventPayload.
impl From<&OrderEvent> for EventAction {
    fn from(_event: &OrderEvent) -> Self {
        // Implementation will be added as we implement appliers
        todo!("Implement event conversion")
    }
}
```

**Step 3: 更新 mod.rs**

在 `edge-server/src/orders/mod.rs` 中添加：
```rust
pub mod actions;
pub mod appliers;
```

**Step 4: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译通过（placeholder 代码不会被执行）

**Step 5: Commit**

```bash
git add edge-server/src/orders/actions edge-server/src/orders/appliers edge-server/src/orders/mod.rs
git commit -m "feat(orders): create actions and appliers module structure"
```

---

## Phase 2: 核心命令实现（高优先级）

### Task 2.1: 实现 OpenTableAction 和 TableOpenedApplier

**Files:**
- Create: `edge-server/src/orders/actions/open_table.rs`
- Create: `edge-server/src/orders/appliers/table_opened.rs`
- Modify: `edge-server/src/orders/actions/mod.rs`
- Modify: `edge-server/src/orders/appliers/mod.rs`

**Step 1: 创建 open_table.rs**

```rust
//! OpenTable command handler
//!
//! Creates a new order with table information.

use async_trait::async_trait;
use uuid::Uuid;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// OpenTable action
#[derive(Debug, Clone)]
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
        // 1. Generate new order ID
        let order_id = Uuid::new_v4().to_string();

        // 2. Create snapshot
        let mut snapshot = ctx.create_snapshot(order_id.clone());
        snapshot.table_id = self.table_id.clone();
        snapshot.table_name = self.table_name.clone();
        snapshot.zone_id = self.zone_id.clone();
        snapshot.zone_name = self.zone_name.clone();
        snapshot.guest_count = self.guest_count;
        snapshot.is_retail = self.is_retail;
        snapshot.status = OrderStatus::Active;
        snapshot.start_time = metadata.timestamp;
        snapshot.created_at = metadata.timestamp;
        snapshot.updated_at = metadata.timestamp;
        snapshot.last_sequence = ctx.next_sequence();

        // 3. Update checksum
        snapshot.update_checksum();

        // 4. Save to context
        ctx.save_snapshot(snapshot);

        // 5. Create event
        let event = OrderEvent::new(
            ctx.next_sequence() - 1, // Use the sequence we allocated
            order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.timestamp,
            OrderEventType::TableOpened,
            EventPayload::TableOpened {
                table_id: self.table_id.clone(),
                table_name: self.table_name.clone(),
                zone_id: self.zone_id.clone(),
                zone_name: self.zone_name.clone(),
                guest_count: self.guest_count,
                is_retail: self.is_retail,
                receipt_number: None,
            },
        );

        Ok(vec![event])
    }
}
```

**Step 2: 创建 table_opened.rs**

```rust
//! TableOpened event applier
//!
//! Applies the TableOpened event to create initial snapshot state.

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot, OrderStatus};

/// TableOpened applier
pub struct TableOpenedApplier;

impl EventApplier for TableOpenedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::TableOpened {
            table_id,
            table_name,
            zone_id,
            zone_name,
            guest_count,
            is_retail,
            receipt_number,
        } = &event.payload
        {
            snapshot.table_id = table_id.clone();
            snapshot.table_name = table_name.clone();
            snapshot.zone_id = zone_id.clone();
            snapshot.zone_name = zone_name.clone();
            snapshot.guest_count = *guest_count;
            snapshot.is_retail = *is_retail;
            snapshot.receipt_number = receipt_number.clone();
            snapshot.status = OrderStatus::Active;
            snapshot.start_time = event.timestamp;
            snapshot.created_at = event.timestamp;
            snapshot.updated_at = event.timestamp;
            snapshot.last_sequence = event.sequence;

            // Update checksum
            snapshot.update_checksum();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_opened_applier() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());

        let event = OrderEvent::new(
            1,
            "order-1".to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            1234567890,
            shared::order::OrderEventType::TableOpened,
            EventPayload::TableOpened {
                table_id: Some("T1".to_string()),
                table_name: Some("Table 1".to_string()),
                zone_id: Some("Z1".to_string()),
                zone_name: Some("Zone 1".to_string()),
                guest_count: 4,
                is_retail: false,
                receipt_number: None,
            },
        );

        let applier = TableOpenedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.table_id, Some("T1".to_string()));
        assert_eq!(snapshot.table_name, Some("Table 1".to_string()));
        assert_eq!(snapshot.guest_count, 4);
        assert_eq!(snapshot.status, OrderStatus::Active);
        assert_eq!(snapshot.last_sequence, 1);
    }
}
```

**Step 3: 更新 actions/mod.rs**

```rust
//! Command action implementations

use enum_dispatch::enum_dispatch;

use crate::orders::traits::CommandHandler;
use shared::order::{OrderCommand, OrderCommandPayload};

mod open_table;

pub use open_table::OpenTableAction;

#[enum_dispatch(CommandHandler)]
pub enum CommandAction {
    OpenTable(OpenTableAction),
}

impl From<&OrderCommand> for CommandAction {
    fn from(cmd: &OrderCommand) -> Self {
        match &cmd.payload {
            OrderCommandPayload::OpenTable {
                table_id,
                table_name,
                zone_id,
                zone_name,
                guest_count,
                is_retail,
            } => CommandAction::OpenTable(OpenTableAction {
                table_id: table_id.clone(),
                table_name: table_name.clone(),
                zone_id: zone_id.clone(),
                zone_name: zone_name.clone(),
                guest_count: *guest_count,
                is_retail: *is_retail,
            }),
            // Other commands will be added here
            _ => todo!("Command not yet implemented"),
        }
    }
}
```

**Step 4: 更新 appliers/mod.rs**

```rust
//! Event applier implementations

use enum_dispatch::enum_dispatch;

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

mod table_opened;

pub use table_opened::TableOpenedApplier;

#[enum_dispatch(EventApplier)]
pub enum EventAction {
    TableOpened(TableOpenedApplier),
}

impl From<&OrderEvent> for EventAction {
    fn from(event: &OrderEvent) -> Self {
        match &event.payload {
            EventPayload::TableOpened { .. } => EventAction::TableOpened(TableOpenedApplier),
            // Other events will be added here
            _ => todo!("Event applier not yet implemented"),
        }
    }
}
```

**Step 5: 验证编译并运行测试**

Run: `cargo test -p edge-server table_opened`
Expected: 测试通过

**Step 6: Commit**

```bash
git add edge-server/src/orders/actions edge-server/src/orders/appliers
git commit -m "feat(orders): implement OpenTableAction and TableOpenedApplier"
```

---

### Task 2.2: 实现 AddItemsAction 和 ItemsAddedApplier

**Files:**
- Create: `edge-server/src/orders/actions/add_items.rs`
- Create: `edge-server/src/orders/appliers/items_added.rs`
- Modify: `edge-server/src/orders/actions/mod.rs`
- Modify: `edge-server/src/orders/appliers/mod.rs`

**Step 1: 创建 add_items.rs**

```rust
//! AddItems command handler
//!
//! Adds items to an existing order.

use async_trait::async_trait;
use uuid::Uuid;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{
    CartItemInput, CartItemSnapshot, EventPayload, OrderEvent, OrderEventType, OrderStatus,
};

/// AddItems action
#[derive(Debug, Clone)]
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
        // 1. Load snapshot
        let mut snapshot = ctx.load_snapshot(&self.order_id)?;

        // 2. Validate order state
        if snapshot.status == OrderStatus::Completed {
            return Err(OrderError::OrderAlreadyCompleted(self.order_id.clone()));
        }
        if snapshot.status == OrderStatus::Void {
            return Err(OrderError::OrderAlreadyVoided(self.order_id.clone()));
        }

        // 3. Convert inputs to snapshots with generated instance_ids
        let items: Vec<CartItemSnapshot> = self
            .items
            .iter()
            .map(|input| input_to_snapshot(input))
            .collect();

        // 4. Update snapshot
        snapshot.items.extend(items.clone());
        snapshot.last_sequence = ctx.next_sequence();
        snapshot.updated_at = metadata.timestamp;
        snapshot.recalculate_totals();
        snapshot.update_checksum();

        // 5. Save snapshot
        ctx.save_snapshot(snapshot);

        // 6. Create event
        let event = OrderEvent::new(
            ctx.next_sequence() - 1,
            self.order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.timestamp,
            OrderEventType::ItemsAdded,
            EventPayload::ItemsAdded { items },
        );

        Ok(vec![event])
    }
}

/// Convert CartItemInput to CartItemSnapshot with generated instance_id
fn input_to_snapshot(input: &CartItemInput) -> CartItemSnapshot {
    let instance_id = Uuid::new_v4().to_string();
    let total = input.unit_price * input.quantity as f64;

    CartItemSnapshot {
        instance_id,
        item_id: input.item_id.clone(),
        item_name: input.item_name.clone(),
        item_name_zh: input.item_name_zh.clone(),
        category_id: input.category_id.clone(),
        quantity: input.quantity,
        unit_price: input.unit_price,
        total,
        modifiers: input.modifiers.clone(),
        notes: input.notes.clone(),
        voided: false,
        voided_at: None,
        void_reason: None,
        printed: false,
        split_quantity: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_to_snapshot() {
        let input = CartItemInput {
            item_id: "item-1".to_string(),
            item_name: "Coffee".to_string(),
            item_name_zh: Some("咖啡".to_string()),
            category_id: Some("cat-1".to_string()),
            quantity: 2,
            unit_price: 5.0,
            modifiers: vec![],
            notes: Some("Extra hot".to_string()),
        };

        let snapshot = input_to_snapshot(&input);

        assert!(!snapshot.instance_id.is_empty());
        assert_eq!(snapshot.item_id, "item-1");
        assert_eq!(snapshot.quantity, 2);
        assert_eq!(snapshot.unit_price, 5.0);
        assert_eq!(snapshot.total, 10.0);
        assert!(!snapshot.voided);
    }
}
```

**Step 2: 创建 items_added.rs**

```rust
//! ItemsAdded event applier
//!
//! Applies the ItemsAdded event to add items to the snapshot.

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

/// ItemsAdded applier
pub struct ItemsAddedApplier;

impl EventApplier for ItemsAddedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::ItemsAdded { items } = &event.payload {
            // Add items directly (they already have instance_ids from the event)
            snapshot.items.extend(items.clone());
            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;

            // Recalculate totals and checksum
            snapshot.recalculate_totals();
            snapshot.update_checksum();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::{CartItemSnapshot, OrderEventType};

    #[test]
    fn test_items_added_applier() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());

        let items = vec![CartItemSnapshot {
            instance_id: "inst-1".to_string(),
            item_id: "item-1".to_string(),
            item_name: "Coffee".to_string(),
            item_name_zh: None,
            category_id: None,
            quantity: 2,
            unit_price: 5.0,
            total: 10.0,
            modifiers: vec![],
            notes: None,
            voided: false,
            voided_at: None,
            void_reason: None,
            printed: false,
            split_quantity: None,
        }];

        let event = OrderEvent::new(
            2,
            "order-1".to_string(),
            "user-1".to_string(),
            "Test User".to_string(),
            1234567891,
            OrderEventType::ItemsAdded,
            EventPayload::ItemsAdded { items },
        );

        let applier = ItemsAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].instance_id, "inst-1");
        assert_eq!(snapshot.total, 10.0);
        assert_eq!(snapshot.last_sequence, 2);
    }
}
```

**Step 3: 更新 actions/mod.rs 添加 AddItemsAction**

在 `actions/mod.rs` 中：
- 添加 `mod add_items;`
- 添加 `pub use add_items::AddItemsAction;`
- 在 enum 中添加 `AddItems(AddItemsAction),`
- 在 From impl 中添加匹配分支

**Step 4: 更新 appliers/mod.rs 添加 ItemsAddedApplier**

在 `appliers/mod.rs` 中：
- 添加 `mod items_added;`
- 添加 `pub use items_added::ItemsAddedApplier;`
- 在 enum 中添加 `ItemsAdded(ItemsAddedApplier),`
- 在 From impl 中添加匹配分支

**Step 5: 验证测试**

Run: `cargo test -p edge-server items_added`
Expected: 测试通过

**Step 6: Commit**

```bash
git add edge-server/src/orders/actions edge-server/src/orders/appliers
git commit -m "feat(orders): implement AddItemsAction and ItemsAddedApplier"
```

---

### Task 2.3: 实现 CompleteOrderAction 和 OrderCompletedApplier

**Files:**
- Create: `edge-server/src/orders/actions/complete_order.rs`
- Create: `edge-server/src/orders/appliers/order_completed.rs`
- Modify: `edge-server/src/orders/actions/mod.rs`
- Modify: `edge-server/src/orders/appliers/mod.rs`

**Step 1: 创建 complete_order.rs**

从 `manager.rs:429-485` 迁移 `handle_complete_order` 逻辑。

关键点：
- 验证支付金额足够
- 计算 payment_summary
- 标记订单为 Completed 状态

**Step 2: 创建 order_completed.rs**

从 `reducer.rs:64-72` 迁移 apply 逻辑。

关键点：
- 设置 status = Completed
- 设置 receipt_number
- 设置 end_time

**Step 3-6: 同上一个 Task**

---

### Task 2.4: 实现 AddPaymentAction 和 PaymentAddedApplier

**Files:**
- Create: `edge-server/src/orders/actions/add_payment.rs`
- Create: `edge-server/src/orders/appliers/payment_added.rs`
- Modify: `edge-server/src/orders/actions/mod.rs`
- Modify: `edge-server/src/orders/appliers/mod.rs`

迁移 `handle_add_payment` 和对应的 reducer 逻辑。

---

## Phase 3: 中优先级命令实现

### Task 3.1: 实现 ModifyItemAction 和 ItemModifiedApplier

**Files:**
- Create: `edge-server/src/orders/actions/modify_item.rs`
- Create: `edge-server/src/orders/appliers/item_modified.rs`

这是最复杂的命令之一，需要处理：
- 部分修改（affected_quantity < item.quantity）
- 生成新的 instance_id
- ItemModificationResult 结构

---

### Task 3.2: 实现 RemoveItemAction 和 ItemRemovedApplier

**Files:**
- Create: `edge-server/src/orders/actions/remove_item.rs`
- Create: `edge-server/src/orders/appliers/item_removed.rs`

---

### Task 3.3: 实现 VoidOrderAction 和 OrderVoidedApplier

**Files:**
- Create: `edge-server/src/orders/actions/void_order.rs`
- Create: `edge-server/src/orders/appliers/order_voided.rs`

---

### Task 3.4: 实现 UpdateOrderInfoAction 和 OrderInfoUpdatedApplier

**Files:**
- Create: `edge-server/src/orders/actions/update_order_info.rs`
- Create: `edge-server/src/orders/appliers/order_info_updated.rs`

---

## Phase 4: 低优先级命令实现

### Task 4.1: 实现 CancelPaymentAction 和 PaymentCancelledApplier

### Task 4.2: 实现 MoveOrderAction 和 OrderMovedApplier

### Task 4.3: 实现 MergeOrdersAction 和 OrdersMergedApplier

### Task 4.4: 实现 SplitOrderAction 和 OrderSplitApplier

### Task 4.5: 实现 RestoreOrderAction 和 OrderRestoredApplier

### Task 4.6: 实现 RestoreItemAction 和 ItemRestoredApplier

---

## Phase 5: OrdersManager 重构

### Task 5.1: 创建新的 execute_command 流程

**Files:**
- Modify: `edge-server/src/orders/manager.rs`

**Step 1: 添加新的 process_command_v2 方法**

```rust
/// Process command using the new action-based architecture
async fn process_command_v2(
    &self,
    cmd: OrderCommand,
) -> ManagerResult<(CommandResponse, Vec<OrderEvent>)> {
    // 1. Idempotency check
    if self.storage.is_command_processed(&cmd.command_id)? {
        return Ok((CommandResponse::duplicate(cmd.command_id), vec![]));
    }

    // 2. Begin transaction
    let txn = self.storage.begin_write()?;

    // Double-check within transaction
    if self.storage.is_command_processed_txn(&txn, &cmd.command_id)? {
        return Ok((CommandResponse::duplicate(cmd.command_id), vec![]));
    }

    // 3. Get current sequence
    let current_sequence = self.storage.current_sequence(&txn)?;

    // 4. Create context
    let mut ctx = CommandContext::new(&txn, &self.storage, current_sequence);

    // 5. Create metadata
    let metadata = CommandMetadata {
        command_id: cmd.command_id.clone(),
        operator_id: cmd.operator_id.clone(),
        operator_name: cmd.operator_name.clone(),
        timestamp: cmd.timestamp,
    };

    // 6. Convert to action and execute
    let action: CommandAction = (&cmd).into();
    let events = action.execute(&mut ctx, &metadata).await
        .map_err(|e| ManagerError::from(e))?;

    // 7. Persist events
    for event in &events {
        self.storage.store_event(&txn, event)?;
    }

    // 8. Persist snapshots
    for snapshot in ctx.modified_snapshots() {
        self.storage.store_snapshot(&txn, snapshot)?;
    }

    // 9. Update sequence counter
    let final_sequence = ctx.next_sequence() - 1;
    self.storage.set_sequence(&txn, final_sequence)?;

    // 10. Mark command processed
    self.storage.mark_command_processed(&txn, &cmd.command_id)?;

    // 11. Commit
    txn.commit().map_err(StorageError::from)?;

    // 12. Return response
    let order_id = events.first().map(|e| e.order_id.clone());
    Ok((
        CommandResponse::success(cmd.command_id, order_id),
        events,
    ))
}
```

**Step 2: 切换 execute_command 使用 v2**

```rust
pub fn execute_command(&self, cmd: OrderCommand) -> CommandResponse {
    // Use tokio runtime for async execution
    let rt = tokio::runtime::Handle::current();
    match rt.block_on(self.process_command_v2(cmd.clone())) {
        Ok((response, events)) => {
            for event in events {
                let _ = self.event_tx.send(event);
            }
            response
        }
        Err(err) => CommandResponse::error(cmd.command_id, err.into()),
    }
}
```

**Step 3: 验证测试**

Run: `cargo test -p edge-server`
Expected: 所有现有测试通过

**Step 4: Commit**

```bash
git add edge-server/src/orders/manager.rs
git commit -m "refactor(orders): switch to action-based command processing"
```

---

### Task 5.2: 实现 rebuild_snapshot 使用 EventAction

**Files:**
- Modify: `edge-server/src/orders/manager.rs`

```rust
/// Rebuild snapshot from events using the applier architecture
pub fn rebuild_snapshot(&self, order_id: &str) -> ManagerResult<OrderSnapshot> {
    let txn = self.storage.begin_read()?;
    let events = self.storage.get_events_for_order(&txn, order_id)?;

    if events.is_empty() {
        return Err(ManagerError::OrderNotFound(order_id.to_string()));
    }

    let mut snapshot = OrderSnapshot::new(order_id.to_string());

    for event in &events {
        let applier: EventAction = event.into();
        applier.apply(&mut snapshot, event);
    }

    Ok(snapshot)
}
```

---

## Phase 6: 清理与验证

### Task 6.1: 删除旧代码

**Files:**
- Modify: `edge-server/src/orders/manager.rs`
- Modify: `edge-server/src/orders/reducer.rs`

**Step 1: 删除 manager.rs 中所有 handle_xxx 方法**

删除以下方法：
- `handle_open_table`
- `handle_complete_order`
- `handle_void_order`
- `handle_restore_order`
- `handle_add_items`
- `handle_modify_item`
- `handle_remove_item`
- `handle_restore_item`
- `handle_add_payment`
- `handle_cancel_payment`
- `handle_split_order`
- `handle_move_order`
- `handle_merge_orders`
- `handle_update_order_info`

**Step 2: 删除旧的 process_command 方法**

保留 `process_command_v2` 并重命名为 `process_command`。

**Step 3: 简化 reducer.rs**

保留辅助函数（如 `input_to_snapshot`），删除 `apply_event` 和相关方法，因为它们已被 appliers 替代。

**Step 4: 验证测试**

Run: `cargo test -p edge-server`
Expected: 所有测试通过

**Step 5: Commit**

```bash
git add edge-server/src/orders/
git commit -m "refactor(orders): remove legacy match-based handlers"
```

---

### Task 6.2: 运行完整测试套件

**Step 1: 运行单元测试**

Run: `cargo test --workspace --lib`
Expected: 所有测试通过

**Step 2: 运行 clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: 无警告

**Step 3: 格式化代码**

Run: `cargo fmt --all`

**Step 4: 最终 Commit**

```bash
git add .
git commit -m "chore: final cleanup and formatting"
```

---

## 命令/事件完整清单

| Command | Action | Event | Applier | Priority |
|---------|--------|-------|---------|----------|
| OpenTable | OpenTableAction | TableOpened | TableOpenedApplier | 🔴 High |
| AddItems | AddItemsAction | ItemsAdded | ItemsAddedApplier | 🔴 High |
| CompleteOrder | CompleteOrderAction | OrderCompleted | OrderCompletedApplier | 🔴 High |
| AddPayment | AddPaymentAction | PaymentAdded | PaymentAddedApplier | 🔴 High |
| ModifyItem | ModifyItemAction | ItemModified | ItemModifiedApplier | 🟡 Medium |
| RemoveItem | RemoveItemAction | ItemRemoved | ItemRemovedApplier | 🟡 Medium |
| VoidOrder | VoidOrderAction | OrderVoided | OrderVoidedApplier | 🟡 Medium |
| UpdateOrderInfo | UpdateOrderInfoAction | OrderInfoUpdated | OrderInfoUpdatedApplier | 🟡 Medium |
| CancelPayment | CancelPaymentAction | PaymentCancelled | PaymentCancelledApplier | 🟢 Low |
| MoveOrder | MoveOrderAction | OrderMoved/OrderMovedOut | OrderMovedApplier | 🟢 Low |
| MergeOrders | MergeOrdersAction | OrderMerged/OrderMergedOut | OrdersMergedApplier | 🟢 Low |
| SplitOrder | SplitOrderAction | OrderSplit | OrderSplitApplier | 🟢 Low |
| RestoreOrder | RestoreOrderAction | OrderRestored | OrderRestoredApplier | 🟢 Low |
| RestoreItem | RestoreItemAction | ItemRestored | ItemRestoredApplier | 🟢 Low |

---

## 验收标准

1. ✅ 所有 14 个命令都有对应的 Action 和 Applier
2. ✅ `cargo test --workspace` 全部通过
3. ✅ `cargo clippy --workspace -- -D warnings` 无警告
4. ✅ manager.rs 从 1200+ 行减少到 ~300 行
5. ✅ 没有分散的 match 语句（只在 From trait 中保留）
6. ✅ 每个 Action/Applier 有独立的单元测试
