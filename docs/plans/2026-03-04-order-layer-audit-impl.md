# 订单层审计修复 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 修复订单层审计中发现的 CRITICAL 和 HIGH 级别问题，按优先级分批执行。

**Architecture:** 从数据源头 (redb/edge-server) 向外修复，再到同步层和前端。每个 Task 独立可提交，不跨层混合修改。

**Tech Stack:** Rust (edge-server, shared), TypeScript/React (red_coral), SQLx (migrations), redb

**验证命令:**
- Rust: `cargo clippy --workspace`
- TypeScript: `cd red_coral && npx tsc --noEmit`

---

## Phase 1: P0 — 数据正确性 (4 tasks)

### Task 1: OrderMergedApplier f64→Decimal [F-001]

**Files:**
- Modify: `edge-server/src/orders/appliers/orders_merged.rs`

**Step 1: 修复 paid_amount 裸 f64 加法**

找到 `OrderMergedApplier::apply()` 中的:
```rust
snapshot.paid_amount += paid_amount;
```

替换为:
```rust
snapshot.paid_amount = to_f64(to_decimal(snapshot.paid_amount) + to_decimal(paid_amount));
```

确保文件顶部已导入 `use crate::order_money::{to_decimal, to_f64};`

同时检查同一 applier 中其他裸 f64 运算（如 `remaining_amount` 计算），统一改为 Decimal。

**Step 2: 验证编译**

Run: `cargo clippy --workspace`
Expected: 无新 warning/error

**Step 3: 提交**

```bash
git add edge-server/src/orders/appliers/orders_merged.rs
git commit -m "fix(orders): use Decimal for paid_amount in OrderMergedApplier"
```

---

### Task 2: next_daily_count 错误传播 [F-002]

**Files:**
- Modify: `edge-server/src/orders/manager/mod.rs`
- Modify: `edge-server/src/orders/storage.rs` (如果 unwrap_or 在此文件)

**Step 1: 找到所有 unwrap_or(1) 调用**

在 manager/mod.rs 和 storage.rs 中搜索 `unwrap_or(1)` 或 `unwrap_or(0)` 用于计数器/序号的场景。

将:
```rust
self.storage.next_daily_count(&date_str).unwrap_or(1)
```

改为:
```rust
self.storage.next_daily_count(&date_str)?
```

确保调用函数的返回类型为 `Result<...>`，错误能正确向上传播。若函数签名需要调整（如从不返回 Result 改为返回 Result），同步修改调用方。

**Step 2: 验证编译**

Run: `cargo clippy --workspace`

**Step 3: 提交**

```bash
git add edge-server/src/orders/manager/mod.rs edge-server/src/orders/storage.rs
git commit -m "fix(orders): propagate next_daily_count errors instead of silent fallback"
```

---

### Task 3: Huella 拒绝不阻塞发票同步队列 [Sync-4]

**Files:**
- Modify: `edge-server/src/cloud/worker.rs` (~sync_invoices_http 方法)
- Modify: `edge-server/src/db/repository/invoice.rs` (添加 mark_sync_error)

**Step 1: 添加 invoice sync_error 标记能力**

在 `edge-server/src/db/repository/invoice.rs` 添加:
```rust
pub async fn mark_invoice_sync_error(pool: &SqlitePool, invoice_id: i64, error: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE invoice SET cloud_synced = -1, sync_error = ?1 WHERE id = ?2"
    )
    .bind(error)
    .bind(invoice_id)
    .execute(pool)
    .await?;
    Ok(())
}
```

检查 invoice 表是否有 `sync_error` 列，如没有需要添加迁移:
```sql
ALTER TABLE invoice ADD COLUMN sync_error TEXT;
```

**Step 2: 修改 worker.rs 发票同步错误处理**

在 `sync_invoices_http` 中，当收到非 duplicate 的拒绝时，不 `break`，而是标记该发票为 sync_error 并继续:

将:
```rust
if !real_errors.is_empty() {
    tracing::warn!(..., "Invoice sync has non-duplicate rejections, stopping catch-up");
    break;
}
```

改为: 解析错误消息，识别 huella 拒绝，标记 sync_error，继续循环。对于非 huella 的真实错误仍然 break。

**Step 3: 修改 list_unsynced_ids 排除 sync_error**

确保 `list_unsynced_ids` 查询条件为 `cloud_synced = 0`（不包括 -1），这样被标记的发票不会反复被拉取。

**Step 4: 验证编译**

Run: `cargo clippy --workspace`

**Step 5: 提交**

