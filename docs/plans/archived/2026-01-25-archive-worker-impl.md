# Archive Worker 实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将订单归档流程从 OrderManager 解耦到独立的 ArchiveWorker，使用 redb 持久化队列保证数据一致性。

**Architecture:** OrderManager 在事务内将终结订单入队，ArchiveWorker 异步消费队列并归档到 SurrealDB，失败时支持重试。

**Tech Stack:** Rust, redb, tokio, SurrealDB

---

## Task 1: 扩展 OrderStorage - 添加 pending_archive 表

**Files:**
- Modify: `edge-server/src/orders/storage.rs`

**Step 1: 添加表定义和类型**

在 `storage.rs` 顶部添加：

```rust
/// Table for pending archive queue: key = order_id, value = JSON-serialized PendingArchive
const PENDING_ARCHIVE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("pending_archive");

/// Pending archive entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PendingArchive {
    pub order_id: String,
    pub created_at: i64,
    pub retry_count: u32,
    pub last_error: Option<String>,
}
```

**Step 2: 在 open() 中初始化表**

在 `open()` 函数的表初始化块中添加：

```rust
let _ = write_txn.open_table(PENDING_ARCHIVE_TABLE)?;
```

**Step 3: 在 open_in_memory() 中初始化表（测试用）**

同样添加表初始化。

**Step 4: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译通过

**Step 5: Commit**

```bash
git add edge-server/src/orders/storage.rs
git commit -m "feat(storage): add pending_archive table definition"
```

---

## Task 2: 实现 Storage 队列操作方法

**Files:**
- Modify: `edge-server/src/orders/storage.rs`

**Step 1: 添加 queue_for_archive 方法**

```rust
// ========== Pending Archive Queue ==========

/// Add order to archive queue (within transaction)
pub fn queue_for_archive(&self, txn: &WriteTransaction, order_id: &str) -> StorageResult<()> {
    let mut table = txn.open_table(PENDING_ARCHIVE_TABLE)?;
    let pending = PendingArchive {
        order_id: order_id.to_string(),
        created_at: chrono::Utc::now().timestamp_millis(),
        retry_count: 0,
        last_error: None,
    };
    let value = serde_json::to_vec(&pending)?;
    table.insert(order_id, value.as_slice())?;
    Ok(())
}
```

**Step 2: 添加 get_pending_archives 方法**

```rust
/// Get all pending archive entries
pub fn get_pending_archives(&self) -> StorageResult<Vec<PendingArchive>> {
    let read_txn = self.db.begin_read()?;
    let table = read_txn.open_table(PENDING_ARCHIVE_TABLE)?;

    let mut entries = Vec::new();
    for result in table.iter()? {
        let (_key, value) = result?;
        let pending: PendingArchive = serde_json::from_slice(value.value())?;
        entries.push(pending);
    }
    Ok(entries)
}
```

**Step 3: 添加 complete_archive 方法**

```rust
/// Complete archive: remove from pending queue and cleanup order data
pub fn complete_archive(&self, order_id: &str) -> StorageResult<()> {
    let txn = self.begin_write()?;

    // 1. Remove from pending queue
    {
        let mut table = txn.open_table(PENDING_ARCHIVE_TABLE)?;
        table.remove(order_id)?;
    }

    // 2. Remove snapshot
    {
        let mut table = txn.open_table(SNAPSHOTS_TABLE)?;
        table.remove(order_id)?;
    }

    // 3. Remove events
    {
        let mut table = txn.open_table(EVENTS_TABLE)?;
        let range_start = (order_id, 0u64);
        let range_end = (order_id, u64::MAX);

        let mut keys_to_remove: Vec<(String, u64)> = Vec::new();
        for result in table.range(range_start..=range_end)? {
            let (key, _) = result?;
            let key_value = key.value();
            keys_to_remove.push((key_value.0.to_string(), key_value.1));
        }

        for (oid, seq) in &keys_to_remove {
            table.remove((oid.as_str(), *seq))?;
        }
    }

    txn.commit()?;
    tracing::debug!(order_id = %order_id, "Archive completed, cleaned up from redb");
    Ok(())
}
```

**Step 4: 添加 mark_archive_failed 方法**

