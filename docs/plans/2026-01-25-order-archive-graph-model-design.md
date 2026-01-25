# 订单归档图模式存储设计

## 概述

重新设计归档订单的后端存储，使用 SurrealDB 图模式，只保留核心数据，移除所有适配层。

## 设计目标

1. **财务审计** - 完整的金额、支付记录、操作员信息
2. **数据分析** - 商品统计、时段分析（另开接口）
3. **客户查询** - 重打小票、查看历史订单

## 数据模型

### 图结构

```
order (核心订单)
  ├── has_item ──> order_item (订单项)
  │                   └── has_option ──> order_item_option (选项)
  ├── has_payment ──> order_payment (支付)
  └── has_event ──> order_event (事件)
```

### order 表

| 字段 | 类型 | 说明 |
|------|------|------|
| receipt_number | string | 小票号，唯一 |
| table_name | string? | 桌台名 |
| zone_name | string? | 区域名 |
| status | enum | COMPLETED/VOID/MOVED/MERGED |
| guest_count | int | 人数 |
| is_retail | bool | 是否零售单 |
| total_amount | float | 应付总额 |
| paid_amount | float | 已付金额 |
| discount_amount | float | 折扣总额 |
| surcharge_amount | float | 附加费总额 |
| start_time | datetime | 开单时间 |
| end_time | datetime | 结账时间 |
| operator_id | string? | 操作员 ID |
| operator_name | string? | 操作员名称（快照） |
| prev_hash | string | 前一订单 hash |
| curr_hash | string | 当前订单 hash |

### order_item 表

| 字段 | 类型 | 说明 |
|------|------|------|
| spec | string | 商品规格 ID |
| instance_id | string | 实例 ID（内容寻址 hash） |
| name | string | 商品名称 |
| spec_name | string? | 规格名称 |
| price | float | 原价 |
| quantity | int | 数量 |
| unpaid_quantity | int | 未付数量（逃单场景） |
| unit_price | float | 单价（折后） |
| line_total | float | 行小计 |
| discount_amount | float | 折扣金额 |
| surcharge_amount | float | 附加费 |
| note | string? | 备注 |

### order_item_option 表

| 字段 | 类型 | 说明 |
|------|------|------|
| attribute_name | string | 属性名称（如"辣度"） |
| option_name | string | 选项名称（如"加辣"） |
| price | float | 价格修改 |

### order_payment 表

| 字段 | 类型 | 说明 |
|------|------|------|
| method | string | 支付方式 |
| amount | float | 支付金额 |
| time | datetime | 支付时间 |
| reference | string? | 参考号 |
| cancelled | bool | 是否已取消 |
| cancel_reason | string? | 取消原因 |
| split_items | array | 分单明细 |

**split_items 结构**：
```json
[
  { "instance_id": "abc123", "name": "啤酒", "quantity": 2 },
  { "instance_id": "def456", "name": "炒饭", "quantity": 1 }
]
```

### order_event 表

| 字段 | 类型 | 说明 |
|------|------|------|
| event_type | string | 事件类型 |
| timestamp | datetime | 事件时间 |
| data | object | 完整 payload |
| prev_hash | string | 前一事件 hash |
| curr_hash | string | 当前事件 hash |

## Hash 链计算

### 事件 Hash

```rust
curr_hash = SHA256(
    prev_hash +
    event_id +
    order_id +
    sequence +
    event_type +
    payload_json    // 包含完整 payload，防篡改
)
```

### 订单 Hash

```rust
order.curr_hash = SHA256(
    prev_order_hash +
    order_id +
    receipt_number +
    status +
    last_event_hash
)
```

## API 设计

### 列表接口 `GET /orders`

**请求参数**：
| 参数 | 类型 | 说明 |
|------|------|------|
| page | int | 页码（默认 1） |
| limit | int | 每页数量（默认 20） |
| search | string? | 搜索小票号 |

**响应**：
```json
{
  "orders": [
    {
      "order_id": "order:abc123",
      "receipt_number": "FAC2026012510001",
      "table_name": "A1",
      "status": "COMPLETED",
      "is_retail": false,
      "total": 128.5,
      "guest_count": 3,
      "start_time": 1737792000000,
      "end_time": 1737795600000
    }
  ],
  "total": 156,
  "page": 1,
  "limit": 20
}
```

### 详情接口 `GET /orders/:id`

