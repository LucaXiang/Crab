# BackgroundTasks 生命周期全面修复

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 消除 edge-server 后台任务孤儿泄露问题，确保 `exit_tenant` 能可靠释放所有资源（包括 redb 文件锁）。

**Architecture:** `BackgroundTasks` 增加 `AbortHandle` 追踪和 `Drop` 安全网；所有后台任务统一使用 `BackgroundTasks` 提供的 shutdown token；`Server::run()` 统一清理路径；`await_mode_shutdown` 超时减到 3s（因为任务本身已可靠退出）。

**Tech Stack:** Rust, tokio (JoinHandle/AbortHandle/CancellationToken), redb, edge-server

---

## 问题根因

`exit_tenant` → `shutdown_token.cancel()` → `Server::run()` 调 `background_tasks.shutdown()` → 4/8 任务不响应 → 卡 10s 超时 → abort server_task → **4 个后台任务变孤儿（JoinHandle drop = detach）** → 持有 `Arc<redb::Database>` → 重入租户时文件锁冲突。

---

### Task 1: `BackgroundTasks` 加 AbortHandle 追踪 + Drop 安全网

**Files:**
- Modify: `edge-server/src/core/tasks.rs`

**Step 1: 修改 `RegisteredTask` 和 `BackgroundTasks` 结构体**

在 `RegisteredTask` 中增加 `abort_handle` 字段，`spawn()` 时保存：

```rust
// edge-server/src/core/tasks.rs

use tokio::task::{JoinHandle, AbortHandle};

struct RegisteredTask {
    name: &'static str,
    kind: TaskKind,
    handle: JoinHandle<()>,
    abort_handle: AbortHandle,  // 新增
}
```

**Step 2: 修改 `spawn()` 保存 AbortHandle**

```rust
pub fn spawn<F>(&mut self, name: &'static str, kind: TaskKind, future: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    // ... 现有的 wrapped_future 不变 ...

    let handle = tokio::spawn(wrapped_future);
    let abort_handle = handle.abort_handle();  // 新增
    tracing::debug!(task = %name, kind = %kind, "Registered background task");
    self.tasks.push(RegisteredTask { name, kind, handle, abort_handle });
}
```

**Step 3: 增加 `Drop` impl — 安全网**

```rust
impl Drop for BackgroundTasks {
    fn drop(&mut self) {
        self.shutdown.cancel();
        let count = self.tasks.len();
        for task in &self.tasks {
            task.abort_handle.abort();
        }
        if count > 0 {
            tracing::warn!(count, "BackgroundTasks dropped — all tasks force-aborted");
        }
    }
}
```

**Step 4: `shutdown()` 加超时 — 不再无限等待**

替换现有的 `shutdown()` 方法：

```rust
/// Graceful shutdown: cancel → 等最多 5s → 超时则 abort 残留任务
pub async fn shutdown(mut self) {
    tracing::info!("Shutting down {} background tasks...", self.tasks.len());

    // 1. 发送取消信号
    self.shutdown.cancel();

    // 2. 带超时地等待所有任务
    let tasks = std::mem::take(&mut self.tasks);
    let deadline = std::time::Duration::from_secs(5);

    match tokio::time::timeout(deadline, Self::await_all(tasks)).await {
        Ok(()) => {
            tracing::info!("All background tasks stopped gracefully");
        }
        Err(_) => {
            tracing::warn!(
                "Background tasks shutdown timed out after 5s — Drop will abort remaining"
            );
            // Drop 会 abort 残留任务（但 tasks 已被 take，所以这里无需额外处理）
            // 注意：已被 take 的 tasks 在 await_all 中已经 await 完成的部分
        }
    }
}

async fn await_all(tasks: Vec<RegisteredTask>) {
    for task in tasks {
        match task.handle.await {
            Ok(()) => tracing::debug!(task = %task.name, "Task completed"),
            Err(e) if e.is_cancelled() => {
                tracing::debug!(task = %task.name, "Task cancelled")
            }
            Err(e) => {
                tracing::error!(task = %task.name, error = ?e, "Task panicked")
            }
        }
    }
}
```

