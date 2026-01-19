# 订单命令架构设计

> 日期: 2026-01-19
> 状态: 设计中

## 概述

订单相关操作通过 Message Bus 处理，基础资源 CRUD 通过 HTTP API 处理。

## 架构决策

### 通信模式分工

| 操作类型 | 通信方式 | 原因 |
|---------|---------|------|
| 订单操作 | Message Bus | 高频、需实时同步、多端协作 |
| 资源管理 | HTTP API | 低频、管理后台、简单 CRUD |

### 为什么订单走 Message Bus

```
HTTP 模式问题:
┌─────────┐  Request   ┌─────────┐
│ POS A   │──────────→ │ Server  │
│         │←──────────│         │
└─────────┘  Response  └─────────┘
    ❌ POS B 和厨房显示屏不知道变化，需要轮询

Message Bus 模式:
┌─────────┐  Command   ┌─────────┐  Broadcast  ┌─────────┐
│ POS A   │──────────→ │ Server  │────────────→│ POS B   │
│         │←──────────│         │────────────→│ Kitchen │
└─────────┘  Response  └─────────┘             └─────────┘
    ✅ 所有客户端实时同步
```

### 优势对比

| 特性 | HTTP | Message Bus |
|------|------|-------------|
| 多端实时同步 | ❌ 需要轮询 | ✅ 广播推送 |
| 厨房显示屏联动 | ❌ | ✅ 即时通知 |
| 离线消息队列 | ❌ | ✅ 消息排队重发 |
| 事件溯源 | 需额外实现 | ✅ 天然契合 |
| 桌台状态协作 | ❌ | ✅ 冲突检测 |
| 连接状态感知 | ❌ | ✅ 心跳检测 |

## 订单命令设计

### OrderCommand 枚举 (Rust)

```rust
/// 订单操作命令 - 通过 Message Bus 发送
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderCommand {
    // ===== 桌台操作 =====
    /// 开台
    OpenTable {
        table_id: String,
        guest_count: Option<u32>,
    },
    /// 换桌
    TransferTable {
        order_id: String,
        new_table_id: String,
    },
    /// 合并桌台
    MergeTables {
        source_order_id: String,
        target_order_id: String,
    },

    // ===== 商品操作 =====
    /// 添加商品
    AddItem {
        order_id: String,
        spec_id: String,
        quantity: u32,
        options: Vec<ItemOption>,
        note: Option<String>,
    },
    /// 修改商品数量
    UpdateItemQuantity {
        order_id: String,
        item_index: usize,
        quantity: u32,
    },
    /// 移除商品
    RemoveItem {
        order_id: String,
        item_index: usize,
        quantity: Option<u32>, // None = 全部移除
    },
    /// 修改商品备注
    UpdateItemNote {
        order_id: String,
        item_index: usize,
        note: Option<String>,
    },

    // ===== 价格调整 =====
    /// 应用折扣规则
    ApplyDiscount {
        order_id: String,
        rule_id: String,
    },
    /// 移除折扣
    RemoveDiscount {
        order_id: String,
        rule_id: String,
    },
    /// 手动调价（需权限）
    ManualPriceAdjust {
        order_id: String,
        item_index: Option<usize>, // None = 整单
        adjustment_type: AdjustmentType,
        amount: i64, // 分
        reason: String,
        authorized_by: String,
    },

    // ===== 支付操作 =====
    /// 处理支付
    ProcessPayment {
        order_id: String,
        method: PaymentMethod,
        amount: i64,
        reference: Option<String>,
    },
    /// 退款
    Refund {
        order_id: String,
        payment_index: usize,
        amount: i64,
        reason: String,
    },

    // ===== 订单状态 =====
    /// 结账（标记完成）
    CloseOrder {
        order_id: String,
    },
    /// 作废订单（需权限）
    VoidOrder {
        order_id: String,
        reason: String,
        authorized_by: String,
    },

    // ===== 厨房操作 =====
    /// 发送到厨房
    SendToKitchen {
        order_id: String,
        item_indices: Option<Vec<usize>>, // None = 全部未发送项
    },
    /// 标记出餐
    MarkItemServed {
        order_id: String,
        item_index: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemOption {
    pub attr_id: String,
    pub option_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentMethod {
    Cash,
    Card,
    WeChat,
    Alipay,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdjustmentType {
    FixedDiscount,      // 固定金额减免
    PercentageDiscount, // 百分比折扣
    FixedSurcharge,     // 固定加价
}
```

### OrderResponse (Rust)

```rust
/// 订单命令响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCommandResponse {
    pub success: bool,
    pub order_id: String,
    pub order_snapshot: Option<Order>, // 更新后的订单快照
    pub error: Option<OrderError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderError {
    OrderNotFound,
    TableOccupied,
    ItemNotFound,
    InsufficientPayment,
    PermissionDenied { required: String },
    InvalidState { current: String, expected: String },
    ConcurrencyConflict { expected_version: u64, actual_version: u64 },
}
```

### OrderBroadcast (广播给所有客户端)

```rust
/// 订单变更广播
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderBroadcast {
    /// 订单创建
    OrderCreated { order: Order },
    /// 订单更新
    OrderUpdated {
        order_id: String,
        changes: Vec<OrderChange>,
        snapshot: Order,
    },
    /// 订单关闭
    OrderClosed { order_id: String },
    /// 订单作废
    OrderVoided { order_id: String, reason: String },
    /// 厨房通知
    KitchenNotification {
        order_id: String,
        table_name: String,
        items: Vec<KitchenItem>,
    },
}
```

## 前端集成

