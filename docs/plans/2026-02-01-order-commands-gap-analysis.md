# OrderManager Command 缺失审查与设计方案

**日期**: 2026-02-01
**范围**: OrderManager 活跃订单 Command/Event 体系完整性审查

---

## 审查背景

对现有 18 个 Command、22 个 Event 进行系统完整性审查，识别缺失的 Command 并设计实现方案。

### 已有 Command 清单

- **生命周期**: OpenTable, CompleteOrder, VoidOrder
- **菜品操作**: AddItems, ModifyItem, RemoveItem, RestoreItem
- **支付**: AddPayment, CancelPayment
- **分单**: SplitByItems, SplitByAmount, StartAaSplit, PayAaSplit
- **桌台**: MoveOrder, MergeOrders
- **其他**: UpdateOrderInfo, ToggleRuleSkip

### 审查结论：不需要的 Command

| 场景 | 结论 | 理由 |
|------|------|------|
| CancelAaSplit | 不需要 | 通过逐个 CancelPayment 自动退出 AA 模式 |
| AdjustItemQuantity | 不需要 | AddItems + RemoveItem 已覆盖 |
| Refund | 不在范围 | OrderManager 只管活跃订单，归档后卸载 |
| TransferItems | 不需要 | SplitByItems 分单支付已覆盖场景 |
| RecalculateRules | 不需要 | 规则对已有订单保持不变（见变更 1） |
| Tip | 暂不需要 | 当前业务不需要小费功能 |
| MarkItemServed | 不是 Order 职责 | 出品状态由厨房系统单独管理 |

---

## 变更 1：价格规则开台定格（优先级：最高）

### 问题

当前规则缓存是内存 `HashMap<order_id, Vec<PriceRule>>`，重启后从数据库重新加载。如果规则在断电期间被修改，重启后活跃订单会拿到不同版本的规则，导致：

1. 同一订单内新旧菜品使用不同规则版本
2. 多节点间哈希链不一致

### 设计意图

**开台时价格规则就定格**——订单生命周期内规则不变。

### 方案：redb 持久化规则快照

```
OpenTable 时:
  1. 从数据库加载匹配规则 (load_matching_rules)
  2. 存入 redb 规则快照表 (key: order_id → Vec<PriceRule>)
  3. 放入内存缓存

重启预热时:
  1. 从 redb 恢复规则快照 (不查数据库)
  2. 放入内存缓存

订单终结时 (Complete/Void/Move/Merge):
  1. 清除内存缓存
  2. 清除 redb 中的规则快照
```

### 影响范围

| 文件 | 变更 |
|------|------|
| `edge-server/src/orders/storage.rs` | 新增 redb 规则快照表 + CRUD |
| `edge-server/src/orders/manager.rs` | `cache_rules` 同时写 redb；启动预热从 redb 恢复 |
| `edge-server/src/core/state.rs` | `warmup_price_rules` 从 redb 读取而非数据库 |
| `edge-server/src/message/processor.rs` | 远程同步时检查 redb 是否已有规则 |

---

## 变更 2a：ApplyOrderDiscount（订单级手动折扣）

### Command

```rust
ApplyOrderDiscount {
    order_id: String,
    discount_percent: Option<f64>,  // 百分比折扣 (0-100)，None = 清除
    discount_fixed: Option<f64>,    // 固定金额折扣，None = 清除
    // percent 和 fixed 互斥，都为 None = 取消折扣
    reason: Option<String>,
    authorizer_id: Option<String>,
    authorizer_name: Option<String>,
}
```

### Event

```rust
OrderDiscountApplied {
    discount_percent: Option<f64>,
    discount_fixed: Option<f64>,
    previous_discount_percent: Option<f64>,
    previous_discount_fixed: Option<f64>,
    reason: Option<String>,
    authorizer_id: Option<String>,
    authorizer_name: Option<String>,
    // 应用后的金额
    subtotal: f64,
    discount: f64,
    total: f64,
}
```

### Applier

更新 `snapshot.order_manual_discount_percent` / `snapshot.order_manual_discount_fixed`，调用 `recalculate_totals()`。

### 备注

- 与自动规则折扣（`order_rule_discount_amount`）共存
- 与订单附加费（变更 2b）共存
- 需要授权（经理权限）

---

## 变更 2b：ApplyOrderSurcharge（订单级手动附加费）

### Command

```rust
ApplyOrderSurcharge {
    order_id: String,
    surcharge_amount: Option<f64>,  // 固定附加费金额，None = 清除
    reason: Option<String>,
    authorizer_id: Option<String>,
    authorizer_name: Option<String>,
}
```

