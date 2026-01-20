# 订单事件溯源架构设计

> 日期: 2026-01-19
> 状态: 设计完成，待实施

## 1. 设计目标

### 1.1 核心需求

1. **服务端状态** - 所有订单状态由服务端管理，前端是只读镜像
2. **事件驱动** - 所有变更通过事件广播，不使用乐观更新
3. **多端同步** - 多个 POS 终端实时看到相同的订单状态
4. **审计严格** - 每个操作完整记录，可追踪事情发展
5. **断线重播** - 客户端断线后可完整重播缺失事件

### 1.2 不变的约束

- 不改变现有 UI/UX
- edge_server 只处理单个门店
- Server 模式无离线问题，Client 模式断联禁止操作

## 2. 整体架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        RedCoral POS 终端                         │
├─────────────────────────────────────────────────────────────────┤
│  React UI                                                        │
│     │                                                            │
│     ▼                                                            │
│  useActiveOrdersStore (只读镜像)                                 │
│     ▲                                                            │
│     │ 事件驱动更新                                                │
│  Tauri Event Listener ←───── emit('order-event', event)         │
├─────────────────────────────────────────────────────────────────┤
│  Tauri Rust Backend                                              │
│     │                                                            │
│     ├─ invoke('send_order_command', cmd) ──→ OrderBridge        │
│     │                                              │             │
│     │                                              ▼             │
│     │                                     MessageBus.send()      │
│     │                                              │             │
│     └─ EventSubscriber ←──────────────── MessageBus.subscribe() │
│              │                                                   │
│              └──→ window.emit('order-event', event)             │
├─────────────────────────────────────────────────────────────────┤
│  Transport Layer                                                 │
│     ├─ Server Mode: MemoryTransport (同进程)                     │
│     └─ Client Mode: TlsTransport (mTLS)                         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Edge Server                               │
├─────────────────────────────────────────────────────────────────┤
│  OrderCommandProcessor (MessageBus Handler)                      │
│     │                                                            │
│     ▼                                                            │
│  OrdersManager (同步处理)                                         │
│     │                                                            │
│     ├─ 1. 检查幂等性 (command_id 是否已处理)                       │
│     ├─ 2. 验证命令合法性                                          │
│     ├─ 3. 生成 OrderEvent (带递增 sequence)                       │
│     ├─ 4. 持久化到 redb (事务)                                    │
│     ├─ 5. 记录 command_id (幂等表)                                │
│     └─ 6. 返回 ACK + 广播 Event                                   │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│  Storage                                                         │
│                                                                  │
│  ┌─────────────────┐         ┌─────────────────────────────┐    │
│  │     redb        │         │        SurrealDB            │    │
│  │  (活跃订单)      │ ──归档─→│       (历史订单)             │    │
│  │                 │         │                             │    │
│  │ • events        │         │ • archived_orders           │    │
│  │ • snapshots     │         │ • archived_events           │    │
│  │ • command_ids   │         │ • 支持复杂查询/报表           │    │
│  │ • sequence      │         │                             │    │
│  └─────────────────┘         └─────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

## 3. 核心概念

### 3.1 命令-事件分离 (CQRS)

- **命令 (Command)**: 前端发送的操作请求，表达"意图"
- **事件 (Event)**: 服务端处理后产生的事实记录，表达"发生了什么"
- **响应 (Response)**: 只返回 ACK/NACK，不返回新状态

```
前端 ─────Command────→ 后端处理 ────→ ACK/NACK
                          │
                          ▼
                     生成 Event
                          │
                          ▼
前端 ←────Event 广播────所有终端
```

### 3.2 事件溯源 (Event Sourcing)

- **事件是唯一真相源** - 订单状态从事件流计算得出
- **快照是缓存** - 可随时从事件重建
- **可重播验证** - `rebuild(events) == snapshot`

### 3.3 instance_id 内容寻址

`instance_id` 是商品属性的 hash，不是固定 ID：

```
instance_id = hash(product_id + price + discount + options + spec + ...)
```

**行为**：
- 相同属性的商品自动合并（增加数量）
- 属性变化时自动分裂（生成新 instance_id）

**示例**：
```
初始：4 个可乐 (instance_id: "abc123", qty: 4)
     ↓
给 2 个打 5 折
     ↓
结果：
  - 2 个原价可乐 (instance_id: "abc123", qty: 2)
  - 2 个打折可乐 (instance_id: "def456", qty: 2)
```

## 4. 数据结构

### 4.1 命令结构

