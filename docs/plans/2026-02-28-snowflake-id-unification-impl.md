# Snowflake ID 全栈统一化 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将所有 UUID (order_id, event_id, command_id, payment_id, comp_id) 统一为 snowflake i64，消除 String↔i64 转换，移除 order_key 概念。

**Architecture:** Bottom-up — 从 shared 类型开始，向上修复 edge-server、cloud、前端。redb 表定义从 `&str` 键改为 `i64` 键。`SyncPayload.id` 和 `CloudSyncItem.resource_id` 改为 `i64`。`broadcast_sync` 拆分为带 ID 和无 ID 两个方法。

**Tech Stack:** Rust (shared, edge-server, crab-cloud), TypeScript (red_coral), SQLite, PostgreSQL, redb

**Design doc:** `docs/plans/2026-02-28-snowflake-id-unification-design.md`

---

### Task 1: shared 订单核心类型 — String → i64

**Files:**
- Modify: `shared/src/order/snapshot.rs:29` — `order_id: String` → `i64`
- Modify: `shared/src/order/event.rs:14,19,32` — `event_id`, `order_id`, `command_id` → `i64`
- Modify: `shared/src/order/event.rs:538` — `uuid::Uuid::new_v4().to_string()` → `snowflake_id()`
- Modify: `shared/src/order/command.rs:12,316` — `command_id: String` → `i64`, UUID → `snowflake_id()`
- Modify: `shared/src/order/command.rs` — 所有 22 个 `order_id: String` 字段 (lines 46,54,77,83,98,115,121,134,144,154,165,176,191-192,202,213,221,237,251,262,278,285,288,292,303) → `i64`
- Modify: `shared/src/order/command.rs:325` — `target_order_id()` 返回 `Option<i64>` 而非 `Option<&str>`
- Modify: `shared/src/order/types.rs:285` — `PaymentRecord.payment_id: String` → `i64`
- Modify: `shared/src/order/types.rs:321,326` — `CommandResponse.command_id: String` → `i64`, `order_id: Option<String>` → `Option<i64>`
- Modify: `shared/src/order/types.rs:333` — `CommandResponse::success()` 参数更新
- Modify: `shared/src/order/types.rs:487` — `CompRecord.comp_id: String` → `i64`
- Modify: `shared/src/order/event.rs:250,262,276,288,306` — `payment_id: String` → `i64` in EventPayload variants
- Modify: `shared/src/order/command.rs:122` — `CancelPayment.payment_id: String` → `i64`

**Step 1: 修改 OrderSnapshot**

`shared/src/order/snapshot.rs:29`:
```rust
pub order_id: i64,
```

找到 `OrderSnapshot::new()` (如果有)，参数改为 `i64`。

**Step 2: 修改 OrderEvent**

`shared/src/order/event.rs`:
```rust
pub event_id: i64,       // line 14
pub order_id: i64,       // line 19
pub command_id: i64,     // line 32
```

`OrderEvent::new()` (line ~536):
```rust
event_id: crate::util::snowflake_id(),  // was uuid::Uuid::new_v4().to_string()
```
参数 `order_id: String` → `i64`, `command_id: String` → `i64`

**Step 3: 修改 OrderCommand**

`shared/src/order/command.rs`:
```rust
pub command_id: i64,  // line 12
```

`OrderCommand::new()` (line 314-322):
```rust
command_id: crate::util::snowflake_id(),
```

`target_order_id()` (line 325):
```rust
pub fn target_order_id(&self) -> Option<i64> {
    match &self.payload {
        OrderCommandPayload::OpenTable { .. } => None,
        OrderCommandPayload::CompleteOrder { order_id, .. } => Some(*order_id),
        // ... all variants return Some(*order_id)
    }
}
```

所有 `OrderCommandPayload` variants 中 `order_id: String` → `order_id: i64`，`source_order_id: String` / `target_order_id: String` → `i64`。

**Step 4: 修改 PaymentRecord, CompRecord, CommandResponse**

`shared/src/order/types.rs`:
```rust
// PaymentRecord (line 285)
pub payment_id: i64,

// CommandResponse (lines 321, 326)
pub command_id: i64,
pub order_id: Option<i64>,

// CompRecord (line 487)
pub comp_id: i64,
```

`CommandResponse::success()` (line 333):
```rust
pub fn success(command_id: i64, order_id: Option<i64>) -> Self { ... }
```

**Step 5: 修改 EventPayload variants 中的 payment_id**

`shared/src/order/event.rs` — EventPayload variants:
- `PaymentAdded { payment_id: i64, ... }` (line 250)
- `PaymentCancelled { payment_id: i64, ... }` (line 262)
- `ItemSplit { payment_id: i64, ... }` (line 276)
- `AmountSplit { payment_id: i64, ... }` (line 288)
- `AaSplitPaid { payment_id: i64, ... }` (line 306)

