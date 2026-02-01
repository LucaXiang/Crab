# RedCoral POS 订单事件溯源架构迁移报告

> 生成日期: 2026-01-20

## 1. 项目概述

### 1.1 背景
RedCoral POS 从本地事件存储 (`useOrderEventStore`) 迁移到服务端状态事件溯源架构 (`useActiveOrdersStore`)，实现多端同步和服务端权威状态。

### 1.2 核心原则
- **服务端权威**: 所有状态变更由服务端确认，禁止乐观更新
- **事件驱动**: 通过 `OrderEvent` 广播实现状态同步
- **MessageBus 协议**: Server/Client 模式统一使用 MessageBus 通信

### 1.3 支持的运行模式

| 模式 | 通信方式 | 数据库 | 适用场景 |
|------|----------|--------|----------|
| Server | In-Process (内存) | 本地 SurrealDB | 单机/主设备 |
| Client | TCP/TLS MessageBus | 远程 Edge Server | 从设备/多端同步 |

---

## 2. 架构设计

### 2.1 整体架构图

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           RedCoral Frontend                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐  │
│   │  useOrderCommands│     │useActiveOrders- │     │  useOrderSync   │  │
│   │   (发送命令)      │     │     Store       │     │   (重连同步)     │  │
│   └────────┬────────┘     │   (状态存储)     │     └────────┬────────┘  │
│            │              └────────▲────────┘              │           │
│            │                       │                       │           │
│            ▼                       │ _applyEvent           ▼           │
│   ┌─────────────────┐     ┌───────┴────────┐     ┌─────────────────┐  │
│   │ order_execute   │     │ useOrderEvent- │     │ order_sync_since│  │
│   │   (Tauri Cmd)   │     │   Listener     │     │   (Tauri Cmd)   │  │
│   └────────┬────────┘     │ (事件监听Hook) │     └────────┬────────┘  │
│            │              └───────▲────────┘              │           │
└────────────┼──────────────────────┼───────────────────────┼───────────┘
             │                      │                       │
             │              emit("order-event")             │
             ▼                      │                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Tauri Bridge (ClientBridge)                       │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   ┌─────────────────────────────┐   ┌─────────────────────────────┐    │
│   │        Server Mode          │   │        Client Mode          │    │
│   ├─────────────────────────────┤   ├─────────────────────────────┤    │
│   │                             │   │                             │    │
│   │  execute_order_command:     │   │  execute_order_command:     │    │
│   │    ↓                        │   │    ↓                        │    │
│   │  OrdersManager              │   │  MessageBus.request()       │    │
│   │    .execute_command_with_   │   │    action: "order.*"        │    │
│   │     events()                │   │    params: OrderCommand     │    │
│   │    ↓                        │   │    ↓                        │    │
│   │  emit("order-event", event) │   │  Remote Edge Server         │    │
│   │                             │   │    ↓                        │    │
│   │  ─────────────────────────  │   │  MessageBus broadcast       │    │
│   │                             │   │    ↓                        │    │
│   │  Message Listener:          │   │  MessageClient.subscribe()  │    │
│   │    server_state.message_bus │   │    ↓                        │    │
│   │      .subscribe()           │   │  MessageRoute::from_bus_msg │    │
│   │    ↓                        │   │    ↓                        │    │
│   │  MessageRoute::from_bus_msg │   │  emit("order-event", event) │    │
│   │    ↓                        │   │                             │    │
│   │  emit("order-event", event) │   │                             │    │
│   │                             │   │                             │    │
│   └─────────────────────────────┘   └─────────────────────────────┘    │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
             │                                              │
             ▼                                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                           Edge Server                                    │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐  │
│   │ RequestCommand- │     │  OrdersManager  │     │   MessageBus    │  │
│   │   Processor     │────▶│                 │────▶│                 │  │
│   │                 │     │ - execute_cmd() │     │ - publish()     │  │
│   │ action:         │     │ - subscribe()   │     │ - broadcast()   │  │
│   │ - order.*       │     │ - get_snapshot  │     │                 │  │
│   │ - sync.*        │     │ - get_events    │     │                 │  │
│   └─────────────────┘     └────────┬────────┘     └─────────────────┘  │
│                                    │                                    │
│                                    ▼                                    │
│                           ┌─────────────────┐                          │
│                           │  OrderStorage   │                          │
│                           │    (redb)       │                          │
│                           │                 │                          │
│                           │ - events table  │                          │
│                           │ - snapshots     │                          │
│                           │ - active_orders │                          │
│                           └─────────────────┘                          │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.2 事件流详解