注意：`shutdown()` takes `mut self`，所以 `Drop` 不会在 timeout 场景重复 abort（tasks 已被 `std::mem::take` 移走）。但如果 `BackgroundTasks` 未调用 `shutdown()` 就被 drop（比如 server_task 被 abort 时），`Drop` 会 abort 所有任务。

**Step 5: 验证编译**

Run: `cargo check -p edge-server`
Expected: PASS (无需修改调用方，API 兼容)

**Step 6: Commit**

```bash
git add edge-server/src/core/tasks.rs
git commit -m "fix(edge-server): BackgroundTasks 增加 AbortHandle 追踪和 Drop 安全网

- spawn() 保存每个任务的 AbortHandle
- Drop impl: cancel + abort 所有任务，防止孤儿泄露
- shutdown() 增加 5s 超时，不再无限等待"
```

---

### Task 2: `EventRouter::run()` 增加 shutdown 支持

**Files:**
- Modify: `edge-server/src/core/event_router.rs:77-98`
- Modify: `edge-server/src/core/state.rs:349-355` (注册调用处)

**Step 1: 修改 `EventRouter::run()` 签名和实现**

增加 `shutdown: CancellationToken` 参数，在 loop 中 `select!`：

```rust
/// 运行路由器（阻塞直到关闭信号或源通道关闭）
pub async fn run(
    self,
    mut source: broadcast::Receiver<OrderEvent>,
    shutdown: CancellationToken,
) {
    tracing::info!("Event router started");

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::info!("Event router received shutdown signal");
                break;
            }
            result = source.recv() => {
                match result {
                    Ok(event) => {
                        self.dispatch(event).await;
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::error!(
                            skipped = n,
                            "Event router lagged! Events skipped - archive data may be lost"
                        );
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!("Source channel closed, event router stopping");
                        break;
                    }
                }
            }
        }
    }
}
```

**Step 2: 更新 `start_background_tasks` 中的注册代码**

在 `edge-server/src/core/state.rs:349-355`，传入 shutdown token：

```rust
// 原:
tasks.spawn("event_router", TaskKind::Worker, async move {
    router.run(source_rx).await;
});

// 改:
let event_router_shutdown = tasks.shutdown_token();
tasks.spawn("event_router", TaskKind::Worker, async move {
    router.run(source_rx, event_router_shutdown).await;
});
```

**Step 3: 更新测试**

在 `event_router.rs` 的测试中，传入一个不会被取消的 token：

```rust
// 测试文件中，给 router.run 加上 CancellationToken::new()
// 例如 test_event_routing:
tokio::spawn(async move {
    router.run(rx, CancellationToken::new()).await;
});
```

同时添加 shutdown 测试：

```rust
#[tokio::test]
async fn test_shutdown() {
    let (router, _channels) = EventRouter::new(16, 16);
    let (_tx, rx) = broadcast::channel::<OrderEvent>(16);
    let shutdown = CancellationToken::new();
    let shutdown_clone = shutdown.clone();

    let handle = tokio::spawn(async move {
        router.run(rx, shutdown_clone).await;
    });

    // 应该在 cancel 后立即退出
    shutdown.cancel();
    tokio::time::timeout(std::time::Duration::from_millis(100), handle)
        .await
        .expect("EventRouter should exit within 100ms")
        .unwrap();
}
```

**Step 4: 验证**

Run: `cargo test -p edge-server --lib event_router`
Expected: PASS

**Step 5: Commit**

```bash
git add edge-server/src/core/event_router.rs edge-server/src/core/state.rs
git commit -m "fix(edge-server): EventRouter 增加 shutdown token 支持

EventRouter::run() 现在通过 tokio::select! 监听 CancellationToken，
收到关闭信号后立即退出。"
```

---

### Task 3: `order_sync_forwarder` 增加 shutdown 支持

**Files:**
- Modify: `edge-server/src/core/state.rs:573-617` (register_order_sync_forwarder)

**Step 1: 在闭包中加入 shutdown token 的 `select!`**