**Step 6: 验证编译**

Run: `cargo check -p shared 2>&1 | head -50`
Expected: 大量下游编译错误（edge-server 等），shared 本身应编译通过。

---

### Task 2: shared canonical hash 更新

**Files:**
- Modify: `shared/src/order/canonical.rs:451` — `write_str → write_i64` for payment_id
- Modify: `shared/src/order/canonical.rs:468` — `write_str → write_i64` for comp_id
- Modify: `shared/src/order/canonical.rs:1014-1020` — `write_str → write_i64` for event_id, order_id, command_id
- Modify: `shared/src/order/canonical.rs:1033-1046` — `compute_order_chain_hash` 参数 `order_id: &str` → `i64`

**Step 1: 修改 PaymentRecord canonical hash**

`canonical.rs:449-463`:
```rust
impl CanonicalHash for PaymentRecord {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_i64(buf, self.payment_id);  // was write_str
        write_str(buf, &self.method);
        // ... rest unchanged
    }
}
```

**Step 2: 修改 CompRecord canonical hash**

`canonical.rs:466-478`:
```rust
impl CanonicalHash for CompRecord {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_i64(buf, self.comp_id);  // was write_str
        write_str(buf, &self.instance_id);  // KEEP String
        // ... rest unchanged
    }
}
```

**Step 3: 修改 OrderEvent canonical hash**

`canonical.rs:1012-1025`:
```rust
impl CanonicalHash for super::event::OrderEvent {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_i64(buf, self.event_id);    // was write_str
        write_i64(buf, self.order_id);    // was write_str
        write_u64(buf, self.sequence);
        write_i64(buf, self.timestamp);
        write_i64(buf, self.operator_id);
        write_str(buf, &self.operator_name);
        write_i64(buf, self.command_id);  // was write_str
        write_opt_i64(buf, self.client_timestamp);
        self.event_type.canonical_bytes(buf);
        write_sep(buf);
        self.payload.canonical_bytes(buf);
    }
}
```

**Step 4: 修改 compute_order_chain_hash**

`canonical.rs:1033-1054`:
```rust
pub fn compute_order_chain_hash(
    prev_hash: &str,
    order_id: i64,           // was &str
    receipt_number: &str,
    status: &OrderStatus,
    last_event_hash: &str,
    total_amount: f64,
    tax: f64,
) -> String {
    use sha2::{Digest, Sha256};
    let mut buf = Vec::with_capacity(256);
    write_str(&mut buf, prev_hash);
    write_i64(&mut buf, order_id);  // was write_str
    write_str(&mut buf, receipt_number);
    status.canonical_bytes(&mut buf);
    write_str(&mut buf, last_event_hash);
    write_f64(&mut buf, total_amount);
    write_f64(&mut buf, tax);
    format!("{:x}", Sha256::digest(&buf))
}
```

**Step 5: 验证编译**

Run: `cargo check -p shared`

---

### Task 3: shared 系统状态和同步协议类型

**Files:**
- Modify: `shared/src/models/system_state.rs:11,13,26,28,44,51` — `last_order_id`, `synced_up_to_id` → `Option<i64>` / `i64`
- Modify: `shared/src/message/payload.rs:182` — `SyncPayload.id: String` → `i64`
- Modify: `shared/src/cloud/sync.rs:190,213` — `CloudSyncItem.resource_id`, `CloudSyncError.resource_id` → `i64`
- Modify: `shared/src/cloud/sync.rs:225` — `OrderDetailSync.order_key: String` → 删除
- Modify: `shared/src/cloud/sync.rs:338` — `CreditNoteSync.original_order_key: String` → `original_order_id: i64`
- Modify: `shared/src/cloud/ws.rs:67` — `CloudRpc::GetOrderDetail { order_key }` → `{ order_id: i64 }`

**Step 1: 修改 SystemState**

`shared/src/models/system_state.rs`:
```rust
pub struct SystemState {
    pub id: i64,
    pub genesis_hash: Option<String>,
    pub last_order_id: Option<i64>,       // was Option<String>
    pub last_chain_hash: Option<String>,
    pub synced_up_to_id: Option<i64>,     // was Option<String>
    pub synced_up_to_hash: Option<String>,
    pub last_sync_time: Option<i64>,
    pub order_count: i64,
    pub last_huella: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct SystemStateUpdate {
    pub genesis_hash: Option<String>,
    pub last_order_id: Option<i64>,       // was Option<String>
    pub last_chain_hash: Option<String>,
    pub synced_up_to_id: Option<i64>,     // was Option<String>
    // ... rest same
}

pub struct UpdateLastOrderRequest {
    pub order_id: i64,                    // was String
    pub order_hash: String,
}

pub struct UpdateSyncStateRequest {
    pub synced_up_to_id: i64,            // was String
    pub synced_up_to_hash: String,
}
```