#### Server 模式命令执行流程
```
1. Frontend: orderOps.handleTableSelect(table, guestCount, ...)
2. Tauri Command: order_execute(command)
3. Bridge: bridge.execute_order_command(command)
4. OrdersManager: execute_command_with_events(command)
   ├─ 4.1 幂等性检查 (command_id)
   ├─ 4.2 开始写事务
   ├─ 4.3 处理命令 → 生成事件
   ├─ 4.4 持久化事件
   ├─ 4.5 更新快照
   ├─ 4.6 标记命令已处理
   ├─ 4.7 提交事务
   └─ 4.8 广播事件 (event_tx.send)
5. Bridge: emit("order-event", event) → Frontend
6. Frontend: _applyEvent(event) → 更新 Store
```

#### Client 模式命令执行流程
```
1. Frontend: orderOps.handleTableSelect(table, guestCount, ...)
2. Tauri Command: order_execute(command)
3. Bridge: bridge.execute_order_command(command)
4. MessageClient: request(RequestCommand { action: "order.*", params: command })
5. Edge Server: RequestCommandProcessor.handle_order_command()
6. OrdersManager: execute_command(command)
   └─ (同 Server 模式 4.1-4.8)
7. Event Forwarder: MessageBus.publish(SyncPayload { resource: "order_event", data: event })
8. MessageClient: subscribe() 收到消息
9. Bridge: MessageRoute::from_bus_message() → emit("order-event", event)
10. Frontend: _applyEvent(event) → 更新 Store
```

---

## 3. 修复的问题清单

### 3.1 Edge Server 问题

| # | 问题 | 文件 | 修复内容 |
|---|------|------|----------|
| 1 | 缺失 `sync.order_snapshot` 处理器 | `processor.rs` | 添加处理器，调用 `orders_manager.get_snapshot()` |
| 2 | 缺失 `sync.active_events` 处理器 | `processor.rs` | 添加处理器，调用 `orders_manager.get_active_events_since()` |
| 3 | Order 命令丢失元数据 | `processor.rs` | 改为解析完整 `OrderCommand` 而非只解析 `payload` |

### 3.2 Tauri Bridge 问题

| # | 问题 | 文件 | 修复内容 |
|---|------|------|----------|
| 4 | Client 模式只发送 `command.payload` | `bridge/mod.rs:1201` | 改为发送完整 `command` |

### 3.3 Frontend 问题

| # | 问题 | 文件 | 修复内容 |
|---|------|------|----------|
| 5 | `useOrderEventListener` 不支持 Client 模式 | `useOrderEventListener.ts:88` | 添加 `ClientAuthenticated` 条件 |
| 6 | Tauri 命令名称错误 | `useOrderSync.ts:55,248` | `sync_orders` → `order_sync_since` |
| 7 | `ItemModifiedPayload` 类型不匹配 | `orderEvent.ts:121-140` | 添加 `operation`, `source`, `results` 等字段 |
| 8 | `OrderSnapshot` 缺少字段 | `orderEvent.ts:450` | 添加 `paid_item_quantities` |
| 9 | `orderReducer` 使用错误字段 | `orderReducer.ts:248-285` | 重写 `applyItemModified` 使用 `results` 数组 |
| 10 | 缺少 `ItemModificationResult` 导出 | `types/index.ts:36` | 添加导出 |

---

## 4. 代码变更清单

### 4.1 Edge Server (`/edge-server/src/`)

```
message/processor.rs
├─ L5:   移除未使用的 OrderCommandPayload import
├─ L135-168: 重写 handle_order_command()，解析完整 OrderCommand
├─ L217-240: 新增 handle_sync_order_snapshot()
├─ L242-265: 新增 handle_sync_active_events()
└─ L338-342: 添加 sync.order_snapshot 和 sync.active_events 路由
```

### 4.2 Tauri Bridge (`/red_coral/src-tauri/src/`)

```
core/bridge/mod.rs
└─ L1198-1202: Client 模式发送完整 OrderCommand
```