**响应**（图遍历获取）：
```json
{
  "order_id": "order:abc123",
  "receipt_number": "FAC2026012510001",
  "table_name": "A1",
  "zone_name": "大厅",
  "status": "COMPLETED",
  "is_retail": false,
  "guest_count": 3,
  "total": 128.5,
  "paid_amount": 128.5,
  "total_discount": 10.0,
  "total_surcharge": 0,
  "start_time": 1737792000000,
  "end_time": 1737795600000,
  "operator_name": "张三",
  "items": [
    {
      "id": "order_item:xxx",
      "instance_id": "abc123",
      "name": "红烧肉",
      "spec_name": "大份",
      "price": 48.0,
      "quantity": 1,
      "unpaid_quantity": 0,
      "unit_price": 43.2,
      "line_total": 43.2,
      "discount_amount": 4.8,
      "surcharge_amount": 0,
      "note": null,
      "selected_options": [
        { "attribute_name": "辣度", "option_name": "微辣", "price_modifier": 0 }
      ]
    }
  ],
  "payments": [
    {
      "method": "微信",
      "amount": 128.5,
      "timestamp": 1737795600000,
      "note": null,
      "cancelled": false,
      "split_items": []
    }
  ],
  "timeline": [
    {
      "event_id": "order_event:xxx",
      "event_type": "TABLE_OPENED",
      "timestamp": 1737792000000,
      "payload": { ... }
    }
  ]
}
```

## 前端适配

### TypeScript 类型

```typescript
// 列表项（预览）
interface OrderSummary {
  order_id: string;
  receipt_number: string;
  table_name: string | null;
  status: 'COMPLETED' | 'VOID' | 'MOVED' | 'MERGED';
  is_retail: boolean;
  total: number;
  guest_count: number;
  start_time: number;
  end_time: number;
}

// 订单项
interface OrderItemDetail {
  id: string;
  instance_id: string;
  name: string;
  spec_name: string | null;
  price: number;
  quantity: number;
  unpaid_quantity: number;
  unit_price: number;
  line_total: number;
  discount_amount: number;
  surcharge_amount: number;
  note: string | null;
  selected_options: OrderItemOption[];
}

// 选项
interface OrderItemOption {
  attribute_name: string;
  option_name: string;
  price_modifier: number;
}

// 支付
interface OrderPaymentDetail {
  method: string;
  amount: number;
  timestamp: number;
  note: string | null;
  cancelled: boolean;
  cancel_reason: string | null;
  split_items: SplitItem[];
}

// 分单明细
interface SplitItem {
  instance_id: string;
  name: string;
  quantity: number;
}

// 事件
interface OrderEventDetail {
  event_id: string;
  event_type: string;
  timestamp: number;
  payload: unknown;
}

// 详情
interface OrderDetail {
  order_id: string;
  receipt_number: string;
  table_name: string | null;
  zone_name: string | null;
  status: string;
  is_retail: boolean;
  guest_count: number;
  total: number;
  paid_amount: number;
  total_discount: number;
  total_surcharge: number;
  start_time: number;
  end_time: number;
  operator_name: string | null;
  items: OrderItemDetail[];
  payments: OrderPaymentDetail[];
  timeline: OrderEventDetail[];
}
```

### Hooks 改动

| Hook | 改动 |
|------|------|
| `useHistoryOrderList` | 返回 `OrderSummary[]`，直接使用后端数据 |
| `useHistoryOrderDetail` | 返回 `OrderDetail`，直接使用后端数据 |

## 清理目标

### 移除的内容

- `snapshot_json` 字段
- `items_json` / `payments_json` 字段
- `HeldOrder` 类型及转换
- `OrderSnapshot` 到前端类型的转换
- `convertArchivedToHeldOrder` 等适配函数
- 所有适配层/兼容层代码

### 新增的内容

- 图边关系：`has_item`, `has_option`, `has_payment`
- 纯净的 `OrderSummary` / `OrderDetail` 类型
- 图遍历查询

## 实现步骤

1. **Schema** - 更新 SurrealDB schema，添加图边关系
2. **Models** - 更新 Rust 模型，移除废弃字段
3. **Archive** - 重写归档服务，使用 RELATE 创建图边
4. **API** - 更新 handler，使用图遍历查询
5. **Frontend Types** - 新建纯净类型定义
6. **Frontend Hooks** - 简化 hooks，移除转换代码
7. **Cleanup** - 删除所有废弃代码
