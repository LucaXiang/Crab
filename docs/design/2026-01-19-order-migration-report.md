# 订单系统迁移报告

> 日期: 2026-01-19
> 从: 乐观更新 + localStorage
> 到: 服务端状态 + 事件溯源 + MessageBus

## 1. 迁移概述

### 1.1 当前架构问题

| 问题 | 影响 |
|------|------|
| 乐观更新 | 多终端可能状态不一致 |
| localStorage 存储 | 数据仅在单终端，无法同步 |
| 前端生成 ID | 可能冲突，审计不清晰 |
| 无全局序列号 | 无法确定操作顺序 |
| 断线无法恢复 | 丢失断线期间的操作 |

### 1.2 新架构优势

| 特性 | 收益 |
|------|------|
| 服务端状态 | 多终端实时同步 |
| 事件溯源 | 完整审计追溯 |
| redb 存储 | 高性能 + 事务保证 |
| 全局 sequence | 明确操作顺序 |
| 断线重播 | 零丢失恢复 |
| 幂等性 | 防止重复操作 |

## 2. 前端迁移详情

### 2.1 废弃的文件

| 文件路径 | 原因 |
|----------|------|
| `src/core/stores/order/useOrderEventStore.ts` | 整体重写为服务端状态镜像 |
| `src/core/stores/order/useDraftOrderStore.ts` | 草稿功能待重新设计 |

### 2.2 新增的文件

| 文件路径 | 职责 |
|----------|------|
| `src/core/stores/order/useActiveOrdersStore.ts` | 只读订单状态镜像 |
| `src/core/stores/order/useOrderCommands.ts` | 命令发送 Hook |
| `src/core/stores/order/useOrderSync.ts` | 断线重连同步 |
| `src/core/stores/order/orderReducer.ts` | 事件 → 状态计算 |

### 2.3 修改的文件

| 文件路径 | 变更内容 |
|----------|----------|
| `src/App.tsx` | 添加事件监听初始化 |
| `src/core/domain/types/index.ts` | 更新类型定义 |
| `src/core/domain/events/types.ts` | 更新事件类型 |
| `src/core/domain/events/adapters.ts` | 适配新事件结构 |
| `src/presentation/components/shared/Timeline/useTimelineEvent.tsx` | 适配新 payload |

### 2.4 Store 方法映射

#### useOrderEventStore → useOrderCommands

| 旧方法 | 新方法 | 变化 |
|--------|--------|------|
| `openTable(params)` | `openTable(params): Promise<CommandResponse>` | 返回 Promise，等待 ACK |
| `addItems(orderKey, items)` | `addItems(orderId, items): Promise<CommandResponse>` | orderId 由服务端分配 |
| `modifyItem(orderKey, instanceId, changes)` | `modifyItem(orderId, instanceId, changes): Promise<CommandResponse>` | 异步确认 |
| `removeItem(orderKey, instanceId, reason)` | `removeItem(orderId, instanceId, quantity, reason): Promise<CommandResponse>` | 支持部分删除 |
| `completeOrder(orderKey, receiptNumber)` | `completeOrder(orderId, receiptNumber): Promise<CommandResponse>` | 异步确认 |
| `voidOrder(orderKey, reason)` | `voidOrder(orderId, reason): Promise<CommandResponse>` | 异步确认 |
| `mergeOrder(target, source)` | `mergeOrders(sourceId, targetId): Promise<CommandResponse>` | 参数顺序调整 |
| `moveOrder(sourceKey, targetTable)` | `moveOrder(orderId, targetTableId, targetTableName): Promise<CommandResponse>` | 更明确的参数 |
| `addPayment(orderKey, payment)` | `addPayment(orderId, payment): Promise<CommandResponse>` | 异步确认 |
| `cancelPayment(orderKey, paymentId, reason)` | `cancelPayment(orderId, paymentId, reason): Promise<CommandResponse>` | 异步确认 |
| `setSurchargeExempt(orderKey, exempt)` | `setSurchargeExempt(orderId, exempt): Promise<CommandResponse>` | 异步确认 |

#### useOrderEventStore → useActiveOrdersStore

| 旧方法 | 新方法 | 变化 |
|--------|--------|------|
| `getOrder(orderKey)` | `getOrder(orderId)` | 参数名变化 |
| `getActiveOrders()` | `getActiveOrders()` | 无变化 |
| `getOrderEvents(orderKey)` | `invoke('get_order_events', { orderId })` | 改为 Tauri 命令 |
| `hydrateActiveFromLocalStorage()` | 删除 | 改为从服务端同步 |

