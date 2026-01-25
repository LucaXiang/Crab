# Archive Worker 解耦设计

> **日期:** 2026-01-25
> **状态:** 已批准

## 问题背景

当前 `OrderManager` 在处理终结事件（CompleteOrder/VoidOrder）时，使用 `tokio::spawn` 异步执行归档：

```rust
tokio::spawn(async move {
    if let Err(e) = archive_svc.archive_order(&snapshot, order_events).await {
        tracing::error!(...);  // 只记录日志，无恢复机制
    }
});
```

**问题：**
1. 归档失败后无重试队列
2. 订单已从 `active_orders` 移除，但未进入 SurrealDB
3. 数据处于"悬空"状态，无法查询

## 解决方案

使用 redb 持久化队列 + 独立 ArchiveWorker 解耦归档流程。

### 架构

```
┌─────────────────────────────────────────┐
│ OrderManager                            │
│  process_command()                      │
│     ├─> queue_for_archive() (原子)      │
│     └─> broadcast event                 │
└───────────────┬─────────────────────────┘
                │ OrderEvent (terminal)
                ▼
┌─────────────────────────────────────────┐
│ ArchiveWorker                           │
│  • 监听事件广播 → 立即处理              │
│  • 定时扫描队列 → 处理重试              │
│  • 成功 → complete_archive()            │
│  • 失败 → mark_archive_failed()         │
└─────────────────────────────────────────┘
```

### 存储设计

新增 redb 表：

| 表名 | Key | Value | 用途 |
|------|-----|-------|------|
| `pending_archive` | `order_id` | `PendingArchive` | 待归档队列 |

```rust
#[derive(Serialize, Deserialize)]
pub struct PendingArchive {
    pub order_id: String,
    pub created_at: i64,
    pub retry_count: u32,
    pub last_error: Option<String>,
}
```

### 工作流

**OrderManager (同步):**
1. 处理命令，生成事件
2. 更新 snapshot，标记 inactive
3. **原子入队** `queue_for_archive()`
4. 提交事务
5. 广播事件

**ArchiveWorker (异步):**
1. 收到事件 → 立即处理
2. 定时扫描 → 处理重试 (指数退避)
3. 成功 → `complete_archive()` 清理 redb
4. 失败 → `mark_archive_failed()` 更新状态

### 重试策略

- 最大重试次数: 10
- 退避策略: `min(base_delay * 2^retry_count, max_delay)`
- base_delay: 5 秒
- max_delay: 1 小时

### 启动恢复

ArchiveWorker 启动时自动扫描 `pending_archive` 表，恢复未完成的归档任务。

## 文件变更

| 文件 | 变更 |
|------|------|
| `edge-server/src/orders/storage.rs` | 新增 `pending_archive` 表和相关方法 |
| `edge-server/src/orders/archive_worker.rs` | 新增 ArchiveWorker 模块 |
| `edge-server/src/orders/manager.rs` | 移除 spawn，改用 queue_for_archive |
| `edge-server/src/orders/mod.rs` | 导出 ArchiveWorker |
| `edge-server/src/core/server.rs` | 启动 ArchiveWorker |

## 优势

1. **数据一致性**: 队列入队与事务原子提交
2. **可恢复**: 重启后自动恢复未完成任务
3. **可观测**: 队列长度/失败次数可暴露为 metrics
4. **解耦**: OrderManager 不知道归档实现细节