**Step 2: 修改 SyncPayload**

`shared/src/message/payload.rs:174-188`:
```rust
pub struct SyncPayload {
    pub resource: crate::cloud::SyncResource,
    pub version: u64,
    pub action: SyncChangeType,
    pub id: i64,                          // was String
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    pub cloud_origin: bool,
}
```

**Step 3: 修改 CloudSyncItem 和 CloudSyncError**

`shared/src/cloud/sync.rs`:
```rust
// line 190
pub resource_id: i64,    // was String

// line 213
pub resource_id: i64,    // was String
```

**Step 4: 修改 OrderDetailSync — 删除 order_key**

`shared/src/cloud/sync.rs:222-242`:
- 删除 `order_key: String` 字段 (line 225)
- 所有构造/引用 `order_key` 的地方需要更新

**Step 5: 修改 CreditNoteSync**

`shared/src/cloud/sync.rs:338`:
```rust
pub original_order_id: i64,  // was original_order_key: String
```

**Step 6: 修改 CloudRpc**

`shared/src/cloud/ws.rs:67`:
```rust
GetOrderDetail { order_id: i64 },  // was order_key: String
```

**Step 7: 更新测试中的 UUID 值**

搜索 `shared/src/cloud/sync.rs` 和 `shared/src/cloud/ws.rs` 中的测试，将 `"uuid-order-001"` 等 UUID 字符串改为 snowflake i64 值。

**Step 8: 验证编译**

Run: `cargo check -p shared`

---

### Task 4: edge-server redb 存储层 — &str → i64

**Files:**
- Modify: `edge-server/src/orders/storage.rs:37-60` — 所有 TableDefinition 键类型
- Modify: `edge-server/src/orders/storage.rs:72,81` — `PendingArchive.order_id`, `DeadLetterEntry.order_id` → `i64`
- Modify: `edge-server/src/orders/storage.rs` — 所有读写方法的参数和逻辑

**Step 1: 修改 TableDefinition**

`storage.rs:36-60`:
```rust
const EVENTS_TABLE: TableDefinition<(i64, u64), &[u8]> = TableDefinition::new("events");
const SNAPSHOTS_TABLE: TableDefinition<i64, &[u8]> = TableDefinition::new("snapshots");
const ACTIVE_ORDERS_TABLE: TableDefinition<i64, ()> = TableDefinition::new("active_orders");
const PROCESSED_COMMANDS_TABLE: TableDefinition<i64, ()> = TableDefinition::new("processed_commands");
const SEQUENCE_TABLE: TableDefinition<&str, u64> = TableDefinition::new("sequence_counter");  // KEEP &str (sentinel keys)
const PENDING_ARCHIVE_TABLE: TableDefinition<i64, &[u8]> = TableDefinition::new("pending_archive");
const DEAD_LETTER_TABLE: TableDefinition<i64, &[u8]> = TableDefinition::new("dead_letter");
const RULE_SNAPSHOTS_TABLE: TableDefinition<i64, &[u8]> = TableDefinition::new("rule_snapshots");
```

注意：`SEQUENCE_TABLE` 保持 `&str` 因为使用 sentinel 键 ("seq", "order_count", "queue_number" 等)。

**Step 2: 修改 PendingArchive / DeadLetterEntry**

```rust
pub struct PendingArchive {
    pub order_id: i64,      // was String
    pub created_at: i64,
    pub retry_count: u32,
    pub last_error: Option<String>,
}

pub struct DeadLetterEntry {
    pub order_id: i64,      // was String
    pub created_at: i64,
    pub failed_at: i64,
    pub retry_count: u32,
    pub last_error: String,
}
```

**Step 3: 修改所有存储方法**

遍历 `storage.rs` 中所有方法，将 `order_id: &str` 参数改为 `order_id: i64`，`command_id: &str` 改为 `command_id: i64`。

关键方法：
- `save_events(txn, order_id: i64, events)` — key 从 `(order_id, seq)` String tuple 改为 `(order_id, seq)` i64 tuple
- `load_events(txn, order_id: i64)` — range scan key prefix 改为 i64
- `save_snapshot(txn, order_id: i64, snapshot)`
- `load_snapshot(txn, order_id: i64)`
- `mark_order_active(txn, order_id: i64)`
- `mark_order_inactive(txn, order_id: i64)`
- `mark_command_processed(txn, command_id: i64)`
- `is_command_processed(txn, command_id: i64)`
- `queue_for_archive(txn, order_id: i64, ...)`
- `get_active_orders(txn)` — 返回 `Vec<i64>`
- `dequeue_archive(txn, order_id: i64)`
- `move_to_dead_letter(txn, order_id: i64, ...)`