### Event

```rust
OrderSurchargeApplied {
    surcharge_amount: Option<f64>,
    previous_surcharge_amount: Option<f64>,
    reason: Option<String>,
    authorizer_id: Option<String>,
    authorizer_name: Option<String>,
    subtotal: f64,
    surcharge: f64,
    total: f64,
}
```

### Applier

更新 `snapshot.order_manual_surcharge_fixed`，调用 `recalculate_totals()`。

### OrderSnapshot 变更

新增字段：

```rust
pub order_manual_surcharge_fixed: Option<f64>,
```

---

## 变更 3：AddOrderNote（订单备注）

### Command

```rust
AddOrderNote {
    order_id: String,
    note: String,  // 备注内容，空字符串 = 清除备注
}
```

### Event

```rust
OrderNoteAdded {
    note: String,
    previous_note: Option<String>,
}
```

### Applier

```rust
snapshot.note = if note.is_empty() { None } else { Some(note) };
```

### OrderSnapshot 变更

新增字段：

```rust
pub note: Option<String>,
```

### 备注

- 覆盖式（不是追加）
- 不需要授权人
- 不影响金额计算

---

## 变更 4：CompItem（赠送菜品）

### Command

```rust
CompItem {
    order_id: String,
    instance_id: String,
    quantity: i32,              // 赠送数量（可以部分赠送）
    reason: String,             // 赠送原因（必填）
    authorizer_id: String,      // 授权人（必填）
    authorizer_name: String,
}
```

### Event

```rust
ItemComped {
    instance_id: String,
    item_name: String,
    quantity: i32,
    reason: String,
    authorizer_id: String,
    authorizer_name: String,
}
```

### Applier

标记菜品 `is_comped = true`，将价格归零，调用 `recalculate_totals()`。

### CartItemSnapshot 变更

新增字段：

```rust
pub is_comped: bool,
```

### 备注

- 与折扣语义不同：折扣是减价，comp 是免费赠送
- 必须有授权人和原因（审计追踪）
- comp 后菜品仍然在订单中，但价格为 0
- 如果部分赠送（quantity < item.quantity），需要拆分 item

---

## 变更 5：RushItem（催菜）

### Command

```rust
RushItem {
    order_id: String,
    instance_id: String,
    note: Option<String>,       // 催菜附加说明
}
```

### Event

```rust
ItemRushed {
    instance_id: String,
    item_name: String,
    note: Option<String>,
}
```

### Applier

**No-op** — 不修改快照状态。

### EventRouter 分发

`ItemRushed` 事件分发给打印系统，打印催菜单。分发类型：**尽力 (丢弃)**，与 ItemsAdded 相同。

### 备注

- 纯事件驱动，不改变订单状态
- 同一道菜可以多次催菜（每次生成新事件）
- 不需要授权人

---

## 变更 6：service_type 移至 CompleteOrder

### 变更内容

| 位置 | 变更 |
|------|------|
| `OpenTable` command | 移除 `service_type` 字段 |
| `TableOpened` event | 移除 `service_type` 字段 |
| `CompleteOrder` command | 新增 `service_type: ServiceType` 字段（必填） |
| `OrderCompleted` event | 新增 `service_type: ServiceType` 字段 |
| `OrderSnapshot` | `service_type` 默认 `None`，结单时设置 |

### 影响范围

| 文件 | 变更 |
|------|------|
| `shared/src/order/command.rs` | OpenTable 移除、CompleteOrder 新增 |
| `shared/src/order/event.rs` | TableOpened 移除、OrderCompleted 新增 |
| `edge-server/src/orders/actions/open_table.rs` | 移除 service_type 处理 |
| `edge-server/src/orders/actions/complete_order.rs` | 新增 service_type 设置 |
| `edge-server/src/orders/appliers/table_opened.rs` | 移除 service_type |
| `edge-server/src/orders/appliers/order_completed.rs` | 新增 service_type |
| `red_coral/` 前端 | 开台界面移除、结单界面新增 service_type 选择 |

---

## 实现优先级建议

1. **变更 1：规则定格** — 影响数据一致性和哈希链，最高优先级
2. **变更 6：service_type** — 涉及现有结构变更，趁早做
3. **变更 2a/2b：订单折扣/附加费** — 字段已存在但无 Command
4. **变更 4：CompItem** — 独立新增，不影响现有逻辑
5. **变更 3：AddOrderNote** — 最简单，独立新增
6. **变更 5：RushItem** — 最简单，纯事件驱动
