# Snowflake ID 全栈统一化设计

## 背景

当前系统中 ID 类型混乱：UUID (order_id, event_id, command_id, payment_id)、autoincrement (部分表)、snowflake (新表)、String (resource_id, SyncPayload.id) 并存。这导致：

1. 类型不一致：Rust `String` ↔ TS `string`，但实际值是 i64 snowflake 的字符串表示
2. 不必要的 `.parse::<i64>()`/`.to_string()` 转换遍布 cloud sync 路径
3. `order_key` (UUID) 概念多余——snowflake 本身就是全局唯一的
4. redb 用 `&str` 键存 UUID，浪费空间且比较慢

## 目标

- 所有实体 ID 统一为 `i64` snowflake（53-bit: 41-bit 时间戳 + 12-bit 随机）
- 移除 `uuid` crate 依赖
- 消除所有 String ↔ i64 ID 转换
- `instance_id` 保持 content-addressed hash String（正确设计）

## 设计

### 1. shared 类型变更

#### 订单事件溯源

| 类型 | 字段 | `String` → `i64` |
|------|------|-------------------|
| `OrderSnapshot` | `order_id` | ✅ |
| `OrderEvent` | `order_id`, `event_id` | ✅ |
| `OrderCommand*` (22 variants) | `order_id` | ✅ |
| `OrderCommand` | `command_id` | ✅ |
| `PaymentRecord` | `payment_id` | ✅ |

#### 同步协议

| 类型 | 字段 | 变更 |
|------|------|------|
| `SyncPayload` | `id: String` → `id: i64` | ✅ |
| `CloudSyncItem` | `resource_id: String` → `resource_id: i64` | ✅ |
| `CloudSyncError` | `resource_id: String` → `resource_id: i64` | ✅ |

#### ID 生成

所有 UUID 生成点改用 `snowflake_id()`：
- `OrderCommand::command_id` (command.rs)
- `OrderEvent::event_id` (event.rs)
- `PaymentRecord::payment_id` (types.rs)
- `open_table.rs` 中的 `order_id`

### 2. redb 存储键类型

```rust
// Before
EVENTS_TABLE: TableDefinition<(&str, u64), &[u8]>
SNAPSHOTS_TABLE: TableDefinition<&str, &[u8]>
ACTIVE_ORDERS_TABLE: TableDefinition<&str, ()>
ORDER_META_TABLE: TableDefinition<&str, &[u8]>

// After
EVENTS_TABLE: TableDefinition<(i64, u64), &[u8]>
SNAPSHOTS_TABLE: TableDefinition<i64, &[u8]>
ACTIVE_ORDERS_TABLE: TableDefinition<i64, ()>
ORDER_META_TABLE: TableDefinition<i64, &[u8]>
```

### 3. broadcast_sync 拆分

当前 `broadcast_sync(id: &str, ...)` 有两个特殊 sentinel 值：
- `StoreInfo` 用 `"main"`
- `Category` batch 用 `"batch"`

拆分为：

```rust
/// 带 ID 的资源同步（Product, Employee, Tag 等）
fn broadcast_sync(&self, resource: SyncResource, id: i64, action: &str, data: Value)

/// 无 ID 的单例/批量资源同步（StoreInfo, Category batch）
fn broadcast_sync_resource(&self, resource: SyncResource, action: &str, data: Value)
```

前端 `SyncPayload` 相应变为：

```typescript
// 带 ID
interface SyncPayload { resource: string; id: number; action: string; data: unknown }
// 单例
interface SyncResourcePayload { resource: string; action: string; data: unknown }
```

### 4. order_key 处理

- 删除 `archived_order.order_key` 列（VARCHAR UUID）
- 删除 Cloud 端 `store_archived_orders.order_key` 列
- Cloud sync `resource_id` 直接使用 `archived_order.id`（snowflake）
- `receipt_number` 继续作为面向用户的编号（FAC 前缀格式）
- 需要数据库 migration

### 5. instance_id

保持不变：
- 基础部分：SHA-256 content-addressed hash (String) — 确定性，允许相同配置的商品行合并
- 拆分后缀变更：`::comp::{uuid}` → `::comp::{snowflake}`，`::mod::{uuid}` → `::mod::{snowflake}`

### 6. Cloud 端变更

`crab-cloud/src/db/sync_store.rs`：
- 移除所有 `item.resource_id.parse::<i64>()` 调用
- `resource_id` 直接作为 `i64` 绑定到 SQL
- `delete_resource()` 参数从 `&str` 改为 `i64`

`crab-cloud/src/api/sync.rs`：
- 响应中 `resource_id` 类型更新

### 7. 前端类型变更

**red_coral (POS)**：
```typescript
// orderEvent.ts
OrderEvent.order_id: string → number
OrderEvent.event_id: string → number
OrderSnapshot.order_id: string → number
OrderCommand.command_id: string → number
PaymentRecord.payment_id: string → number
CommandResponse.order_id: string → number
```

**crab-console**：
- 消费相同的 API，ID 字段从 `string` 改为 `number`
- JS `number` 53-bit 安全，snowflake 53-bit 完美匹配

### 8. 哈希链影响

`canonical.rs` 中 `CanonicalHash` 实现需要更新：
- `order_id` 序列化从 `write_str` 改为 `write_i64`
- `event_id`, `command_id`, `payment_id` 同理
- **注意**：这意味着新旧 hash 不兼容。由于是开发阶段（CLAUDE.md: "不要兼容性"），直接切换

### 9. 安全性

- 租户隔离：每个 edge-server 服务单一租户，snowflake 碰撞概率可忽略
- 53-bit snowflake：41-bit 时间戳 + 12-bit 随机，同毫秒碰撞概率 1/4096
- JS 安全：`Number.MAX_SAFE_INTEGER` = 2^53 - 1，完全覆盖

## 影响范围

### Rust crates

| Crate | 变更规模 |
|-------|----------|
| `shared` | 大 — 订单类型、同步协议、canonical hash |
| `edge-server` | 大 — redb 存储、OrdersManager、所有 action/applier、归档、cloud worker |
| `crab-cloud` | 中 — sync_store、API 响应 |
| `crab-client` | 小 — 类型透传 |

### 前端

| 项目 | 变更规模 |
|------|----------|
| `red_coral` | 中 — 类型定义、订单相关组件 |
| `crab-console` | 小 — 订单详情显示 |

### 数据库

- Edge SQLite：需要 migration 删除 `order_key` 列
- Cloud PostgreSQL：需要 migration 删除 `order_key` 列
- redb：数据格式变更，需要清空重建（开发阶段可接受）

## 不在范围内

- `instance_id` 基础 hash 不变（已是正确的 content-addressed 设计）
- `receipt_number`、`credit_note_number`、`invoice_number` 不变（面向用户的编号）