**Step 4: 验证编译**

Run: `cargo check -p edge-server 2>&1 | head -80`
Expected: 大量下游错误，storage 本身应通过。

---

### Task 5: edge-server OrdersManager 和 traits

**Files:**
- Modify: `edge-server/src/orders/traits.rs` — `CommandContext` snapshot cache key 和方法参数
- Modify: `edge-server/src/orders/manager/mod.rs:98` — `rule_cache: HashMap<String, ...>` → `HashMap<i64, ...>`
- Modify: `edge-server/src/orders/manager/mod.rs` — 所有 `order_id: &str` 参数 → `i64`
- Modify: `edge-server/src/orders/manager/mod.rs:128,188` — epoch UUID → snowflake
- Modify: `edge-server/src/orders/manager/mod.rs:1209` — `get_snapshot(&self, order_id: i64)`

**Step 1: 修改 CommandContext (traits.rs)**

将 `snapshot_cache: HashMap<String, OrderSnapshot>` 改为 `HashMap<i64, OrderSnapshot>`。
`create_snapshot(order_id: i64)`, `save_snapshot()`, `get_snapshot(order_id: i64)` 等方法参数更新。

**Step 2: 修改 OrdersManager**

- `rule_cache: Arc<RwLock<HashMap<i64, Vec<PriceRule>>>>` (line 98)
- 所有 `get_snapshot(&self, order_id: &str)` → `get_snapshot(&self, order_id: i64)`
- `rebuild_snapshot(&self, order_id: &str)` → `i64`
- `load_snapshot(&event.order_id)` — 现在 order_id 是 i64，直接传
- `events.first().map(|e| e.order_id)` — 不再需要 `.clone()`
- epoch: `uuid::Uuid::new_v4().to_string()` → `snowflake_id().to_string()` (epoch 仅用于日志标识，保持 String 但不再需要 UUID)

**Step 3: 验证编译**

Run: `cargo check -p edge-server 2>&1 | head -80`

---

### Task 6: edge-server 所有 CommandHandler actions — order_id/payment_id UUID → snowflake

**Files:**
- Modify: `edge-server/src/orders/actions/open_table.rs:117` — `Uuid::new_v4().to_string()` → `snowflake_id()`
- Modify: `edge-server/src/orders/actions/add_payment.rs:68` — payment_id UUID → snowflake
- Modify: `edge-server/src/orders/actions/complete_order.rs:124` — payment_id UUID → snowflake
- Modify: `edge-server/src/orders/actions/comp_item.rs:111` — `::comp::{}` suffix UUID → snowflake
- Modify: `edge-server/src/orders/actions/modify_item.rs:307` — `::mod::{}` suffix UUID → snowflake
- Modify: `edge-server/src/orders/actions/split_order/aa_split.rs:102,188` — payment_id UUID → snowflake
- Modify: `edge-server/src/orders/actions/split_order/split_by_items.rs:58` — payment_id UUID → snowflake
- Modify: `edge-server/src/orders/actions/split_order/split_by_amount.rs:47` — payment_id UUID → snowflake
- Modify: `edge-server/src/orders/appliers/order_split.rs:338` — payment_id UUID → snowflake
- Modify: `edge-server/src/orders/appliers/item_removed.rs:126` — synthetic event event_id UUID → snowflake

**Step 1: open_table.rs — order_id 生成**

```rust
// line 117: was Uuid::new_v4().to_string()
let order_id = snowflake_id();
```

添加 `use shared::util::snowflake_id;`，移除 `use uuid::Uuid;`

**Step 2: 所有 payment_id 生成**

所有 `uuid::Uuid::new_v4().to_string()` 用于 payment_id 的地方改为 `snowflake_id()`。
`complete_order.rs:124` 的 `format!("pay-{}", uuid::Uuid::new_v4())` 改为 `snowflake_id()`。

**Step 3: comp/modify split suffixes**

```rust
// comp_item.rs:111 — was format!("{}::comp::{}", self.instance_id, uuid::Uuid::new_v4())
let derived_id = format!("{}::comp::{}", self.instance_id, snowflake_id());

// modify_item.rs:307 — was format!("{}::mod::{}", base_id, uuid::Uuid::new_v4())
format!("{}::mod::{}", base_id, snowflake_id())
```

**Step 4: synthetic event IDs**

`item_removed.rs:126`, `event_router.rs:189,196` — 合成事件的 event_id/command_id:
```rust
event_id: snowflake_id(),   // was uuid::Uuid::new_v4().to_string()
command_id: snowflake_id(), // was uuid::Uuid::new_v4().to_string()
```

**Step 5: 验证编译**

Run: `cargo check -p edge-server 2>&1 | head -80`

---

### Task 7: edge-server broadcast_sync 拆分 + state.rs