```rust
pub struct OrderCommand {
    pub command_id: Uuid,           // 幂等 ID（前端生成）
    pub timestamp: i64,
    pub operator_id: String,
    pub operator_name: String,
    pub payload: OrderCommandPayload,
}

pub enum OrderCommandPayload {
    OpenTable { ... },
    CompleteOrder { order_id, receipt_number },
    VoidOrder { order_id, reason },
    AddItems { order_id, items },
    ModifyItem { order_id, instance_id, changes },
    RemoveItem { order_id, instance_id, quantity, reason },
    AddPayment { order_id, method, amount, ... },
    CancelPayment { order_id, payment_id, reason },
    SplitOrder { order_id, split_amount, payment_method, items },
    MoveOrder { order_id, target_table_id, target_table_name },
    MergeOrders { source_order_id, target_order_id },
    SetSurchargeExempt { order_id, exempt },
    UpdateOrderInfo { order_id, ... },
}
```

### 4.2 事件结构

```rust
pub struct OrderEvent {
    pub event_id: String,           // UUID
    pub sequence: u64,              // 全局递增序列号
    pub order_id: String,
    pub timestamp: i64,
    pub operator_id: String,
    pub operator_name: String,      // 快照，防止后续改名
    pub command_id: String,         // 审计追溯
    pub event_type: OrderEventType,
    pub payload: EventPayload,
}
```

### 4.3 ITEM_MODIFIED 事件（审计明确）

```json
{
  "event_type": "ITEM_MODIFIED",
  "payload": {
    "operation": "APPLY_DISCOUNT",
    "source": {
      "instance_id": "abc123",
      "product_id": "cola",
      "name": "可口可乐",
      "quantity": 4,
      "price": 3.00,
      "discount_percent": null
    },
    "affected_quantity": 2,
    "changes": {
      "discount_percent": 50
    },
    "results": [
      {
        "instance_id": "abc123",
        "quantity": 2,
        "price": 3.00,
        "discount_percent": null,
        "action": "UNCHANGED"
      },
      {
        "instance_id": "def456",
        "quantity": 2,
        "price": 1.50,
        "discount_percent": 50,
        "action": "CREATED"
      }
    ],
    "authorizer_id": "mgr_01",
    "authorizer_name": "李经理"
  }
}
```

**审计可读性**：
> "张三 对 4个可乐(abc123) 中的 2个 应用了 50%折扣，经李经理授权。结果：2个保持原价(abc123)，2个变为折扣价(def456)"

### 4.4 命令响应

```rust
pub struct CommandResponse {
    pub command_id: Uuid,
    pub success: bool,
    pub order_id: Option<String>,  // 仅 OpenTable 返回新 ID
    pub error: Option<CommandError>,
}

pub struct CommandError {
    pub code: String,  // "ORDER_NOT_FOUND", "ALREADY_COMPLETED" 等
    pub message: String,
}
```

## 5. OrdersManager 设计

### 5.1 职责

- 接收命令，同步处理
- 生成事件，持久化到 redb
- 广播事件给所有订阅者
- 管理订单生命周期（活跃 → 归档）
- 分配 order_id 和 sequence

### 5.2 redb 表结构

| 表名 | Key | Value | 用途 |
|------|-----|-------|------|
| `events` | `(order_id, sequence)` | `OrderEvent` | 事件流（只追加） |
| `snapshots` | `order_id` | `OrderSnapshot` | 快照缓存（可重建） |
| `active_orders` | `order_id` | `()` | 活跃订单索引 |
| `processed_commands` | `command_id` | `()` | 幂等性检查 |
| `sequence_counter` | `()` | `u64` | 全局序列号 |

### 5.3 命令执行流程

```rust
impl OrdersManager {
    pub fn execute_command(&self, cmd: OrderCommand) -> Result<CommandResponse> {
        // 1. 幂等检查
        if self.is_command_processed(&cmd.command_id) {
            return Ok(CommandResponse::duplicate(cmd.command_id));
        }

        // 2. 开启 redb 写事务
        let txn = self.db.begin_write()?;

        // 3. 处理命令，生成事件
        let result = self.process_command(&txn, &cmd);

        match result {
            Ok(event) => {
                // 4. 持久化事件
                self.persist_event(&txn, &event)?;

                // 5. 更新快照
                self.update_snapshot(&txn, &event)?;

                // 6. 标记命令已处理
                self.mark_command_processed(&txn, &cmd.command_id)?;

                // 7. 提交事务
                txn.commit()?;

                // 8. 广播事件（事务提交后）
                self.broadcast_event(event.clone());

                Ok(CommandResponse::success(cmd.command_id, event.order_id))
            }
            Err(e) => Ok(CommandResponse::error(cmd.command_id, e))
        }
    }
}
```

## 6. 断线重播机制

### 6.1 同步协议