### 2.5 UI 层调用点修改

#### POS 主界面

```typescript
// 旧代码
const { addItems } = useOrderEventStore();
addItems(orderKey, items);

// 新代码
const { addItems } = useOrderCommands();
const response = await addItems(orderId, items);
if (!response.success) {
  toast.error(response.error?.message);
}
```

#### 结账界面

```typescript
// 旧代码
const { completeOrder, addPayment } = useOrderEventStore();
addPayment(orderKey, payment);
completeOrder(orderKey, receiptNumber);

// 新代码
const { addPayment, completeOrder } = useOrderCommands();
const paymentRes = await addPayment(orderId, payment);
if (!paymentRes.success) return;

const completeRes = await completeOrder(orderId, receiptNumber);
if (!completeRes.success) {
  toast.error(completeRes.error?.message);
}
```

#### 桌台管理

```typescript
// 旧代码
const { mergeOrder, moveOrder, setSurchargeExempt } = useOrderEventStore();
mergeOrder(targetOrder, sourceOrder);

// 新代码
const { mergeOrders, moveOrder, setSurchargeExempt } = useOrderCommands();
const response = await mergeOrders(sourceOrder.id, targetOrder.id);
if (!response.success) {
  toast.error(response.error?.message);
}
```

### 2.6 事件监听初始化

```typescript
// src/App.tsx
import { listen } from '@tauri-apps/api/event';
import { useActiveOrdersStore } from '@/core/stores/order/useActiveOrdersStore';

function App() {
  useEffect(() => {
    // 初始化事件监听
    const unlisten = listen<OrderEvent>('order-event', (event) => {
      useActiveOrdersStore.getState()._applyEvent(event.payload);
    });

    // 初始化连接状态监听
    const unlistenConnection = listen<'connected' | 'disconnected'>('order-connection', (event) => {
      useActiveOrdersStore.getState()._setConnectionState(event.payload);
    });

    // 初始加载活跃订单
    invoke<SyncResponse>('sync_orders', { sinceSequence: 0 }).then((response) => {
      useActiveOrdersStore.getState()._fullSync(response.active_orders, response.server_sequence);
    });

    return () => {
      unlisten.then(f => f());
      unlistenConnection.then(f => f());
    };
  }, []);

  // ...
}
```

## 3. 后端迁移详情

### 3.1 新增的 Rust 模块

#### edge-server/src/orders/

```
orders/
├── mod.rs              # 模块导出
├── manager.rs          # OrdersManager 核心
├── commands.rs         # 命令处理
├── events.rs           # 事件生成
├── reducer.rs          # 状态重播
├── storage.rs          # redb 存储层
└── sync.rs             # 同步 API
```

#### shared/src/order/

```
order/
├── mod.rs              # 模块导出
├── command.rs          # 命令类型定义
├── event.rs            # 事件类型定义
└── snapshot.rs         # 快照类型定义
```

### 3.2 Cargo.toml 变更

```toml
# edge-server/Cargo.toml
[dependencies]
redb = "2.0"  # 新增
```

### 3.3 MessageBus 集成

```rust
// edge-server/src/message/processor.rs

pub struct OrderCommandProcessor {
    orders_manager: Arc<OrdersManager>,
}

#[async_trait]
impl MessageProcessor for OrderCommandProcessor {
    fn event_type(&self) -> EventType {
        EventType::RequestCommand
    }

    async fn process(&self, msg: &BusMessage) -> Result<ProcessResult> {
        let payload: RequestCommandPayload = msg.payload()?;

        if payload.action != "order_command" {
            return Ok(ProcessResult::Skip);
        }

        let command: OrderCommand = serde_json::from_value(payload.params.unwrap())?;
        let response = self.orders_manager.execute_command(command)?;

        Ok(ProcessResult::Respond(serde_json::to_value(response)?))
    }
}
```

### 3.4 Tauri 层新增

```rust
// src-tauri/src/orders/commands.rs

#[tauri::command]
pub async fn send_order_command(
    bridge: State<'_, OrderBridge>,
    command: OrderCommand,
) -> Result<CommandResponse, String> {
    bridge.send_command(command).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_orders(
    bridge: State<'_, OrderBridge>,
    since_sequence: u64,
) -> Result<SyncResponse, String> {
    bridge.sync(since_sequence).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_order_events(
    bridge: State<'_, OrderBridge>,
    order_id: String,
) -> Result<Vec<OrderEvent>, String> {
    bridge.get_events(&order_id).await.map_err(|e| e.to_string())
}
```