```rust
/// Mark archive as failed, increment retry count
pub fn mark_archive_failed(&self, order_id: &str, error: &str) -> StorageResult<()> {
    let txn = self.begin_write()?;
    let mut table = txn.open_table(PENDING_ARCHIVE_TABLE)?;

    if let Some(value) = table.get(order_id)? {
        let mut pending: PendingArchive = serde_json::from_slice(value.value())?;
        pending.retry_count += 1;
        pending.last_error = Some(error.to_string());
        let new_value = serde_json::to_vec(&pending)?;
        table.insert(order_id, new_value.as_slice())?;
    }

    txn.commit()?;
    Ok(())
}
```

**Step 5: 添加 remove_from_pending 方法（用于超过重试次数时清理）**

```rust
/// Remove from pending queue without cleanup (for dead letter)
pub fn remove_from_pending(&self, order_id: &str) -> StorageResult<()> {
    let txn = self.begin_write()?;
    {
        let mut table = txn.open_table(PENDING_ARCHIVE_TABLE)?;
        table.remove(order_id)?;
    }
    txn.commit()?;
    Ok(())
}
```

**Step 6: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译通过

**Step 7: Commit**

```bash
git add edge-server/src/orders/storage.rs
git commit -m "feat(storage): add pending archive queue operations"
```

---

## Task 3: 创建 ArchiveWorker 模块

**Files:**
- Create: `edge-server/src/orders/archive_worker.rs`
- Modify: `edge-server/src/orders/mod.rs`

**Step 1: 创建 archive_worker.rs**

```rust
//! Archive Worker - Processes pending archive queue
//!
//! Listens for terminal events and processes archive queue with retry logic.

use super::archive::OrderArchiveService;
use super::storage::{OrderStorage, PendingArchive};
use shared::order::{OrderEvent, OrderEventType};
use std::time::Duration;
use tokio::sync::broadcast;

/// Terminal event types that trigger archiving
const TERMINAL_EVENT_TYPES: &[OrderEventType] = &[
    OrderEventType::OrderCompleted,
    OrderEventType::OrderVoided,
    OrderEventType::OrderMoved,
    OrderEventType::OrderMerged,
];

/// Archive worker configuration
const MAX_RETRY_COUNT: u32 = 10;
const RETRY_BASE_DELAY_SECS: u64 = 5;
const RETRY_MAX_DELAY_SECS: u64 = 3600; // 1 hour
const QUEUE_SCAN_INTERVAL_SECS: u64 = 60;

/// Worker for processing archive queue
pub struct ArchiveWorker {
    storage: OrderStorage,
    archive_service: OrderArchiveService,
}

impl ArchiveWorker {
    pub fn new(storage: OrderStorage, archive_service: OrderArchiveService) -> Self {
        Self {
            storage,
            archive_service,
        }
    }

    /// Run the archive worker
    pub async fn run(self, mut event_rx: broadcast::Receiver<OrderEvent>) {
        tracing::info!("ArchiveWorker started");

        // Process any pending archives from previous run
        self.process_pending_queue().await;

        let mut scan_interval = tokio::time::interval(Duration::from_secs(QUEUE_SCAN_INTERVAL_SECS));

        loop {
            tokio::select! {
                // Handle new terminal events
                result = event_rx.recv() => {
                    match result {
                        Ok(event) if TERMINAL_EVENT_TYPES.contains(&event.event_type) => {
                            tracing::debug!(order_id = %event.order_id, event_type = ?event.event_type, "Received terminal event");
                            self.process_order(&event.order_id).await;
                        }
                        Ok(_) => {} // Ignore non-terminal events
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!(skipped = n, "Event receiver lagged, processing queue");
                            self.process_pending_queue().await;
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::info!("Event channel closed, shutting down");
                            break;
                        }
                    }
                }
                // Periodic queue scan for retries
                _ = scan_interval.tick() => {
                    self.process_pending_queue().await;
                }
            }
        }
    }

    /// Process all pending archives
    async fn process_pending_queue(&self) {
        let pending = match self.storage.get_pending_archives() {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(error = %e, "Failed to get pending archives");
                return;
            }
        };

        if pending.is_empty() {
            return;
        }

        tracing::info!(count = pending.len(), "Processing pending archive queue");

        for entry in pending {
            if self.should_retry(&entry) {
                self.process_order(&entry.order_id).await;
            }
        }
    }

    /// Check if entry should be retried based on backoff
    fn should_retry(&self, entry: &PendingArchive) -> bool {
        if entry.retry_count >= MAX_RETRY_COUNT {
            tracing::error!(
                order_id = %entry.order_id,
                retry_count = entry.retry_count,
                last_error = ?entry.last_error,
                "Max retry count exceeded, giving up"
            );
            // TODO: Move to dead letter queue or alert
            let _ = self.storage.remove_from_pending(&entry.order_id);
            return false;
        }

        // Exponential backoff
        let delay_secs = (RETRY_BASE_DELAY_SECS * 2u64.pow(entry.retry_count))
            .min(RETRY_MAX_DELAY_SECS);
        let next_retry_at = entry.created_at + (delay_secs as i64 * 1000);
        let now = chrono::Utc::now().timestamp_millis();

        now >= next_retry_at
    }

    /// Process a single order archive
    async fn process_order(&self, order_id: &str) {
        // 1. Load snapshot and events from redb
        let snapshot = match self.storage.get_snapshot(order_id) {
            Ok(Some(s)) => s,
            Ok(None) => {
                tracing::warn!(order_id = %order_id, "Snapshot not found, removing from queue");
                let _ = self.storage.remove_from_pending(order_id);
                return;
            }
            Err(e) => {
                tracing::error!(order_id = %order_id, error = %e, "Failed to load snapshot");
                return;
            }
        };

        let events = match self.storage.get_events_for_order(order_id) {
            Ok(e) => e,
            Err(e) => {
                tracing::error!(order_id = %order_id, error = %e, "Failed to load events");
                return;
            }
        };

        // 2. Archive to SurrealDB
        match self.archive_service.archive_order(&snapshot, events).await {
            Ok(()) => {
                tracing::info!(order_id = %order_id, "Order archived successfully");
                // 3. Cleanup redb
                if let Err(e) = self.storage.complete_archive(order_id) {
                    tracing::error!(order_id = %order_id, error = %e, "Failed to complete archive cleanup");
                }
            }
            Err(e) => {
                tracing::error!(order_id = %order_id, error = %e, "Archive failed");
                if let Err(e2) = self.storage.mark_archive_failed(order_id, &e.to_string()) {
                    tracing::error!(order_id = %order_id, error = %e2, "Failed to mark archive failed");
                }
            }
        }
    }
}
```