```rust
pub struct SyncRequest {
    pub since_sequence: u64,
}

pub struct SyncResponse {
    pub events: Vec<OrderEvent>,
    pub active_orders: Vec<OrderSnapshot>,
    pub server_sequence: u64,
    pub requires_full_sync: bool,
}
```

### 6.2 重连流程

```
客户端重连
    │
    ▼
获取本地 lastSequence
    │
    ▼
请求 sync(since=lastSequence)
    │
    ▼
服务端返回缺失事件
    │
    ▼
┌─────────────────────────┐
│ gap > 1000?             │
├─────────────────────────┤
│ YES → 全量重置          │
│ NO  → 逐个应用事件      │
└─────────────────────────┘
    │
    ▼
更新 lastSequence
    │
    ▼
订阅实时事件流
```

### 6.3 保证

- 事件只追加，永不删除（活跃订单期间）
- sequence 全局递增，无间隙
- 任何断线时长都能恢复一致状态

## 7. 前端架构

### 7.1 Store 设计

```typescript
// useActiveOrdersStore.ts - 只读镜像
interface ActiveOrdersState {
  orders: Map<string, OrderSnapshot>;
  lastSequence: number;
  connectionState: 'connected' | 'disconnected' | 'syncing';

  // 只读查询
  getOrder(orderId: string): OrderSnapshot | undefined;
  getActiveOrders(): OrderSnapshot[];
  getOrderByTable(tableId: string): OrderSnapshot | undefined;

  // 内部方法（仅事件驱动调用）
  _applyEvent(event: OrderEvent): void;
  _applyEvents(events: OrderEvent[]): void;
  _fullSync(orders: OrderSnapshot[], sequence: number): void;
  _setConnectionState(state: ConnectionState): void;
}

// useOrderCommands.ts - 命令发送
interface OrderCommands {
  openTable(params: OpenTableParams): Promise<CommandResponse>;
  addItems(orderId: string, items: CartItemInput[]): Promise<CommandResponse>;
  modifyItem(orderId: string, instanceId: string, changes: ItemChanges): Promise<CommandResponse>;
  removeItem(orderId: string, instanceId: string, quantity?: number, reason?: string): Promise<CommandResponse>;
  addPayment(orderId: string, payment: PaymentInput): Promise<CommandResponse>;
  completeOrder(orderId: string, receiptNumber: string): Promise<CommandResponse>;
  voidOrder(orderId: string, reason?: string): Promise<CommandResponse>;
  // ... 其他命令
}
```

### 7.2 数据流

```
用户点击"加菜"
    │
    ▼
useOrderCommands.addItems(orderId, items)
    │
    ▼
invoke('send_order_command', { command })
    │
    ▼
等待 ACK/NACK
    │
    ├─ NACK → 显示错误
    │
    └─ ACK → 等待事件广播
              │
              ▼
         Tauri emit('order-event', event)
              │
              ▼
         useActiveOrdersStore._applyEvent(event)
              │
              ▼
         UI 自动更新
```

### 7.3 断线处理

```typescript
// Client 模式断线时
function onDisconnect() {
  useActiveOrdersStore.getState()._setConnectionState('disconnected');
}

// UI 层阻断
function OrderButton({ onClick, children }) {
  const isConnected = useActiveOrdersStore(s => s.connectionState === 'connected');

  if (!isConnected) {
    return <button disabled className="opacity-50">断线中...</button>;
  }

  return <button onClick={onClick}>{children}</button>;
}
```

## 8. 支持的操作清单

| 类别 | 操作 | 命令类型 | 事件类型 |
|------|------|---------|---------|
| **生命周期** | 开台 | `OPEN_TABLE` | `TABLE_OPENED` |
| | 结单 | `COMPLETE_ORDER` | `ORDER_COMPLETED` |
| | 作废 | `VOID_ORDER` | `ORDER_VOIDED` |
| **商品** | 加菜 | `ADD_ITEMS` | `ITEMS_ADDED` |
| | 编辑商品 | `MODIFY_ITEM` | `ITEM_MODIFIED` |
| | 删除商品 | `REMOVE_ITEM` | `ITEM_REMOVED` |
| **支付** | 添加支付 | `ADD_PAYMENT` | `PAYMENT_ADDED` |
| | 取消支付 | `CANCEL_PAYMENT` | `PAYMENT_CANCELLED` |
| | 分账 | `SPLIT_ORDER` | `ORDER_SPLIT` |
| **桌台** | 移动订单 | `MOVE_ORDER` | `ORDER_MOVED` + `ORDER_MOVED_OUT` |
| | 合并订单 | `MERGE_ORDERS` | `ORDER_MERGED` + `ORDER_MERGED_OUT` |
| | 豁免服务费 | `SET_SURCHARGE_EXEMPT` | `SURCHARGE_EXEMPT_SET` |
| **其他** | 更新信息 | `UPDATE_ORDER_INFO` | `ORDER_INFO_UPDATED` |