```bash
git add edge-server/src/cloud/worker.rs edge-server/src/db/repository/invoice.rs
# 如有迁移文件也 add
git commit -m "fix(sync): mark huella-rejected invoices as sync_error instead of blocking queue"
```

---

### Task 4: RefundModal reason 发送枚举值 [FE-05]

**Files:**
- Modify: `red_coral/src/screens/History/RefundModal.tsx`

**Step 1: 修复 reason 字段**

找到:
```ts
reason: t(`credit_note.reason.${reason}`),
```

改为:
```ts
reason: reason,
```

确保 `reason` 变量存储的是原始枚举值（如 `'CUSTOMER_REQUEST'`），不是翻译后的文本。检查 reason 的赋值来源，确认 UI 展示时走 i18n 但提交时用原始值。

**Step 2: 验证编译**

Run: `cd red_coral && npx tsc --noEmit`

**Step 3: 提交**

```bash
git add red_coral/src/screens/History/RefundModal.tsx
git commit -m "fix(refund): send enum value for reason instead of translated text"
```

---

## Phase 2: P1 — HIGH 级别修复 (11 tasks)

### Task 5: recalculate_totals 总是调用 [F-004, F-005]

**Files:**
- Modify: `edge-server/src/orders/appliers/order_completed.rs`
- Modify: `edge-server/src/orders/appliers/payment_added.rs`

**Step 1: OrderCompletedApplier 添加 recalculate_totals**

在 `apply()` 末尾、`update_checksum()` 之前添加:
```rust
recalculate_totals(snapshot);
```

**Step 2: PaymentAddedApplier 移除条件判断**

将:
```rust
if snapshot.remaining_amount <= 0.0 {
    recalculate_totals(snapshot);
}
```

改为:
```rust
recalculate_totals(snapshot);
```

**Step 3: 验证 + 提交**

Run: `cargo clippy --workspace`

```bash
git add edge-server/src/orders/appliers/order_completed.rs edge-server/src/orders/appliers/payment_added.rs
git commit -m "fix(orders): always call recalculate_totals in OrderCompleted and PaymentAdded appliers"
```

---

### Task 6: 规则缓存写入合并到主事务 [F-006]

**Files:**
- Modify: `edge-server/src/orders/manager/mod.rs`
- Modify: `edge-server/src/orders/storage.rs` (如需修改 store_rule_snapshot 签名)

**Step 1: 分析当前 OpenTable 流程**

读取 manager/mod.rs 中 OpenTable 命令的处理流程，找到:
1. 主 redb 写事务 commit 的位置
2. `cache_rules` / `store_rule_snapshot` 的调用位置

**Step 2: 将 store_rule_snapshot 移入主事务**

确保 `store_rule_snapshot` 在 commit 之前调用，接受 `&WriteTransaction` 参数。commit 后只更新内存 cache。

**Step 3: 验证 + 提交**

Run: `cargo clippy --workspace`

```bash
git add edge-server/src/orders/manager/mod.rs edge-server/src/orders/storage.rs
git commit -m "fix(orders): atomically persist rule snapshot within OpenTable transaction"
```

---

### Task 7: 删除内层重试，统一外层控制 [H-2, A-1]

**Files:**
- Modify: `edge-server/src/archiving/service.rs`

**Step 1: 移除 archive_order 内部重试循环**

找到 `archive_order()` 中的重试循环 (retry 3 次 + sleep 退避)，简化为单次尝试。保留 semaphore 获取和释放。

将整个 retry loop 替换为单次调用 `archive_order_internal()`，错误直接返回给 ArchiveWorker。

**Step 2: 验证 + 提交**

Run: `cargo clippy --workspace`

```bash
git add edge-server/src/archiving/service.rs
git commit -m "refactor(archiving): remove inner retry loop, let ArchiveWorker handle all retries"
```

---

### Task 8: credit_note 事务内读 last_chain_hash [H-3]

**Files:**
- Modify: `edge-server/src/archiving/credit_note.rs`

**Step 1: 将 system_state 读取移入事务**

找到:
```rust
let system_state = system_state::get_or_create(&self.pool).await...
let prev_hash = system_state.last_chain_hash.unwrap_or_else(|| "genesis".to_string());
let mut tx = self.pool.begin().await...
```

改为:
```rust
let mut tx = self.pool.begin().await...
let prev_hash: String = sqlx::query_scalar(
    "SELECT COALESCE(last_chain_hash, 'genesis') FROM system_state WHERE id = 1"
)
.fetch_one(&mut *tx)
.await?;
```

参考 `anulacion.rs` 的实现模式。