## 4. 数据迁移

### 4.1 现有订单处理

迁移时，现有 localStorage 中的活跃订单需要：

1. **Server 模式首次启动**：
   - 检测 localStorage 中的活跃订单
   - 转换为命令序列发送到服务端
   - 清理 localStorage

2. **或者选择**：
   - 要求用户在迁移前完成所有活跃订单
   - 迁移后从空状态开始

### 4.2 历史订单

已归档到 SurrealDB 的历史订单不受影响，继续保持查询能力。

## 5. 测试计划

### 5.1 单元测试

| 测试项 | 文件 |
|--------|------|
| 命令处理 | `orders/manager_test.rs` |
| 事件重播 | `orders/reducer_test.rs` |
| 幂等性 | `orders/idempotency_test.rs` |
| 快照一致性 | `orders/snapshot_test.rs` |

### 5.2 集成测试

| 测试项 | 描述 |
|--------|------|
| 多终端同步 | 两个终端同时操作同一订单 |
| 断线重播 | 模拟断线，验证重连后状态一致 |
| 并发冲突 | 两个终端同时结账同一订单 |
| 事件顺序 | 验证所有终端看到相同的事件顺序 |

### 5.3 UI 回归测试

| 测试项 | 描述 |
|--------|------|
| 开台流程 | 选择桌台 → 输入客人数 → 创建订单 |
| 加菜流程 | 选择商品 → 配置选项 → 添加到订单 |
| 编辑商品 | 修改数量、折扣、移除 |
| 结账流程 | 选择支付方式 → 完成支付 → 打印收据 |
| 作废流程 | 作废订单 → 输入原因 |
| 桌台管理 | 合并、移动、豁免服务费 |
| 分账流程 | 选择商品 → 分账支付 |
| 断线处理 | 断线时 UI 阻断，重连后恢复 |

## 6. 回滚计划

### 6.1 Feature Flag

```typescript
// src/core/config.ts
export const USE_EVENT_SOURCING = import.meta.env.VITE_USE_EVENT_SOURCING === 'true';

// 使用示例
if (USE_EVENT_SOURCING) {
  // 新架构
  const { addItems } = useOrderCommands();
} else {
  // 旧架构
  const { addItems } = useOrderEventStore();
}
```

### 6.2 保留旧代码

迁移完成后保留旧 Store 代码 2 个版本周期，确认稳定后再删除。

## 7. 时间线

| 阶段 | 内容 | 依赖 |
|------|------|------|
| **Phase 1** | 后端 OrdersManager + redb 存储 | - |
| **Phase 2** | MessageBus 集成 + 同步 API | Phase 1 |
| **Phase 3** | Tauri 层 OrderBridge | Phase 2 |
| **Phase 4** | 前端新 Store + 命令 Hook | Phase 3 |
| **Phase 5** | UI 层调用点迁移 | Phase 4 |
| **Phase 6** | 测试 + 修复 | Phase 5 |
| **Phase 7** | 清理旧代码 | Phase 6 稳定后 |

## 8. 检查清单

### 8.1 后端完成标准

- [ ] OrdersManager 实现完成
- [ ] 所有命令类型处理实现
- [ ] 所有事件类型生成实现
- [ ] redb 存储层实现
- [ ] 幂等性检查实现
- [ ] 同步 API 实现
- [ ] 事件广播实现
- [ ] 单元测试通过

### 8.2 Tauri 层完成标准

- [ ] OrderBridge 实现完成
- [ ] Server 模式 MemoryTransport 集成
- [ ] Client 模式 TlsTransport 集成
- [ ] Tauri 命令注册
- [ ] 事件订阅和转发实现
- [ ] 连接状态管理

### 8.3 前端完成标准

- [ ] useActiveOrdersStore 实现完成
- [ ] useOrderCommands 实现完成
- [ ] 事件监听初始化
- [ ] 断线重连同步
- [ ] 所有 UI 调用点迁移
- [ ] 时间线适配
- [ ] 断线 UI 阻断
- [ ] UI 回归测试通过

### 8.4 审计验证

- [ ] 每个事件包含完整审计字段
- [ ] 时间线显示正确
- [ ] 可追溯操作员和授权人
- [ ] 事件序列号连续无间隙
- [ ] 快照与事件重播结果一致
