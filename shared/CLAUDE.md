# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Shared

跨 crate 共享类型、协议定义、错误系统、订单事件溯源定义。

## 命令

```bash
cargo check -p shared
cargo test -p shared --lib
```

## 模块结构

```
src/
├── error/          # 统一错误系统
│   ├── codes.rs        # ErrorCode 枚举 (按领域分区 0xxx-9xxx)
│   ├── types.rs        # AppError + ApiResponse<T>
│   ├── category.rs     # ErrorCategory 分类
│   └── http.rs         # HTTP 状态码映射
├── order/          # 订单事件溯源类型
│   ├── command.rs      # OrderCommand + OrderCommandPayload
│   ├── event.rs        # OrderEvent + OrderEventType + EventPayload
│   ├── snapshot.rs     # OrderSnapshot + OrderStatus
│   ├── types.rs        # CartItemSnapshot, PaymentRecord, VoidType 等
│   └── applied_rule.rs # AppliedRule (价格规则追踪)
├── models/         # 数据模型 (与前端 TypeScript 对齐)
│   ├── product.rs      # Product + EmbeddedSpec
│   ├── category.rs     # Category
│   ├── tag.rs          # Tag
│   ├── attribute.rs    # Attribute
│   ├── zone.rs         # Zone
│   ├── dining_table.rs # DiningTable
│   ├── employee.rs     # Employee
│   ├── shift.rs        # Shift
│   ├── daily_report.rs # DailyReport
│   ├── store_info.rs   # StoreInfo
│   ├── system_state.rs # SystemState
│   ├── price_rule.rs   # PriceRule + RuleType/ProductScope/AdjustmentType
│   ├── print_destination.rs # PrintDestination
│   ├── label_template.rs   # LabelTemplate
│   └── sync.rs         # SyncPayload + SyncStatus
├── message/        # 消息总线协议
│   ├── mod.rs          # Message<T>, EventType, BusMessage
│   └── payload.rs      # Notification, ServerCommand, SyncPayload, Response
├── activation.rs   # 激活协议 (ActivationResponse, SignedBinding, SubscriptionInfo)
├── app_state.rs    # 应用状态 (HealthStatus, ActivationProgress, SubscriptionBlocked)
├── client.rs       # 客户端类型
├── request.rs      # 请求类型
├── types.rs        # 通用类型 (UserRole, Permission, Timestamp=i64)
└── util.rs         # 工具函数
```

## 核心类型

### ErrorCode (错误码分区)

```
0xxx  General     (Success=0, ValidationError=1001, NotFound=1002)
1xxx  Auth        (NotAuthenticated, TokenExpired, InvalidCredentials)
2xxx  Permission  (PermissionDenied, InsufficientRole)
3xxx  Tenant      (TenantNotFound, ActivationRequired, SubscriptionExpired)
4xxx  Order       (OrderNotFound, OrderAlreadyCompleted, InvalidOrderState)
5xxx  Payment     (PaymentFailed, InsufficientPayment)
6xxx  Product     (ProductNotFound, CategoryNotFound)
7xxx  Table       (TableNotFound, TableOccupied)
8xxx  Employee    (EmployeeNotFound, ShiftNotOpen)
9xxx  System      (DatabaseError, InternalError, StorageError)
```

### 订单事件溯源

**OrderCommand** → `OrderCommandPayload`:
- 生命周期: OpenTable, CompleteOrder, VoidOrder
- 商品: AddItems, ModifyItem, RemoveItem, RestoreItem, CompItem, UncompItem
- 支付: AddPayment, CancelPayment
- 拆分: SplitByItems, SplitByAmount, StartAaSplit, PayAaSplit
- 桌台: MoveOrder, MergeOrders
- 整单调价: ApplyOrderDiscount, ApplyOrderSurcharge
- 其他: UpdateOrderInfo, AddOrderNote, ToggleRuleSkip

**OrderEvent** → `OrderEventType` + `EventPayload`:
- 每个事件有 `event_id`, `sequence` (服务端严格有序), `timestamp` (服务端权威)
- EventPayload 与 CommandPayload 一一对应

**OrderSnapshot** (完整订单状态):
- 状态: `OrderStatus` (Active, Completed, Void, Merged)
- 作废: `VoidType` (Cancelled, LossSettled) + `LossReason`
- 服务: `ServiceType` (DineIn, Takeout)
- 金额: original_total, subtotal, discount, surcharge, tax, total, paid, remaining
- 规则: order_applied_rules, order_rule_discount/surcharge_amount
- 校验: state_checksum (SHA256)

### 消息总线

**EventType**: Handshake, Notification, ServerCommand, RequestCommand, Sync, Response

**SyncPayload**: `{ resource, version, action, id, data }`
- action: "created" / "updated" / "deleted"
- version: 自动递增 (ResourceVersions)

### 激活/订阅系统

**SubscriptionStatus**: Inactive, Active, PastDue, Expired, Canceled, Unpaid
**PlanType**: Basic (1店), Pro (3店), Enterprise (无限)
**SignedBinding**: 硬件绑定 + 时钟篡改检测 (±1小时/30天)

### 应用状态

**ActivationRequiredReason**: FirstTimeSetup, CertificateExpired, DeviceMismatch, ClockTampering 等
**HealthLevel**: Healthy, Warning, Critical, Unknown

## 类型对齐

修改 `models/` 时，必须同步更新:
- 前端: `red_coral/src/core/domain/types/api/models.ts`
- 验证: `cargo check && npx tsc --noEmit`

**关键约定**:
- 时间戳: `i64` Unix 毫秒 (非 string)
- ID: `String` 格式 `"table:id"` (对应 SurrealDB RecordId)
- 金额: 服务端 `rust_decimal`，前端 `decimal.js`
- 枚举序列化: `SCREAMING_SNAKE_CASE`
- 可选字段: `#[serde(skip_serializing_if = "Option::is_none")]`

## 响应语言

使用中文回答。