**Step 2: 验证 + 提交**

Run: `cargo clippy --workspace`

```bash
git add edge-server/src/archiving/credit_note.rs
git commit -m "fix(credit_note): read last_chain_hash inside transaction for consistency"
```

---

### Task 9: 未知资源不标记已同步 [Sync-2]

**Files:**
- Modify: `edge-server/src/cloud/worker.rs`

**Step 1: 修复 send_catalog_changelog**

找到两处将未知资源/action 加入 `changelog_ids` 的代码:

```rust
Err(_) => {
    tracing::warn!(resource, "Unknown catalog_changelog resource, skipping");
    changelog_ids.push(*id);  // ← 删除此行
    continue;
}
```

和:
```rust
_ => {
    changelog_ids.push(*id);  // ← 删除此行
    continue;
}
```

删除这些 `changelog_ids.push(*id)` 行，让未知条目保持 unsynced 状态。

**Step 2: 验证 + 提交**

Run: `cargo clippy --workspace`

```bash
git add edge-server/src/cloud/worker.rs
git commit -m "fix(sync): don't mark unknown catalog resources as synced"
```

---

### Task 10: 重连添加 jitter [Sync-3]

**Files:**
- Modify: `edge-server/src/cloud/worker.rs`

**Step 1: 在退避延迟中添加随机抖动**

找到 reconnect delay sleep:
```rust
tokio::time::sleep(reconnect_delay).await;
```

改为:
```rust
use rand::Rng;
let jitter = rand::thread_rng().gen_range(0..=reconnect_delay.as_millis() as u64 / 2);
tokio::time::sleep(reconnect_delay + Duration::from_millis(jitter)).await;
```

检查 `rand` 是否已在 edge-server 的依赖中，否则需添加。

**Step 2: 验证 + 提交**

Run: `cargo clippy --workspace`

```bash
git add edge-server/src/cloud/worker.rs
# 如修改了 Cargo.toml 也 add
git commit -m "fix(sync): add jitter to reconnect backoff to prevent thundering herd"
```

---

### Task 11: 零售订单创建前 checkCommandLock [FE-02]

**Files:**
- Modify: `red_coral/src/hooks/useOrderHandlers.ts`

**Step 1: 在 handleCheckoutStart 开头添加检查**

在 `handleCheckoutStart` 函数开头（进入 retail 分支前）添加:
```ts
const lockResult = checkCommandLock();
if (!lockResult.canExecute) {
    toast.error(t('common.message.connection_required'));
    return;
}
```

确保 `checkCommandLock` 已导入（从 `sendCommand` 或相关模块）。

**Step 2: 验证 + 提交**

Run: `cd red_coral && npx tsc --noEmit`

```bash
git add red_coral/src/hooks/useOrderHandlers.ts
git commit -m "fix(checkout): check command lock before retail order creation"
```

---

### Task 12: currentOrderKey 统一为 order_id [FE-03]

**Files:**
- Modify: `red_coral/src/core/stores/order/useCheckoutStore.ts`
- Modify: `red_coral/src/hooks/useOrderHandlers.ts`
- Modify: 其他设置 `setCurrentOrderKey` 的文件

**Step 1: 分析所有 setCurrentOrderKey 调用点**

搜索 `setCurrentOrderKey` 的所有调用位置，找到传入 `table.id` 的地方（堂食流程），改为传入 `order_id`。

**Step 2: 简化 useCheckoutOrder 查找逻辑**

在 useCheckoutStore.ts 中，`useCheckoutOrder()` 只需:
```ts
const order = state.orders.get(currentOrderKey);
return order?.status === 'ACTIVE' ? order : null;
```

删除按 table_id 遍历的 fallback 路径。

**Step 3: 删除 checkoutOrder fallback 字段**

如果 `checkoutOrder` 字段仅用于 fallback（审计发现 FE-01），一并移除。

**Step 4: 验证 + 提交**

Run: `cd red_coral && npx tsc --noEmit`

```bash
git add red_coral/src/core/stores/order/useCheckoutStore.ts red_coral/src/hooks/useOrderHandlers.ts
# 添加其他修改的文件
git commit -m "refactor(checkout): unify currentOrderKey to always use order_id"
```

---

### Task 13: ItemSplitPage 使用 Currency 计算 [FE-04]

**Files:**
- Modify: `red_coral/src/screens/Checkout/payment/ItemSplitPage.tsx`

**Step 1: 替换 handleSplitPayment 中的浮点计算**