**Files:**
- Modify: `edge-server/src/core/state.rs:883-905` — 拆分 `broadcast_sync`
- Modify: `edge-server/src/core/state.rs:635-656` — order sync forwarder 使用 i64
- Modify: `edge-server/src/core/state.rs:311` — epoch UUID → snowflake

**Step 1: 拆分 broadcast_sync**

```rust
/// 带 ID 的资源同步
pub async fn broadcast_sync<T: serde::Serialize>(
    &self,
    resource: SyncResource,
    action: SyncChangeType,
    id: i64,                    // was &str
    data: Option<&T>,
    cloud_origin: bool,
) {
    let version = self.resource_versions.increment(resource);
    let payload = SyncPayload {
        resource,
        version,
        action,
        id,                     // was id.to_string()
        data: data.and_then(|d| serde_json::to_value(d).ok()),
        cloud_origin,
    };
    tracing::debug!(resource = %resource, action = ?action, id, cloud_origin, "Broadcasting sync event");
    match self.message_bus().publish(BusMessage::sync(&payload)).await {
        Ok(_) => {}
        Err(e) => tracing::error!("Sync broadcast failed: {}", e),
    }
}

/// 无 ID 的单例资源同步 (StoreInfo, batch updates)
pub async fn broadcast_sync_singleton<T: serde::Serialize>(
    &self,
    resource: SyncResource,
    action: SyncChangeType,
    data: Option<&T>,
    cloud_origin: bool,
) {
    let version = self.resource_versions.increment(resource);
    let payload = SyncPayload {
        resource,
        version,
        action,
        id: 0,                  // sentinel for singleton
        data: data.and_then(|d| serde_json::to_value(d).ok()),
        cloud_origin,
    };
    tracing::debug!(resource = %resource, action = ?action, cloud_origin, "Broadcasting singleton sync");
    match self.message_bus().publish(BusMessage::sync(&payload)).await {
        Ok(_) => {}
        Err(e) => tracing::error!("Sync broadcast failed: {}", e),
    }
}
```

**Step 2: 更新所有 broadcast_sync 调用者**

搜索 `broadcast_sync(` 并根据类型更新：
- 使用 `"main"` 的调用 → `broadcast_sync_singleton`
- 使用 `"batch"` 的调用 → `broadcast_sync_singleton`
- 使用 i64 ID 的调用 → `broadcast_sync` (参数已是 i64)
- 使用 `&id.to_string()` 的调用 → 直接传 `id`

**Step 3: 更新 order sync forwarder (line 635-656)**

```rust
let order_id = event.order_id;  // now i64, no clone needed
let payload = SyncPayload {
    resource: SyncResource::OrderSync,
    version: sequence,
    action,
    id: order_id,
    // ...
};
```

**Step 4: epoch 更新**

```rust
// line 311
let epoch = snowflake_id().to_string();  // was uuid::Uuid::new_v4().to_string()
```

**Step 5: 验证编译**

Run: `cargo check -p edge-server 2>&1 | head -80`

---

### Task 8: edge-server 归档服务 — 删除 order_key

**Files:**
- Modify: `edge-server/src/archiving/service.rs` — 删除 `order_key` 字段引用，归档 SQL 不再存/查 order_key
- Modify: `edge-server/src/db/repository/order.rs:14,106` — 删除 `order_key` 字段
- Modify: `edge-server/src/db/repository/credit_note.rs:219,229,253` — `order_key` → `order_id` (i64)
- Modify: `edge-server/src/archiving/credit_note.rs` — build_sync 中 order_key 相关

**Step 1: 修改 archived_order INSERT (service.rs)**

`service.rs:442` — INSERT SQL 中移除 `order_key` 列。当前 `order_key` 来自 `OrderSnapshot.order_id` (UUID) — 现在 order_id 是 snowflake，整个 order_key 概念删除。

**Step 2: 修改 order repository**

`db/repository/order.rs:14`:
- 删除 `order_key: String` 字段
- 所有 SELECT 查询移除 `order_key`

**Step 3: 修改 chain hash 查询 (service.rs:850,971)**

将 `ao.order_key` 引用改为不再需要（chain hash 函数现在接受 `i64` order_id）

**Step 4: 修改 OrderDetailSync 构建 (service.rs:1015)**

```rust
// was: order_id: order.order_key
// OrderDetailSync 不再有 order_key 字段
// order_id 现在是 snowflake i64，通过 resource_id 传递
```

**Step 5: 修改 credit_note build_sync**

`db/repository/credit_note.rs:219,229,253`:
- CnRow 中 `order_key: String` → `order_id: i64` (从 `ao.id` 查询)
- SQL JOIN: `ao.order_key` → `ao.id`
- `CreditNoteSync` 构造: `original_order_key` → `original_order_id: ao.id`

**Step 6: 验证编译**

