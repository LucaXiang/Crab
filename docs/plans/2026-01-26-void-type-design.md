# 订单作废类型设计

## 背景

当前订单作废（Void）只有一种类型，无法区分：
- 未付款取消
- 已付部分款但无法收回（逃单/无力支付）

后者需要记录损失金额用于报税。

## 设计方案

### 作废类型枚举

```rust
// shared/src/order/types.rs

/// 作废类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum VoidType {
    #[default]
    Cancelled,    // 取消订单 - 未付款，直接取消
    LossSettled,  // 损失结算 - 已付部分，剩余计入损失
}

/// 损失原因（预设选项）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LossReason {
    CustomerFled,      // 客人逃单
    CustomerInsolvent, // 客人无力支付
    Other,             // 其他
}
```

### EventPayload 更新

```rust
// shared/src/order/event.rs

OrderVoided {
    void_type: VoidType,                  // 新增：作废类型
    loss_reason: Option<LossReason>,      // 新增：损失原因（仅 LossSettled）
    loss_amount: Option<f64>,             // 新增：损失金额（仅 LossSettled）
    note: Option<String>,                 // 新增：可选备注
    authorizer_id: Option<String>,
    authorizer_name: Option<String>,
}
```

### 现金追踪逻辑

| VoidType | 条件 | 现金处理 |
|----------|------|---------|
| `Cancelled` | paid_amount == 0 | 跳过现金追踪 |
| `LossSettled` | paid_amount > 0 | 已付现金计入班次 expected_cash |

```rust
// edge-server/src/orders/archive_worker.rs

async fn update_shift_cash(&self, snapshot: &OrderSnapshot, events: &[OrderEvent]) {
    // 获取 void_type
    let void_type = events
        .iter()
        .rev()
        .find_map(|e| {
            if let EventPayload::OrderVoided { void_type, .. } = &e.payload {
                Some(void_type.clone())
            } else {
                None
            }
        });

    // Cancelled 类型跳过现金追踪
    if matches!(void_type, Some(VoidType::Cancelled)) {
        return;
    }

    // LossSettled 或其他终态：正常计入已付现金
    // ... 现有逻辑
}
```

### UI 流程

```
┌─────────────────────────────────────┐
│         作废订单                     │
├─────────────────────────────────────┤
│  订单金额: ¥100                     │
│  已付金额: ¥60    未付: ¥40         │
├─────────────────────────────────────┤
│  ○ 取消订单                         │  ← paid=0 时默认选中
│  ● 损失结算                         │  ← paid>0 时默认选中
│                                     │
│  损失原因: [客人逃单 ▼]              │  ← 仅 LossSettled 显示
│  备注: [____________________]       │  ← 可选
├─────────────────────────────────────┤
│     [ 取消 ]      [ 确认作废 ]       │
└─────────────────────────────────────┘
```

**智能默认**：
- `paid_amount == 0` → 默认选中"取消订单"，隐藏损失原因
- `paid_amount > 0` → 默认选中"损失结算"，显示损失原因选择

### 损失报表

用于报税的损失报表需要包含：

| 字段 | 来源 | 用途 |
|------|------|------|
| 日期 | event.timestamp | 报税周期 |
| 订单号 | snapshot.order_id | 追溯 |
| 损失金额 | loss_amount | 税务申报 |
| 损失原因 | loss_reason | 分类统计 |
| 操作员 | event.operator_name | 责任追踪 |
| 备注 | note | 审计说明 |

## 实现清单

### 后端 (Rust)

1. `shared/src/order/types.rs` - 添加 `VoidType`, `LossReason` 枚举
2. `shared/src/order/event.rs` - 更新 `EventPayload::OrderVoided`
3. `edge-server/src/orders/appliers/order_voided.rs` - 更新 applier
4. `edge-server/src/orders/actions/void_order.rs` - 更新 action 参数
5. `edge-server/src/orders/archive_worker.rs` - 更新现金追踪逻辑

### 前端 (TypeScript)

1. `red_coral/src/core/domain/types/api/` - 添加类型定义
2. `red_coral/src/features/orders/` - 更新作废订单弹窗
3. `red_coral/src/infrastructure/i18n/` - 添加翻译

## 兼容性

- `void_type` 默认值为 `Cancelled`，兼容旧数据
- 旧的 `reason` 字段迁移到 `note`