**Step 2: 在 mod.rs 中导出**

在 `edge-server/src/orders/mod.rs` 添加：

```rust
mod archive_worker;
pub use archive_worker::ArchiveWorker;
```

**Step 3: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译通过

**Step 4: Commit**

```bash
git add edge-server/src/orders/archive_worker.rs edge-server/src/orders/mod.rs
git commit -m "feat(orders): add ArchiveWorker module"
```

---

## Task 4: 修改 OrderManager - 移除 spawn，改用队列

**Files:**
- Modify: `edge-server/src/orders/manager.rs`

**Step 1: 移除 TERMINAL_EVENT_TYPES（已移到 archive_worker）**

删除 manager.rs 顶部的：
```rust
const TERMINAL_EVENT_TYPES: &[shared::order::OrderEventType] = &[...];
```

**Step 2: 修改 process_command 中的归档逻辑**

找到步骤 8（Persist snapshots）中的归档检查，改为入队：

```rust
// 8. Persist snapshots and update active order tracking
for snapshot in ctx.modified_snapshots() {
    self.storage.store_snapshot(&txn, snapshot)?;

    match snapshot.status {
        OrderStatus::Active => {
            self.storage.mark_order_active(&txn, &snapshot.order_id)?;
        }
        OrderStatus::Completed
        | OrderStatus::Void
        | OrderStatus::Merged
        | OrderStatus::Moved => {
            self.storage.mark_order_inactive(&txn, &snapshot.order_id)?;
            // Queue for archive (atomic with transaction)
            if self.archive_service.is_some() {
                self.storage.queue_for_archive(&txn, &snapshot.order_id)?;
            }
        }
    }
}
```

**Step 3: 移除 tokio::spawn 归档代码**

删除步骤 13 的整个 spawn 块：
```rust
// 删除这整段代码
// 13. Archive to SurrealDB if terminal event
if let (Some(archive_service), Some(snapshot)) = (&self.archive_service, archive_snapshot) {
    // ... 整个 tokio::spawn 块
}
```