Run: `cargo check -p edge-server 2>&1 | head -80`

---

### Task 9: edge-server cloud worker 和 RPC — 消除 String 转换

**Files:**
- Modify: `edge-server/src/cloud/worker.rs` — `resource_id` 直接用 i64，不再 `.to_string()`
- Modify: `edge-server/src/cloud/rpc_executor.rs:37-42` — `order_key` → `order_id: i64` 查询
- Modify: `edge-server/src/cloud/worker.rs:368-398` — 同上

**Step 1: CloudWorker pending HashMap**

```rust
// was: HashMap<SyncResource, HashMap<String, CloudSyncItem>>
pending: HashMap<SyncResource, HashMap<i64, CloudSyncItem>>
```

所有 `resource_id.clone()` 作为 key 改为直接 `resource_id` (i64 Copy)。

**Step 2: CloudSyncItem 构建**

所有 `resource_id: id.to_string()` 改为 `resource_id: id`。
StoreInfo sentinel: `resource_id: 0` (或 1，与 broadcast_sync_singleton 一致)。

**Step 3: extract_sync_item 更新**

```rust
// was: resource_id: payload.id
resource_id: payload.id,  // now i64 directly
```

**Step 4: RPC executor 更新**

`rpc_executor.rs:37-42`:
```rust
CloudRpc::GetOrderDetail { order_id } => {
    // Resolve order_id → pk (now they're the same!)
    let detail = order_repo::get_order_detail(&state.pool, order_id).await;
    // ...
}
```

**Step 5: 验证编译**

Run: `cargo check -p edge-server 2>&1 | head -80`

---

### Task 10: edge-server 其他模块 — 补全所有编译错误

**Files:**
- Modify: `edge-server/src/api/orders/handler.rs` — order_key 引用
- Modify: `edge-server/src/api/kitchen_orders/handler.rs` — order_key 引用
- Modify: `edge-server/src/api/statistics/handler.rs` — 如有 order_id 类型
- Modify: `edge-server/src/api/system_state/handler.rs` — UpdateLastOrderRequest 等
- Modify: `edge-server/src/services/https.rs` — 如有 order_id 类型
- Modify: `edge-server/src/printing/service.rs:132` — print record UUID → snowflake
- Modify: `edge-server/src/order_sync.rs` — 如有 order_id 类型
- Modify: `edge-server/src/api/sync/handler.rs` — 如有 order_id 类型
- Modify: `edge-server/src/message/tcp_server.rs:320,400` — client_name/request_id UUID 处理

**Step 1: 运行 cargo check 并逐一修复**

Run: `cargo check -p edge-server 2>&1`

按错误逐一修复。关键模式：
- `order_key` 引用 → 删除或改为 `order_id` (i64)
- `order_id: &str` 参数 → `i64`
- `.to_string()` / `.clone()` on order_id → 直接使用 (i64 is Copy)
- kitchen_orders handler 中的 `order_key` 查询 → 使用 `order_id`

**Step 2: 处理 printing service UUID**

`printing/service.rs:132`:
```rust
id: snowflake_id().to_string(),  // print record ID (仍用 String 格式)
```
或者如果 print record id 也是 i64，直接 `snowflake_id()`。

**Step 3: 处理 tcp_server UUID**

`tcp_server.rs:320` — client_name fallback UUID → 保持（这是连接标识，不是实体 ID）
`tcp_server.rs:400` — request_id UUID → 可保持（内部请求跟踪）

**Step 4: 验证编译**

Run: `cargo check -p edge-server`
Expected: 零错误

---

### Task 11: edge-server 数据库 migration — 删除 order_key

**Files:**
- Create: `edge-server/migrations/XXXX_drop_order_key.sql` (上下迁移)

**Step 1: 创建 migration**

Run: `sqlx migrate add -r -s drop_order_key --source edge-server/migrations`

Up migration:
```sql
-- 删除 order_key 列 (SQLite 不支持 DROP COLUMN，需要重建表)
-- 如果 SQLite 版本 >= 3.35.0 可以直接 ALTER TABLE ... DROP COLUMN

-- 确认 SQLite 版本支持 DROP COLUMN (3.35.0+)
ALTER TABLE archived_order DROP COLUMN order_key;
```

Down migration:
```sql
ALTER TABLE archived_order ADD COLUMN order_key TEXT;
```

注意：开发阶段，如果 SQLite 版本不支持 DROP COLUMN，可以用 `CREATE TABLE ... AS SELECT` 重建表。

**Step 2: 验证 migration**

Run: `sqlx migrate run --source edge-server/migrations`

---

### Task 12: crab-cloud 同步入库 — 消除 parse::<i64>()

