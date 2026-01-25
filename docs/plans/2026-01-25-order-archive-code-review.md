# Order Archive Graph Model - 代码评审报告

> 日期: 2026-01-25
> 范围: 订单归档图模型实现 (commits 4183501..a5ec8d9)

## 评审摘要

| 类别 | 状态 | 说明 |
|------|------|------|
| 架构完整性 | ✅ 通过 | 图模型实现完整，RELATE 边关系正确 |
| 类型一致性 | ⚠️ 需修复 | 存在重复定义 |
| 废弃代码清理 | ✅ 通过 | 无遗留的 snapshot_json 等废弃代码 |
| 测试覆盖 | ✅ 通过 | 438 后端测试 + 22 前端测试通过 |
| 代码质量 | ⚠️ 需修复 | 3 个 Clippy 警告，1 个 unwrap |

## 发现的问题

### P1: 严重 - 需立即修复

#### 1. OrderSummary 重复定义
**位置:**
- `edge-server/src/db/models/order.rs:221`
- `edge-server/src/api/orders/handler.rs:283`

**问题:** 两处定义完全相同，违反 DRY 原则。

**修复:** 删除 `handler.rs` 中的定义，导入 `models::OrderSummary`。

### P2: 中等 - 应该修复

#### 2. Tauri 层类型重复
**位置:** `red_coral/src-tauri/src/commands/orders.rs`

**问题:** 重新定义了 `OrderDetail`, `OrderItemDetail` 等类型，与 `edge-server` 中的定义重复。

**建议:** 考虑创建 `shared` crate 或直接导入。当前可接受，因为 Tauri 需要独立序列化。

#### 3. 未使用的导入
**位置:** `red_coral/src/screens/History/HistoryDetail.tsx:5`

**问题:** `Trash2` 导入但未使用。

**修复:** 移除未使用的导入。

#### 4. 潜在 panic 风险
**位置:** `edge-server/src/orders/archive.rs:210`

```rust
Some(serde_json::to_value(&event.payload).unwrap()),
```

**问题:** 使用 `unwrap()` 可能导致 panic。

**修复:** 使用 `ok()` 或 `?` 操作符。

### P3: 低 - 代码质量

#### 5. Clippy 警告 (3个)
**位置:** `edge-server/src/orders/archive.rs:369, 372`

**问题:** 不必要的闭包用于 `unwrap_or_else`。

**修复:** 运行 `cargo clippy --fix`。

## 验证通过项

### 架构完整性 ✅
- SurrealDB Schema 与 Rust 模型完全对齐
- 4 个图边关系正确定义 (has_item, has_option, has_payment, has_event)
- 前后端类型字段匹配

### 废弃代码清理 ✅
- 无 `snapshot_json` 残留引用
- 无 `items_json`, `payments_json` 残留
- 无 TODO/FIXME/HACK 标记

### 类型系统设计 ✅
- 活动订单使用 `HeldOrder`/`OrderSnapshot` (事件溯源)
- 归档订单使用 `ArchivedOrderDetail` (图模型)
- 两个系统职责分离清晰

## 修复计划

1. [x] 删除 handler.rs 中重复的 OrderSummary ✅
2. [x] 移除 HistoryDetail.tsx 中未使用的 Trash2 导入 ✅
3. [x] 修复 archive.rs 中的 unwrap() ✅
4. [x] 运行 cargo clippy --fix 修复警告 ✅

## 修复提交

- Commit: `5433e8a`
- 测试: 438 后端测试通过, TypeScript 编译通过