**Step 4: 移除 archive_snapshot 变量声明和收集**

删除：
```rust
let mut archive_snapshot: Option<OrderSnapshot> = None;
```

和收集逻辑：
```rust
if events.iter().any(...) {
    archive_snapshot = Some(snapshot.clone());
}
```

**Step 5: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译通过

**Step 6: 运行测试**

Run: `cargo test -p edge-server --lib`
Expected: 所有测试通过

**Step 7: Commit**

```bash
git add edge-server/src/orders/manager.rs
git commit -m "refactor(manager): replace spawn archive with queue_for_archive"
```

---

## Task 5: 集成 ArchiveWorker 到 Server

**Files:**
- Modify: `edge-server/src/core/server.rs` 或启动入口

**Step 1: 查找 Server 启动代码**

找到 OrderManager 初始化的位置。

**Step 2: 启动 ArchiveWorker**

在 OrderManager 设置 archive_service 之后：

```rust
// Start ArchiveWorker if archive service is available
if let Some(ref archive_svc) = manager.archive_service {
    let worker = ArchiveWorker::new(
        manager.storage().clone(),
        archive_svc.clone(),
    );
    let event_rx = manager.subscribe();
    tokio::spawn(async move {
        worker.run(event_rx).await;
    });
    tracing::info!("ArchiveWorker started");
}
```

**Step 3: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译通过

**Step 4: Commit**

```bash
git add edge-server/src/core/server.rs
git commit -m "feat(server): start ArchiveWorker on server init"
```

---

## Task 6: 添加单元测试

**Files:**
- Modify: `edge-server/src/orders/storage.rs` (tests module)
- Modify: `edge-server/src/orders/archive_worker.rs` (tests module)

**Step 1: 添加 storage 队列测试**

```rust
#[test]
fn test_pending_archive_queue() {
    let storage = OrderStorage::open_in_memory().unwrap();
    let order_id = "order-1";

    // Queue for archive
    let txn = storage.begin_write().unwrap();
    storage.queue_for_archive(&txn, order_id).unwrap();
    txn.commit().unwrap();

    // Check pending
    let pending = storage.get_pending_archives().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].order_id, order_id);
    assert_eq!(pending[0].retry_count, 0);

    // Mark failed
    storage.mark_archive_failed(order_id, "test error").unwrap();
    let pending = storage.get_pending_archives().unwrap();
    assert_eq!(pending[0].retry_count, 1);
    assert_eq!(pending[0].last_error, Some("test error".to_string()));

    // Remove from pending
    storage.remove_from_pending(order_id).unwrap();
    let pending = storage.get_pending_archives().unwrap();
    assert!(pending.is_empty());
}
```

**Step 2: 运行测试**

Run: `cargo test -p edge-server --lib`
Expected: 所有测试通过

**Step 3: Commit**

```bash
git add edge-server/src/orders/storage.rs
git commit -m "test(storage): add pending archive queue tests"
```

---

## Task 7: 清理废弃代码

**Files:**
- Modify: `edge-server/src/orders/storage.rs`
- Modify: `edge-server/src/orders/archive.rs`

**Step 1: 移除 storage.rs 中的 cleanup_archived_order**

这个方法现在被 `complete_archive` 替代，可以删除。

**Step 2: 移除 archive.rs 中的重试逻辑**

ArchiveWorker 现在处理重试，archive.rs 的 `archive_order` 可以简化为单次尝试。

删除：
- `MAX_RETRY_ATTEMPTS`, `RETRY_BASE_DELAY_MS` 常量
- `archive_semaphore` 字段（并发控制移到 Worker）
- 重试循环

**Step 3: 验证编译和测试**

Run: `cargo check -p edge-server && cargo test -p edge-server --lib`
Expected: 编译通过，测试通过

**Step 4: Commit**

```bash
git add edge-server/src/orders/storage.rs edge-server/src/orders/archive.rs
git commit -m "refactor: cleanup deprecated archive code"
```

---

## 验证清单

- [ ] `cargo check -p edge-server` 通过
- [ ] `cargo test -p edge-server --lib` 通过
- [ ] `cargo clippy -p edge-server` 无警告
- [ ] 手动测试：完成订单后检查 SurrealDB 归档
- [ ] 手动测试：模拟归档失败，检查重试机制