**Files:**
- Modify: `crab-cloud/src/db/sync_store.rs` — 移除所有 `.parse::<i64>()` 调用
- Modify: `crab-cloud/src/db/sync_store.rs:274,297,337,351,358,382` — order_key → order_id
- Modify: `crab-cloud/src/db/tenant_queries.rs:158,174-175,179,193,216,238,484-496,507-518,1160-1172` — order_key 相关查询
- Modify: `crab-cloud/src/api/tenant/order.rs:54-66` — order_key URL 参数 → order_id
- Modify: `crab-cloud/src/api/tenant/command.rs:38-44` — order_key → order_id
- Modify: `crab-cloud/src/api/mod.rs:79,83` — URL path `order_key` → `order_id`

**Step 1: sync_store.rs 清理**

所有 `let source_id: i64 = item.resource_id.parse()?;` 改为直接使用 `item.resource_id` (已是 i64)。

**Step 2: sync_store.rs order_key 清理**

- INSERT 语句移除 `order_key` 列
- `ON CONFLICT` 从 `(tenant_id, store_id, order_key)` 改为 `(tenant_id, store_id, source_id)`
- `credit_note` 的 `original_order_key` → `original_order_id` (i64，直接绑定)

**Step 3: tenant_queries.rs 更新**

所有 `order_key` 引用改为查询 `source_id`。URL path 从 `/orders/{order_key}` 改为 `/orders/{order_id}`。

**Step 4: Cloud PostgreSQL migration**

需要创建 migration：
- 删除 `store_archived_orders.order_key` 列
- 更新 UNIQUE constraint 从 `(tenant_id, store_id, order_key)` 改为 `(tenant_id, store_id, source_id)`
- `store_credit_notes.original_order_key` → `original_order_id` (BIGINT)

**Step 5: 验证编译**

Run: `cargo check -p crab-cloud`

---

### Task 13: edge-server tests 修复

**Files:**
- Modify: `edge-server/src/orders/manager/tests/` — 所有 6 个测试文件
- Modify: `edge-server/src/orders/storage.rs` — 底部测试模块 (~line 982+)
- Modify: `edge-server/src/order_money/tests.rs`

**Step 1: storage.rs 测试**

测试中的 UUID 字面量改为 snowflake:
```rust
// was: event_id: uuid::Uuid::new_v4().to_string()
event_id: snowflake_id(),
```

**Step 2: manager tests**

搜索所有 `order_id:` 赋值，从 `"test-order-1".to_string()` 等改为 `snowflake_id()` 或固定 i64 值如 `100001`。

**Step 3: 运行测试**

Run: `cargo test -p edge-server --lib 2>&1 | tail -30`

---

### Task 14: shared tests 修复

**Files:**
- Modify: `shared/src/cloud/sync.rs` — 测试中的 UUID → snowflake
- Modify: `shared/src/cloud/ws.rs` — 测试中的 UUID → snowflake
- Modify: `shared/src/order/canonical.rs` — canonical hash 测试（hash 值会变）

**Step 1: 修改 sync.rs 测试**

将所有 `"uuid-order-001"`, `"uuid"` 等测试值改为 i64 snowflake 值。

**Step 2: 修改 ws.rs 测试**

同上。

**Step 3: canonical hash 测试**

Hash 值因 write_i64 vs write_str 而改变，更新期望的 hash 值。

**Step 4: 运行测试**

Run: `cargo test -p shared --lib`

---

### Task 15: 前端类型更新 — TypeScript

**Files:**
- Modify: `red_coral/src/core/domain/types/orderEvent.ts:69,73,83` — `string` → `number`
- Modify: `red_coral/src/core/domain/types/orderEvent.ts:763,766,896` — CommandResponse, OrderSnapshot
- Modify: `red_coral/src/core/domain/types/orderEvent.ts` — 所有 command payload variants 中的 `order_id: string` → `number`
- Modify: `red_coral/src/core/domain/types/orderEvent.ts` — EventPayload variants 中 `payment_id: string` → `number`
- Modify: `red_coral/src/core/domain/types/archivedOrder.ts:118` — 删除 `order_key: string`
- Modify: `red_coral/src/core/domain/types/api/models.ts:839,858` — `order_id: string` → `number`

**Step 1: OrderEvent interface**

```typescript
export interface OrderEvent {
  event_id: number;      // was string
  sequence: number;
  order_id: number;      // was string
  timestamp: number;
  client_timestamp?: number | null;
  operator_id: number;
  operator_name: string;
  command_id: number;    // was string
  event_type: OrderEventType;
  payload: EventPayload;
}
```

**Step 2: OrderSnapshot interface**

```typescript
export interface OrderSnapshot {
  order_id: number;      // was string
  // ... rest same
}
```

**Step 3: CommandResponse**

```typescript
export interface CommandResponse {
  command_id: number;    // was string
  success: boolean;
  order_id?: number | null;  // was string | null
  error?: CommandError | null;
}
```