找到:
```ts
let total = 0;
itemsToSplit.forEach((splitItem) => {
  total += splitItem.unit_price * splitItem.quantity;
});
```

替换为使用已有的 `splitTotal` useMemo 值，或改用 Currency 类:
```ts
const total = splitTotal;  // 使用已有 useMemo
```

**Step 2: 验证 + 提交**

Run: `cd red_coral && npx tsc --noEmit`

```bash
git add red_coral/src/screens/Checkout/payment/ItemSplitPage.tsx
git commit -m "fix(checkout): use Currency-based splitTotal instead of raw float addition"
```

---

### Task 14: MergedOut applier 清零金额 [F-012]

**Files:**
- Modify: `edge-server/src/orders/appliers/orders_merged.rs`

**Step 1: OrderMergedOutApplier 清空 items 和 payments**

在 `apply()` 中，设置 `status = Merged` 之前:
```rust
snapshot.items.clear();
snapshot.payments.clear();
snapshot.comps.clear();
snapshot.paid_item_quantities.clear();
recalculate_totals(snapshot);
```

这会自动将所有金额字段归零。

**Step 2: 验证 + 提交**

Run: `cargo clippy --workspace`

```bash
git add edge-server/src/orders/appliers/orders_merged.rs
git commit -m "fix(orders): clear items/payments and recalculate in OrderMergedOutApplier"
```

---

### Task 15: CHAIN_BREAK 分类修正 [H-8]

**Files:**
- Modify: `edge-server/src/archiving/service.rs` (~verify_daily_chain 方法)

**Step 1: 识别 CHAIN_BREAK prev_hash 为系统事故**

找到 `verify_daily_chain()` 中链断裂分类逻辑，添加:
```rust
if prev_hash == "CHAIN_BREAK" {
    chain_resets += 1;  // 系统事故，不是篡改
} else if prev_hash != expected_prev_hash {
    chain_breaks += 1;  // 真正的链断裂
}
```

**Step 2: 验证 + 提交**

Run: `cargo clippy --workspace`

```bash
git add edge-server/src/archiving/service.rs
git commit -m "fix(archiving): classify CHAIN_BREAK prev_hash as chain_reset not chain_break"
```

---

## Phase 3: P2 — MEDIUM 级别修复 (按需执行)

### Task 16: checksum 扩展或注释 [F-003]

扩展 `compute_checksum()` 覆盖 member_id, has_amount_split, aa_total_shares, stamp_redemptions.len, comps.len, paid_item_quantities.len。或明确修改注释标注覆盖范围。

### Task 17: archived_order_item UNIQUE 约束 [A-4]

新建迁移添加 `CREATE UNIQUE INDEX idx_archived_item_order_instance ON archived_order_item(order_pk, instance_id);`

### Task 18: desglose ROUND [A-8]

SQL 查询改为 `ROUND(SUM(line_total - tax), 2)`。

### Task 19: huella fetch_one [H-4]

invoice.rs 中 `fetch_optional` 改 `fetch_one`。

### Task 20: 前端错误处理统一 [FE-14, FE-15, FE-16]

- AnulacionModal: 将 `result.reason` 改为 ErrorCode 映射
- useOrderHandlers: TABLE_OCCUPIED 用 error code 匹配
- SelectModePage: addOrderNote 添加 try/catch

### Task 21: void 按钮双触发修复 [FE-12]

删除 `<button onClick={onVoid}>`，只保留 `EscalatableGate.onAuthorized` 中的调用。

### Task 22: ChainEntryItem.status 强类型 [FE-18]

定义 `type ChainEntryStatus = 'VOID' | 'LOSS' | 'MERGED' | 'ANULADA' | 'COMPLETED' | null`。

### Task 23: generateCartKey 浮点修复 [FE-20]

用 `Math.round(discount * 100)` 序列化到 key，阈值改为 `discount > 0`。

### Task 24: PG 金额列迁移到 NUMERIC [Sync-7]

crab-cloud 迁移 store_archived_orders/store_order_items/store_credit_notes 金额列从 float8 到 NUMERIC(12,2)。Rust 侧改为 Decimal 绑定。

---

## 执行注意事项

1. **提交前必须通过**: `cargo clippy --workspace` + `cd red_coral && npx tsc --noEmit`
2. **跨 Rust + TypeScript 的变更两边都验证**
3. **每个 Task 独立提交**，不混合多个 Task
4. **Phase 1 (P0) 必须全部完成**后再进入 Phase 2
5. **Phase 3 按需选择**，不必一次全做
6. **读取相关文件后再修改** — 审计报告中的行号可能因 git 变更偏移