```rust
fn register_order_sync_forwarder(
    &self,
    tasks: &mut BackgroundTasks,
    mut event_rx: mpsc::Receiver<std::sync::Arc<shared::order::OrderEvent>>,
) {
    let message_bus = self.message_bus.bus().clone();
    let orders_manager = self.orders_manager.clone();
    let shutdown = tasks.shutdown_token();  // 新增

    tasks.spawn("order_sync_forwarder", TaskKind::Listener, async move {
        tracing::debug!("Order sync forwarder started");

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    tracing::info!("Order sync forwarder received shutdown signal");
                    break;
                }
                event = event_rx.recv() => {
                    let Some(event) = event else {
                        tracing::debug!("Sync channel closed, order sync forwarder stopping");
                        break;
                    };

                    let order_id = event.order_id.clone();
                    let sequence = event.sequence;
                    let action = event.event_type.to_string();

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
                                })
                                .into(),
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
            }
        }
    });
}
```

**Step 2: 验证编译**

Run: `cargo check -p edge-server`
Expected: PASS

**Step 3: Commit**

```bash
git add edge-server/src/core/state.rs
git commit -m "fix(edge-server): order_sync_forwarder 增加 shutdown token 支持"
```

---

### Task 4: `KitchenPrintWorker::run()` 增加 shutdown 支持

**Files:**
- Modify: `edge-server/src/printing/worker.rs:48-58`
- Modify: `edge-server/src/core/state.rs:623-640` (register_kitchen_print_worker)

**Step 1: 修改 `KitchenPrintWorker::run()` 签名**

```rust
/// 运行工作者（阻塞直到关闭信号或通道关闭）
pub async fn run(
    self,
    mut event_rx: mpsc::Receiver<ArcOrderEvent>,
    shutdown: CancellationToken,
) {
    tracing::info!("Kitchen print worker started");
    let executor = PrintExecutor::new();

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::info!("Kitchen print worker received shutdown signal");
                break;
            }
            event = event_rx.recv() => {
                let Some(event) = event else {
                    tracing::info!("Print channel closed, kitchen print worker stopping");
                    break;
                };
                self.handle_items_added(&event, &executor).await;
            }
        }
    }
}
```

**Step 2: 更新注册代码**

在 `edge-server/src/core/state.rs` 的 `register_kitchen_print_worker` 中：

```rust
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
        self.pool.clone(),
    );

    let shutdown = tasks.shutdown_token();  // 新增
    tasks.spawn("kitchen_print_worker", TaskKind::Listener, async move {
        worker.run(event_rx, shutdown).await;  // 传入 shutdown
    });
}
```

**Step 3: 验证编译**

Run: `cargo check -p edge-server`
Expected: PASS

**Step 4: Commit**

```bash
git add edge-server/src/printing/worker.rs edge-server/src/core/state.rs
git commit -m "fix(edge-server): KitchenPrintWorker 增加 shutdown token 支持"
```

---

### Task 5: `MessageHandler` 使用 BackgroundTasks 的 shutdown token

**Files:**
- Modify: `edge-server/src/core/state.rs:553-568` (register_message_handler)

**问题:** `MessageHandler` 已经有 `select!` 监听 `shutdown_token`，但注册时传入了 `message_bus.bus().shutdown_token()` 而非 `BackgroundTasks` 的 token。

**Step 1: 修改注册代码**

```rust
fn register_message_handler(&self, tasks: &mut BackgroundTasks) {
    let handler_receiver = self.message_bus.bus().subscribe_to_clients();
    let handler_shutdown = tasks.shutdown_token();  // 改: 使用 BackgroundTasks 的 token
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
```

**Step 2: 验证编译**

Run: `cargo check -p edge-server`
Expected: PASS

**Step 3: Commit**

```bash
git add edge-server/src/core/state.rs
git commit -m "fix(edge-server): MessageHandler 使用 BackgroundTasks shutdown token

之前使用 message_bus.shutdown_token()，该 token 在 BackgroundTasks.shutdown()
时不会被 cancel，导致 MessageHandler 不响应关闭信号。"
```

---

### Task 6: `Server::run()` 统一清理路径

**Files:**
- Modify: `edge-server/src/core/server.rs:53-200`

**问题:** Phase 3/4/4.5 的 early return 缺少 `pool.close()`，与 Phase 7 的清理不一致。

**Step 1: 提取清理逻辑为独立方法**

在 `Server` impl 中添加：