**Step 4: 所有 command payload variants**

搜索所有 `order_id: string` → `order_id: number`。
搜索所有 `payment_id: string` → `payment_id: number`。

**Step 5: ArchivedOrder**

删除 `order_key: string` 字段。

**Step 6: 验证编译**

Run: `cd red_coral && npx tsc --noEmit 2>&1 | head -50`

---

### Task 16: 前端组件修复 — 逐一修复 TS 编译错误

**Files:**
- Modify: `red_coral/src-tauri/src/commands/order_es.rs:78` — `order_id: String` → `i64`
- Modify: `red_coral/src/core/stores/order/` — order_id 类型使用
- Modify: `red_coral/src/core/hooks/useOrderEventListener.ts` — order_id 比较
- Modify: `red_coral/src/screens/` — 任何使用 order_id/order_key 的页面
- Modify: `red_coral/src/hooks/useHistoryOrderDetail.ts` — order_key 引用

**Step 1: Tauri commands**

`commands/order_es.rs:78`: `order_id: String` → `order_id: i64`

**Step 2: 运行 tsc 并逐一修复**

Run: `cd red_coral && npx tsc --noEmit 2>&1`

关键模式：
- `order_id` 比较: `=== "somestring"` → `=== someNumber`
- `order_key` 引用 → 删除或改为 `order_id`
- `toString()` on order_id → 不再需要（直接 number）
- Map key: `Map<string, OrderSnapshot>` → `Map<number, OrderSnapshot>`

**Step 3: 验证编译**

Run: `cd red_coral && npx tsc --noEmit`
Expected: 零错误

---

### Task 17: 移除 uuid crate 依赖

**Files:**
- Modify: `shared/Cargo.toml` — 如果 shared 依赖 uuid，移除
- Modify: `edge-server/Cargo.toml` — 评估是否可移除 uuid（tcp_server 和 event_router 可能仍需要）
- Modify: `Cargo.toml` (workspace) — 如果 uuid 不再被任何 crate 使用

**Step 1: 检查 uuid 使用**

搜索所有剩余的 `uuid::Uuid` 使用。非 ID 用途（如 epoch、client_name、request_id）可以改用 snowflake。

**Step 2: 尽可能移除**

如果 `crab-cloud` 仍需要 uuid（refresh_token、activation 等场景），仅移除 `shared` 和 `edge-server` 的 uuid 依赖。

**Step 3: 验证编译**

Run: `cargo check --workspace`

---

### Task 18: 全面验证

**Step 1: Rust workspace**

```bash
cargo check --workspace
cargo clippy --workspace
cargo test --workspace --lib
```

**Step 2: TypeScript**

```bash
cd red_coral && npx tsc --noEmit
```

**Step 3: 清理**

- 移除所有 `#[allow(unused)]` 等临时注解
- 确认无 clippy warnings
- 确认无 TS errors

---

## 执行顺序约束

```
Task 1 (shared 订单类型) ──┬── Task 2 (canonical hash)
                           └── Task 3 (系统状态+同步协议)
                                  │
Task 4 (redb 存储) ───────────────┤
                                  │
Task 5 (OrdersManager) ──────────┤
                                  │
Task 6 (CommandHandler actions) ──┤
                                  │
Task 7 (broadcast_sync 拆分) ─────┤
                                  │
Task 8 (归档 order_key 删除) ─────┤
                                  │
Task 9 (cloud worker) ────────────┤
                                  │
Task 10 (其他模块) ───────────────┤
                                  │
Task 11 (SQLite migration) ───────┤
                                  │
Task 12 (crab-cloud) ─────────────┤
                                  │
Task 13 (edge-server tests) ──────┤
                                  │
Task 14 (shared tests) ───────────┤
                                  │
Task 15 (前端类型) ───────────────┤
                                  │
Task 16 (前端组件) ───────────────┤
                                  │
Task 17 (uuid 清理) ──────────────┘
                                  │
Task 18 (全面验证) ───────────────┘
```

前 3 个 Task 是基础层，必须先完成。之后 Task 4-16 按照自然编译依赖顺序执行。

## 注意事项

1. **redb 数据不兼容**: 表键类型变更后旧数据无法读取，需要清空 `orders.redb`。开发阶段可接受。
2. **Hash 链不兼容**: canonical hash 变更后旧 hash 无效。需要重新归档或清空数据库。
3. **instance_id 不变**: content-addressed hash 保持 String，只有 split suffix 从 UUID 改为 snowflake。
4. **epoch 保持 String**: server/manager epoch 仅用于日志标识，不是实体 ID，可以用 snowflake.to_string() 或保持任意字符串。
5. **crab-cloud uuid**: 云端仍可能需要 uuid crate（refresh_token、activation 等），不强制移除。