### 4.3 Frontend (`/red_coral/src/`)

```
core/hooks/useOrderEventListener.ts
└─ L88-91: 添加 ClientAuthenticated 到监听条件

core/stores/order/useOrderSync.ts
└─ L55,248: sync_orders → order_sync_since

core/stores/order/orderReducer.ts
└─ L248-285: 重写 applyItemModified() 使用 results 数组

core/domain/types/orderEvent.ts
├─ L121-148: 重写 ItemModifiedPayload，添加新字段
├─ L141-148: 新增 ItemModificationResult 接口
└─ L450: OrderSnapshot 添加 paid_item_quantities

core/domain/types/index.ts
└─ L36: 导出 ItemModificationResult
```

---

## 5. 类型系统对照

### 5.1 OrderEvent (Rust ↔ TypeScript)

```rust
// Rust: shared/src/order/event.rs
pub struct OrderEvent {
    pub event_id: String,
    pub sequence: u64,
    pub order_id: String,
    pub timestamp: i64,
    pub operator_id: String,
    pub operator_name: String,
    pub command_id: String,
    pub event_type: OrderEventType,
    pub payload: EventPayload,
}
```

```typescript
// TypeScript: orderEvent.ts
interface OrderEvent {
  event_id: string;
  sequence: number;
  order_id: string;
  timestamp: number;
  operator_id: string;
  operator_name: string;
  command_id: string;
  event_type: OrderEventType;
  payload: EventPayload;
}
```

### 5.2 OrderSnapshot (Rust ↔ TypeScript)

```rust
// Rust: shared/src/order/snapshot.rs
pub struct OrderSnapshot {
    pub order_id: String,
    pub table_id: Option<String>,
    pub table_name: Option<String>,
    pub zone_id: Option<String>,
    pub zone_name: Option<String>,
    pub guest_count: i32,
    pub is_retail: bool,
    pub status: OrderStatus,
    pub items: Vec<CartItemSnapshot>,
    pub payments: Vec<PaymentRecord>,
    pub subtotal: f64,
    pub tax: f64,
    pub discount: f64,
    pub surcharge: Option<SurchargeConfig>,
    pub surcharge_exempt: bool,
    pub total: f64,
    pub paid_amount: f64,
    pub paid_item_quantities: HashMap<String, i32>,  // ✅ 已添加
    pub receipt_number: Option<String>,
    pub is_pre_payment: bool,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_sequence: u64,
}
```

### 5.3 ItemModifiedPayload (修复后)

```typescript
// TypeScript: orderEvent.ts (修复后)
interface ItemModifiedPayload {
  type: 'ITEM_MODIFIED';
  operation: string;           // ✅ 新增
  source: CartItemSnapshot;    // ✅ 新增
  affected_quantity: number;   // ✅ 新增
  changes: ItemChanges;
  previous_values: ItemChanges;
  results: ItemModificationResult[];  // ✅ 新增
  authorizer_id?: string | null;      // ✅ 新增
  authorizer_name?: string | null;    // ✅ 新增
}

interface ItemModificationResult {  // ✅ 新增类型
  instance_id: string;
  quantity: number;
  price: number;
  discount_percent?: number | null;
  action: string;  // "UPDATED" | "UNCHANGED" | "CREATED"
}
```

---

## 6. MessageBus 协议

### 6.1 RequestCommand Actions

| Action | 参数 | 返回 | 用途 |
|--------|------|------|------|
| `order.open_table` | `OrderCommand` | `CommandResponse` | 开台 |
| `order.add_items` | `OrderCommand` | `CommandResponse` | 添加商品 |
| `order.modify_item` | `OrderCommand` | `CommandResponse` | 修改商品 |
| `order.remove_item` | `OrderCommand` | `CommandResponse` | 删除商品 |
| `order.restore_item` | `OrderCommand` | `CommandResponse` | 恢复商品 |
| `order.add_payment` | `OrderCommand` | `CommandResponse` | 添加支付 |
| `order.cancel_payment` | `OrderCommand` | `CommandResponse` | 取消支付 |
| `order.split` | `OrderCommand` | `CommandResponse` | 分单 |
| `order.move` | `OrderCommand` | `CommandResponse` | 转台 |
| `order.merge` | `OrderCommand` | `CommandResponse` | 并单 |
| `order.complete` | `OrderCommand` | `CommandResponse` | 结账 |
| `order.void` | `OrderCommand` | `CommandResponse` | 作废 |
| `order.set_surcharge_exempt` | `OrderCommand` | `CommandResponse` | 设置服务费豁免 |
| `order.update_info` | `OrderCommand` | `CommandResponse` | 更新订单信息 |
| `sync.orders` | `{ since_sequence }` | `SyncResponse` | 同步订单 |
| `sync.order_snapshot` | `{ order_id }` | `OrderSnapshot` | 获取单个快照 |
| `sync.active_events` | `{ since_sequence }` | `{ events, server_sequence }` | 获取活跃事件 |