```rust
/// 统一清理逻辑（所有退出路径共用）
async fn cleanup(state: ServerState, background_tasks: BackgroundTasks) {
    // 1. 审计关闭
    state.audit_service.on_shutdown();

    // 2. 停止所有后台任务
    background_tasks.shutdown().await;

    // 3. 关闭 SQLite 连接池
    state.pool.close().await;

    // 4. 等待审计 worker drain
    let audit_handle = state.audit_worker_handle.lock().await.take();
    drop(state);
    if let Some(handle) = audit_handle
        && tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .is_err()
    {
        tracing::warn!("Audit worker drain timed out after 5s");
    }
}
```

**Step 2: 替换所有 early return 清理代码**

Phase 3 early return (原 line 73-77):
```rust
None => {
    tracing::info!("Shutdown requested during activation wait");
    Self::cleanup(state, background_tasks).await;
    return Ok(());
}
```

Phase 4 early return (原 line 91-95):
```rust
_ = self.shutdown_token.cancelled() => {
    tracing::info!("Shutdown requested during subscription check");
    Self::cleanup(state, background_tasks).await;
    return Ok(());
}
```

Phase 4.5 early return (类似模式):
```rust
_ = self.shutdown_token.cancelled() => {
    tracing::info!("Shutdown requested during P12 check");
    Self::cleanup(state, background_tasks).await;
    return Ok(());
}
```

Phase 7 normal shutdown (原 line 163-200): 替换为：
```rust
// Phase 7: Graceful shutdown
if let Err(e) = state
    .audit_service
    .log_sync(
        crate::audit::AuditAction::SystemShutdown,
        "system",
        "main",
        serde_json::json!({"epoch": &state.epoch}),
    )
    .await
{
    tracing::error!("Failed to log system shutdown: {:?}", e);
}

Self::cleanup(state, background_tasks).await;
```

**Step 3: 验证编译**

Run: `cargo check -p edge-server`
Expected: PASS

**Step 4: Commit**

```bash
git add edge-server/src/core/server.rs
git commit -m "fix(edge-server): Server::run() 统一所有退出路径的清理逻辑

提取 cleanup() 方法，所有 early return 和正常 shutdown 共用。
修复 Phase 3/4/4.5 缺少 pool.close() 的问题。"
```

---

### Task 7: `await_mode_shutdown` 降低超时（可选优化）

**Files:**
- Modify: `red_coral/src-tauri/src/core/bridge/lifecycle.rs:19`

**说明:** Task 1-6 修复后，所有后台任务都能在毫秒级响应 shutdown。`BackgroundTasks::shutdown()` 自身有 5s 超时。所以 `await_mode_shutdown` 的 10s 超时可以降低为 3s（覆盖 edge case）。

**Step 1: 修改超时时间**

```rust
// 原: std::time::Duration::from_secs(10)
// 改:
match tokio::time::timeout(std::time::Duration::from_secs(3), async {
```

**Step 2: 验证编译**

Run: `cargo check -p red_coral_lib` (或 `cargo check --workspace`)
Expected: PASS

**Step 3: Commit**

```bash
git add red_coral/src-tauri/src/core/bridge/lifecycle.rs
git commit -m "refactor: await_mode_shutdown 超时从 10s 降至 3s

所有后台任务现已正确响应 shutdown token，无需长时间等待。"
```

---

### Task 8: 全量验证

**Step 1: workspace 编译检查**

Run: `cargo check --workspace`
Expected: PASS (零错误)

**Step 2: clippy 检查**

Run: `cargo clippy --workspace`
Expected: PASS (零警告)

**Step 3: 运行测试**

Run: `cargo test --workspace --lib`
Expected: PASS

**Step 4: TypeScript 类型检查**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

**Step 5: 手动集成测试**

1. 启动 Tauri dev: `cd red_coral && npm run tauri:dev`
2. 进入一个订阅已取消的租户（触发 SubscriptionBlocked 页面）
3. 点击退出租户 → **应在 1-2 秒内完成**（不再卡 10s）
4. 重新进入同一租户 → **应成功启动**（不再报 "Database already open"）
5. 检查日志：应看到 "All background tasks stopped gracefully"（8/8 任务都正常退出）

**Step 6: Final commit**

```bash
git add -A
git commit -m "test: 验证 BackgroundTasks 生命周期修复

- workspace 编译零错误
- clippy 零警告
- 所有测试通过
- 手动验证 exit_tenant 快速且 redb 无锁冲突"
```