### TypeScript 类型

```typescript
// src/core/domain/types/commands/orderCommand.ts

export type OrderCommand =
  | { type: 'OpenTable'; table_id: string; guest_count?: number }
  | { type: 'AddItem'; order_id: string; spec_id: string; quantity: number; options: ItemOption[]; note?: string }
  | { type: 'RemoveItem'; order_id: string; item_index: number; quantity?: number }
  | { type: 'ProcessPayment'; order_id: string; method: PaymentMethod; amount: number; reference?: string }
  | { type: 'CloseOrder'; order_id: string }
  // ... 其他命令
  ;

export interface ItemOption {
  attr_id: string;
  option_index: number;
}

export type PaymentMethod = 'Cash' | 'Card' | 'WeChat' | 'Alipay' | { Other: string };
```

### 前端发送命令

```typescript
// src/infrastructure/message/orderCommandClient.ts

import { MessageClient } from '@/infrastructure/message/messageClient';

export class OrderCommandClient {
  constructor(private messageClient: MessageClient) {}

  async sendCommand(command: OrderCommand): Promise<OrderCommandResponse> {
    return this.messageClient.request({
      type: 'OrderCommand',
      payload: command,
    });
  }

  // 便捷方法
  async openTable(tableId: string, guestCount?: number) {
    return this.sendCommand({
      type: 'OpenTable',
      table_id: tableId,
      guest_count: guestCount
    });
  }

  async addItem(orderId: string, specId: string, quantity: number, options: ItemOption[] = []) {
    return this.sendCommand({
      type: 'AddItem',
      order_id: orderId,
      spec_id: specId,
      quantity,
      options,
    });
  }

  async processPayment(orderId: string, method: PaymentMethod, amount: number) {
    return this.sendCommand({
      type: 'ProcessPayment',
      order_id: orderId,
      method,
      amount,
    });
  }
}
```

### 前端订阅广播

```typescript
// src/core/hooks/useOrderBroadcast.ts

export function useOrderBroadcast() {
  const messageClient = useMessageClient();

  useEffect(() => {
    const unsubscribe = messageClient.subscribe('OrderBroadcast', (broadcast) => {
      switch (broadcast.type) {
        case 'OrderCreated':
          useOrderStore.getState().addOrder(broadcast.order);
          break;
        case 'OrderUpdated':
          useOrderStore.getState().updateOrder(broadcast.order_id, broadcast.snapshot);
          break;
        case 'OrderClosed':
          useOrderStore.getState().removeOrder(broadcast.order_id);
          break;
        // ...
      }
    });

    return unsubscribe;
  }, []);
}
```

## 通信流程

### 添加商品流程

```
┌─────────────┐                    ┌─────────────┐                    ┌─────────────┐
│   POS A     │                    │ Edge Server │                    │   POS B     │
└──────┬──────┘                    └──────┬──────┘                    └──────┬──────┘
       │                                  │                                  │
       │ ── AddItem Command ────────────→ │                                  │
       │                                  │                                  │
       │                                  │ ── 验证权限                       │
       │                                  │ ── 检查库存(可选)                 │
       │                                  │ ── 计算价格(规则引擎)             │
       │                                  │ ── 更新订单                       │
       │                                  │ ── 持久化                         │
       │                                  │                                  │
       │ ←─ Response (success + snapshot) │                                  │
       │                                  │                                  │
       │                                  │ ── Broadcast: OrderUpdated ────→ │
       │ ←─ Broadcast: OrderUpdated ───── │                                  │
       │                                  │                                  │
```

### 离线处理

```
┌─────────────┐                    ┌─────────────┐
│   POS A     │                    │ Edge Server │
│  (离线)     │                    │  (不可达)   │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │ ── AddItem Command ──→ [队列]    │ ❌ 连接失败
       │                                  │
       │ (本地乐观更新 UI)                 │
       │                                  │
       ═══════════════════════════════════════════ 网络恢复
       │                                  │
       │ ── [队列重发] AddItem ─────────→ │
       │ ←─ Response ───────────────────  │
       │                                  │
       │ (同步本地状态与服务器)            │
```

## 迁移路径

### Phase 1: 当前状态 (保持)
- 订单逻辑在前端处理
- 通过 HTTP API 提交最终订单
- 无实时同步

### Phase 2: Message Bus 基础设施
- [ ] 实现 MessageClient (前端)
- [ ] 实现 OrderCommand 处理器 (后端)
- [ ] 实现广播机制

### Phase 3: 命令迁移
- [ ] OpenTable / CloseOrder
- [ ] AddItem / RemoveItem
- [ ] ProcessPayment
- [ ] 权限相关命令 (VoidOrder, ManualPriceAdjust)

### Phase 4: 前端简化
- [ ] 移除前端价格计算逻辑
- [ ] 移除前端订单状态机
- [ ] useOrderStore 变为纯只读

## HTTP API 保留范围

以下操作继续使用 HTTP API (TauriApiClient):

```typescript
// 基础资源 CRUD - 低频，管理后台使用
api.listProducts()
api.createProduct(data)
api.updateProduct(id, data)
api.deleteProduct(id)

api.listCategories()
api.listTags()
api.listZones()
api.listTables()
api.listAttributes()
api.listKitchenPrinters()
api.listEmployees()
api.listRoles()
api.listPriceRules()

// 认证
api.login(credentials)
api.logout()
api.refreshToken()

// 统计报表
api.getStatistics(params)
```

## 相关文档

- [Store 迁移笔记](./2026-01-19-store-migration-notes.md)
- [边缘服务器架构](../../CLAUDE.md)