### 6.2 Event Broadcast (SyncPayload)

```rust
SyncPayload {
    resource: "order_event",
    version: event.sequence,
    action: event.event_type.to_string(),
    id: event.order_id,
    data: serde_json::to_value(&event),
}
```

---

## 7. 关键文件索引

### 7.1 Edge Server

| 文件 | 职责 |
|------|------|
| `orders/manager.rs` | 命令处理、事件生成、广播 |
| `orders/storage.rs` | redb 持久化层 |
| `orders/reducer.rs` | 事件 → 快照转换 |
| `message/processor.rs` | RequestCommand 处理器 |
| `core/state.rs` | ServerState，事件转发器 |

### 7.2 Tauri Bridge

| 文件 | 职责 |
|------|------|
| `core/bridge/mod.rs` | Server/Client 模式统一接口 |
| `commands/order_es.rs` | Tauri 命令定义 |
| `events.rs` | MessageRoute 事件路由 |

### 7.3 Frontend

| 文件 | 职责 |
|------|------|
| `stores/order/useActiveOrdersStore.ts` | 订单状态存储 (只读镜像) |
| `stores/order/useOrderOperations.ts` | 命令发送函数 |
| `stores/order/useOrderSync.ts` | 重连同步逻辑 |
| `stores/order/orderReducer.ts` | 事件 → 快照 Reducer |
| `hooks/useOrderEventListener.ts` | Tauri 事件监听 Hook |
| `domain/types/orderEvent.ts` | 类型定义 |

---

## 8. 测试建议

### 8.1 单元测试

```typescript
// orderReducer.test.ts
describe('applyItemModified', () => {
  it('should handle UPDATED action', () => { ... });
  it('should handle UNCHANGED + CREATED split', () => { ... });
});
```

### 8.2 集成测试

```bash
# Server 模式完整流程
1. 开台 → 验证 order-event 触发
2. 添加商品 → 验证 Store 更新
3. 结账 → 验证订单状态变更

# Client 模式完整流程
1. 连接远程 Edge Server
2. 执行命令 → 验证 MessageBus 通信
3. 验证事件广播接收
```

### 8.3 压力测试

```bash
# 并发命令测试
- 多个客户端同时发送命令
- 验证幂等性 (相同 command_id)
- 验证事件序列一致性
```

---

## 9. 已知限制与后续工作

### 9.1 已知限制

| 限制 | 影响 | 优先级 |
|------|------|--------|
| `order-connection` 事件未实现 | 前端无法自动感知断线 | 低 |
| `order-sync-request` 事件未实现 | 服务器无法主动触发客户端同步 | 低 |
| 断线自动重连未实现 | 需手动调用 reconnect | 中 |

### 9.2 建议后续工作

1. **连接状态管理**: 实现 `order-connection` 事件
2. **自动重连**: MessageClient 断线检测 + 自动重连
3. **离线队列**: Client 模式离线时命令排队
4. **冲突解决**: 多端并发修改的冲突处理
5. **性能优化**: 事件批量处理、快照缓存

---

## 10. 总结

本次迁移成功实现了：

- ✅ **服务端权威状态** - 所有状态由 Edge Server 管理
- ✅ **事件溯源架构** - OrderEvent 驱动状态变更
- ✅ **多端同步** - Server/Client 模式统一协议
- ✅ **类型安全** - Rust ↔ TypeScript 类型对齐
- ✅ **幂等性保证** - command_id 防止重复执行

核心功能已完整可用，后续可根据实际需求迭代完善。