**暂不实现**：`RESTORE_ITEM`（Schema 保留，待后续扩展）

## 9. 审计追溯能力

### 9.1 事件包含的审计字段

| 字段 | 用途 |
|------|------|
| `event_id` | 事件唯一标识 |
| `sequence` | 全局顺序（可排序、可验证无间隙） |
| `command_id` | 追溯到触发的命令 |
| `operator_id` | 谁执行的 |
| `operator_name` | 操作员姓名快照 |
| `timestamp` | 精确时间 |
| `authorizer_*` | 授权人信息（需权限的操作） |
| `payload.source` | 变更前状态 |
| `payload.results` | 变更后状态 |

### 9.2 可回答的审计问题

- **谁** 在 **什么时间** 对 **哪个订单** 做了 **什么操作**？
- 某个商品的价格/折扣是 **谁** 改的？**谁** 授权的？
- 订单从开台到结账的 **完整时间线**？
- 两个终端的操作 **顺序** 是什么？（用 sequence）
- 某时刻订单的 **历史状态** 是什么？（重播到该 sequence）

## 10. 文件变更清单

### 10.1 后端新增/修改

| 路径 | 操作 | 说明 |
|------|------|------|
| `edge-server/src/orders/mod.rs` | 新增 | 订单模块入口 |
| `edge-server/src/orders/manager.rs` | 新增 | OrdersManager 核心 |
| `edge-server/src/orders/commands.rs` | 新增 | 命令定义 |
| `edge-server/src/orders/events.rs` | 新增 | 事件定义 |
| `edge-server/src/orders/reducer.rs` | 新增 | 状态重播器 |
| `edge-server/src/orders/storage.rs` | 新增 | redb 存储层 |
| `edge-server/src/message/processor.rs` | 修改 | 添加 OrderCommandProcessor |
| `shared/src/order/mod.rs` | 新增 | 共享类型 |
| `shared/src/order/command.rs` | 新增 | 命令类型 |
| `shared/src/order/event.rs` | 新增 | 事件类型 |

### 10.2 Tauri 层新增/修改

| 路径 | 操作 | 说明 |
|------|------|------|
| `src-tauri/src/orders/mod.rs` | 新增 | 订单模块入口 |
| `src-tauri/src/orders/bridge.rs` | 新增 | OrderBridge |
| `src-tauri/src/orders/commands.rs` | 新增 | Tauri 命令 |
| `src-tauri/src/orders/subscriber.rs` | 新增 | 事件订阅器 |
| `src-tauri/src/lib.rs` | 修改 | 注册命令 |

### 10.3 前端新增/修改

| 路径 | 操作 | 说明 |
|------|------|------|
| `src/core/stores/order/useActiveOrdersStore.ts` | 新增 | 只读镜像 Store |
| `src/core/stores/order/useOrderCommands.ts` | 新增 | 命令发送 |
| `src/core/stores/order/useOrderEventStore.ts` | 删除 | 废弃旧 Store |
| `src/core/domain/types/order.ts` | 修改 | 更新类型定义 |
| `src/core/domain/events/types.ts` | 修改 | 更新事件类型 |
| `src/App.tsx` | 修改 | 初始化事件监听 |

## 11. 迁移步骤

### Phase 1: 后端基础设施

1. 添加 redb 依赖
2. 实现 OrdersManager 核心
3. 实现命令处理器
4. 实现事件广播
5. 添加同步 API

### Phase 2: Tauri 集成

1. 实现 OrderBridge
2. 实现 Tauri 命令
3. 实现事件订阅和转发
4. 测试 Server/Client 两种模式

### Phase 3: 前端迁移

1. 创建新 Store
2. 实现事件监听
3. 实现命令发送 Hook
4. 逐个替换旧的 Store 调用
5. 删除旧代码

### Phase 4: 测试验证

1. 单元测试：命令处理、事件重播
2. 集成测试：多终端同步
3. 断线测试：重连恢复
4. 审计测试：时间线完整性

## 12. 风险与缓解

| 风险 | 缓解措施 |
|------|---------|
| redb 数据损坏 | 定期快照备份 + 事件可从 SurrealDB 归档恢复 |
| 事件广播延迟 | sequence 保证顺序，客户端可检测间隙并请求补发 |
| 幂等表膨胀 | 定期清理已归档订单的 command_id |
| 前端迁移风险 | 保留旧 Store 代码，用 feature flag 切换 |
