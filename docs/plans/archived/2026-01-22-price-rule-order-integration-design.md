# Price Rule 订单集成设计文档

> 日期: 2026-01-22
> 状态: 待实现

---

## 一、概述

将 PriceRule（价格调整规则）集成到订单系统中，实现：
- 订单创建时加载匹配规则到内存
- add_items / modify_item 时自动应用规则
- 支持规则跳过/恢复操作
- 完整的价格明细用于小票渲染

---

## 二、数据结构设计

### 2.1 PriceRule（更新）

```rust
pub struct PriceRule {
    pub id: Option<String>,
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,
    pub description: Option<String>,

    // === 规则类型 ===
    pub rule_type: RuleType,              // Discount | Surcharge
    pub adjustment_type: AdjustmentType,  // Percentage | FixedAmount
    pub adjustment_value: i32,            // 值 (百分比: 10=10%, 固定: cents)

    // === 作用范围 ===
    pub product_scope: ProductScope,      // Global | Category | Tag | Product
    pub target: Option<String>,           // 目标ID
    pub zone_scope: i32,                  // -1=全部, 0=retail, >0=特定zone

    // === 优先级与叠加 ===
    pub priority: i32,                    // 用户定义优先级
    pub is_stackable: bool,               // 可叠加
    pub is_exclusive: bool,               // 严格排他 (独占)

    // === 时间控制 ===
    pub valid_from: Option<i64>,          // 生效时间戳 (None=立即生效)
    pub valid_until: Option<i64>,         // 过期时间戳 (None=永不过期)
    pub active_days: Option<Vec<u8>>,     // 星期几 (0=周日, None=每天)
    pub active_start_time: Option<String>,// 每天开始时间 "HH:MM"
    pub active_end_time: Option<String>,  // 每天结束时间 "HH:MM"

    // === 状态 ===
    pub is_active: bool,
    pub created_by: Option<String>,
    pub created_at: i64,                  // 创建时间 (用于同优先级排序)
}
```

**删除的字段：**
- `time_mode` - 不再需要
- `schedule_config` - 扁平化为 active_days/active_start_time/active_end_time

---

### 2.2 AppliedRule

```rust
pub struct AppliedRule {
    // === 规则标识 ===
    pub rule_id: String,
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,

    // === 规则类型 ===
    pub rule_type: RuleType,              // Discount | Surcharge
    pub adjustment_type: AdjustmentType,  // Percentage | FixedAmount

    // === 范围信息 ===
    pub product_scope: ProductScope,
    pub zone_scope: i32,

    // === 计算信息 ===
    pub adjustment_value: f64,            // 原始值
    pub calculated_amount: f64,           // 计算后金额
    pub priority: i32,
    pub is_stackable: bool,
    pub is_exclusive: bool,

    // === 控制 ===
    pub skipped: bool,                    // 是否跳过此规则
}
```

---

### 2.3 CartItemSnapshot（更新）

```rust
pub struct CartItemSnapshot {
    pub id: String,
    pub instance_id: String,
    pub name: String,
    pub original_price: f64,              // 原价 (含规格)
    pub quantity: i32,
    pub unpaid_quantity: i32,

    // === 选项 ===
    pub selected_options: Option<Vec<ItemOption>>,
    pub selected_specification: Option<SpecificationInfo>,

    // === 手动调整 ===
    pub manual_discount_percent: Option<f64>,

    // === 规则调整 ===
    pub rule_discount_amount: Option<f64>,
    pub rule_surcharge_amount: Option<f64>,
    pub applied_rules: Option<Vec<AppliedRule>>,

    // === 最终价格 ===
    pub price: f64,

    // === 其他 ===
    pub note: Option<String>,
    pub authorizer_id: Option<String>,
    pub authorizer_name: Option<String>,
}
```

---

### 2.4 OrderSnapshot（更新）

```rust
pub struct OrderSnapshot {
    pub order_id: String,
    pub table_id: Option<String>,
    pub table_name: Option<String>,
    pub zone_id: Option<String>,
    pub zone_name: Option<String>,
    pub guest_count: i32,
    pub is_retail: bool,
    pub status: OrderStatus,

    // === 商品 ===
    pub items: Vec<CartItemSnapshot>,

    // === 整单规则调整 ===
    pub order_rule_discount_amount: Option<f64>,
    pub order_rule_surcharge_amount: Option<f64>,
    pub order_applied_rules: Option<Vec<AppliedRule>>,

    // === 整单手动调整 (二选一) ===
    pub order_manual_discount_percent: Option<f64>,
    pub order_manual_discount_fixed: Option<f64>,

    // === 规则控制 ===
    pub skipped_rule_ids: Option<Vec<String>>,  // 已废弃，用 AppliedRule.skipped

    // === 金额汇总 ===
    pub subtotal: f64,
    pub discount: f64,
    pub surcharge: f64,
    pub tax: f64,
    pub total: f64,

    // === 支付 ===
    pub payments: Vec<PaymentRecord>,
    pub paid_amount: f64,
    pub paid_item_quantities: HashMap<String, i32>,

    // === 其他 ===
    pub receipt_number: Option<String>,
    pub is_pre_payment: bool,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_sequence: u64,
    pub state_checksum: String,
}
```

---

### 2.5 ItemChanges（更新）

```rust
pub struct ItemChanges {
    pub price: Option<f64>,                    // 修改原价
    pub quantity: Option<i32>,
    pub manual_discount_percent: Option<f64>,  // 修改手动折扣
    pub note: Option<String>,
    pub selected_options: Option<Vec<ItemOption>>,
    pub selected_specification: Option<SpecificationInfo>,
}
```

**删除的字段：**
- `surcharge` - 没有手动附加费
- `discount_percent` - 改名为 `manual_discount_percent`

---

### 2.6 价格明细结构

#### ItemPriceBreakdown

```rust
pub struct ItemPriceBreakdown {
    // === 基础 ===
    pub original_price: f64,
    pub options_modifier: f64,
    pub base: f64,

    // === 手动折扣 ===
    pub manual_discount_percent: f64,
    pub manual_discount_amount: f64,
    pub after_manual: f64,

    // === 规则折扣 ===
    pub rule_discount_amount: f64,
    pub after_discount: f64,

    // === 规则附加费 ===
    pub rule_surcharge_amount: f64,

    // === 最终 ===
    pub item_final: f64,

    // === 规则明细 ===
    pub applied_rules: Vec<AppliedRule>,
}
```

#### OrderPriceBreakdown

```rust
pub struct OrderPriceBreakdown {
    // === 商品汇总 ===
    pub subtotal: f64,

    // === 整单规则折扣 ===
    pub order_rule_discount_amount: f64,
    pub after_order_rule_discount: f64,

    // === 整单规则附加费 ===
    pub order_rule_surcharge_amount: f64,
    pub after_order_rule: f64,

    // === 整单手动折扣 ===
    pub order_manual_discount_type: Option<String>,  // "percent" | "fixed"
    pub order_manual_discount_value: f64,
    pub order_manual_discount_amount: f64,

    // === 最终 ===
    pub total: f64,

    // === 汇总 ===
    pub total_discount: f64,
    pub total_surcharge: f64,

    // === 规则明细 ===
    pub order_applied_rules: Vec<AppliedRule>,
}
```

---

## 三、新增 Command 和 Event

### ToggleRuleSkip

```rust
// OrderCommand
pub enum OrderCommandPayload {
    // ... 现有命令 ...

    ToggleRuleSkip {
        order_id: String,
        rule_id: String,
        skipped: bool,
    },
}

// OrderEvent
pub enum OrderEventType {
    // ... 现有事件 ...
    RuleSkipToggled,
}

pub enum EventPayload {
    // ... 现有负载 ...

    RuleSkipToggled {
        rule_id: String,
        skipped: bool,
        recalculated_amounts: RecalculatedAmounts,
    },
}

pub struct RecalculatedAmounts {
    pub item_totals: HashMap<String, f64>,
    pub subtotal: f64,
    pub discount: f64,
    pub surcharge: f64,
    pub total: f64,
}
```

---

## 四、规则加载策略

```
OpenTable 时:
  1. 根据 zone_id, is_retail 从数据库加载匹配的规则
  2. 缓存到 OrdersManager 内存中 (HashMap<order_id, Vec<PriceRule>>)
  3. 后续 add_items / modify_item 直接使用缓存

规则缓存生命周期:
  - 创建: OpenTable
  - 使用: add_items, modify_item
  - 销毁: 订单完成/作废时清理

服务重启:
  - 缓存丢失
  - 活跃订单下次操作时重新从数据库加载
```

---

## 五、优先级计算

### 隐含优先级

```rust
fn calculate_priority(rule: &PriceRule) -> i32 {
    let zone_weight = match rule.zone_scope {
        -1 => 0,           // 全局
        0 => 1,            // 零售
        _ => 2,            // 具体区域
    };

    let product_weight = match rule.product_scope {
        ProductScope::Global => 0,
        ProductScope::Category => 1,
        ProductScope::Tag => 2,
        ProductScope::Product => 3,
    };

    // 综合优先级 = 隐含优先级 × 1000 + 用户定义优先级
    (zone_weight * 10 + product_weight) * 1000 + rule.priority
}
```

### 优先级层次

```
is_exclusive=true   >   is_stackable=false   >   is_stackable=true
    (独占)                (同类互斥)               (可叠加)
```

### 同优先级处理

取 `created_at DESC`（创建时间最新的优先）

---

## 六、计算公式

### 6.1 商品级计算

```
输入:
  - original_price              // 原价 (含规格)
  - selected_options            // 选项列表
  - manual_discount_percent     // 手动折扣百分比
  - matched_rules[]             // 匹配的规则 (已过滤 skipped=true)

计算:
  // Step 0: 计算基础价
  options_modifier = Σ(option.price_modifier)
  base = original_price + options_modifier

  // Step 1: 手动折扣 (基于 base)
  after_manual = base × (1 - manual_discount_percent)

  // Step 2: 规则折扣
  discount_rules = matched_rules.filter(type=Discount, skipped=false)

  // 检查 exclusive 规则
  exclusive_rules = discount_rules.filter(is_exclusive=true)
  if exclusive_rules.not_empty():
      winner = exclusive_rules.sort_by(priority DESC, created_at DESC).first()
      // 独占，只应用这一条
      if winner.adjustment_type == Percentage:
          after_discount = after_manual × (1 - winner.adjustment_value / 100)
      else:
          after_discount = max(0, after_manual - winner.adjustment_value)
  else:
      // 正常逻辑
      non_stackable = discount_rules.filter(is_stackable=false)
      stackable = discount_rules.filter(is_stackable=true)

      winner = non_stackable.sort_by(priority DESC, created_at DESC).first()

      // 百分比叠乘
      discount_multiplier = 1.0
      if winner && winner.adjustment_type == Percentage:
          discount_multiplier *= (1 - winner.adjustment_value / 100)
      for rule in stackable.filter(adjustment_type=Percentage):
          discount_multiplier *= (1 - rule.adjustment_value / 100)

      after_percent_discount = after_manual × discount_multiplier

      // 固定金额直接减
      fixed_discount = 0
      if winner && winner.adjustment_type == FixedAmount:
          fixed_discount += winner.adjustment_value
      for rule in stackable.filter(adjustment_type=FixedAmount):
          fixed_discount += rule.adjustment_value

      after_discount = max(0, after_percent_discount - fixed_discount)

  // Step 3: 规则附加费 (基于 base，最后加)
  surcharge_rules = matched_rules.filter(type=Surcharge, skipped=false)

  // 同样检查 exclusive
  exclusive_surcharges = surcharge_rules.filter(is_exclusive=true)
  if exclusive_surcharges.not_empty():
      winner = exclusive_surcharges.sort_by(priority DESC, created_at DESC).first()
      if winner.adjustment_type == Percentage:
          surcharge_total = base × winner.adjustment_value / 100
      else:
          surcharge_total = winner.adjustment_value
  else:
      non_stackable = surcharge_rules.filter(is_stackable=false)
      stackable = surcharge_rules.filter(is_stackable=true)

      winner = non_stackable.sort_by(priority DESC, created_at DESC).first()

      // 百分比叠乘
      surcharge_multiplier = 1.0
      if winner && winner.adjustment_type == Percentage:
          surcharge_multiplier *= (1 + winner.adjustment_value / 100)
      for rule in stackable.filter(adjustment_type=Percentage):
          surcharge_multiplier *= (1 + rule.adjustment_value / 100)

      surcharge_from_percent = base × surcharge_multiplier - base

      // 固定金额直接加
      fixed_surcharge = 0
      if winner && winner.adjustment_type == FixedAmount:
          fixed_surcharge += winner.adjustment_value
      for rule in stackable.filter(adjustment_type=FixedAmount):
          fixed_surcharge += rule.adjustment_value

      surcharge_total = surcharge_from_percent + fixed_surcharge

  // Step 4: 最终价格 (唯一舍入点)
  item_final = round(max(0, after_discount + surcharge_total), 2)

  // 存储计算结果
  rule_discount_amount = round(after_manual - after_discount, 2)
  rule_surcharge_amount = round(surcharge_total, 2)
```

---

### 6.2 整单级计算

```
输入:
  - items[]                           // 所有商品 (已计算 item_final)
  - order_matched_rules[]             // 整单匹配的规则
  - order_manual_discount_percent     // 整单手动折扣百分比 (二选一)
  - order_manual_discount_fixed       // 整单手动折扣固定金额 (二选一)

计算:
  // Step 1: 商品小计
  subtotal = Σ(item.item_final × item.quantity)

  // Step 2: 整单规则折扣 (基于 subtotal)
  // 逻辑同商品级，使用 order_matched_rules
  after_order_rule_discount = ... // 同上
  order_rule_discount_amount = subtotal - after_order_rule_discount

  // Step 3: 整单规则附加费 (基于 subtotal)
  order_rule_surcharge_amount = ... // 同上
  after_order_rule = after_order_rule_discount + order_rule_surcharge_amount

  // Step 4: 整单手动折扣 (基于 after_order_rule)
  if order_manual_discount_percent != null:
      order_manual_discount = after_order_rule × order_manual_discount_percent
  else if order_manual_discount_fixed != null:
      order_manual_discount = order_manual_discount_fixed
  else:
      order_manual_discount = 0

  after_manual_discount = max(0, after_order_rule - order_manual_discount)

  // Step 5: 最终总价 (唯一舍入点)
  order_total = round(max(0, after_manual_discount), 2)

  // 汇总
  discount = round(order_rule_discount_amount + order_manual_discount, 2)
  surcharge = round(order_rule_surcharge_amount, 2)
```

---

## 七、modify_item 处理流程

```
1. 应用变更到 item
   - price → original_price
   - quantity → quantity
   - manual_discount_percent → manual_discount_percent
   - selected_options → selected_options
   - selected_specification → selected_specification
   - note → note

2. 触发规则重算的条件 (任一变更):
   - price
   - selected_options
   - selected_specification
   - manual_discount_percent

3. 重新计算流程:
   - 从缓存获取规则
   - 匹配适用规则
   - 执行商品级计算
   - 更新 item_final 和 applied_rules
   - 重算整单金额
```

---

## 八、小票渲染示例

```
┌─────────────────────────────────────────┐
│ 红烧肉 (大份)                    ¥120.00│
│   + 加辣                          +¥5.00│
│   基础价                         ¥125.00│
│   手动折扣 (10%)                 -¥12.50│
│   午市折扣 (10%)                 -¥11.25│
│   VIP包厢费 (10%)                +¥12.50│
│   小计                           ¥113.75│
├─────────────────────────────────────────┤
│ 小炒肉                            ¥50.00│
├─────────────────────────────────────────┤
│ 商品合计                         ¥163.75│
│ 满100减10                        -¥10.00│
│ 整单手动折扣                      -¥5.00│
├─────────────────────────────────────────┤
│ 应付                             ¥148.75│
└─────────────────────────────────────────┘
```

---

## 九、实现计划

### Phase 1: 数据结构更新
- [ ] 更新 PriceRule 字段 (时间控制、is_exclusive)
- [ ] 更新 CartItemSnapshot 字段
- [ ] 更新 OrderSnapshot 字段
- [ ] 新增 AppliedRule 结构
- [ ] 新增价格明细结构

### Phase 2: 规则加载
- [ ] OrdersManager 添加规则缓存 (HashMap)
- [ ] OpenTable 时加载规则
- [ ] 订单完成/作废时清理缓存
- [ ] 服务重启后延迟加载

### Phase 3: 计算引擎
- [ ] 实现商品级计算公式
- [ ] 实现整单级计算公式
- [ ] 实现优先级计算
- [ ] 实现 exclusive/stackable 逻辑

### Phase 4: 命令集成
- [ ] add_items 集成规则计算
- [ ] modify_item 集成规则重算
- [ ] 新增 ToggleRuleSkip 命令
- [ ] 新增 RuleSkipToggled 事件

### Phase 5: API 响应
- [ ] 返回 ItemPriceBreakdown
- [ ] 返回 OrderPriceBreakdown
- [ ] 支持小票渲染需要的明细

---

## 十、测试用例

### 基础场景
- [ ] 单商品无规则
- [ ] 单商品单规则 (百分比折扣)
- [ ] 单商品单规则 (固定金额折扣)
- [ ] 单商品单规则 (附加费)

### 叠加场景
- [ ] 多规则叠乘 (stackable)
- [ ] non-stackable 竞争
- [ ] exclusive 独占
- [ ] 手动折扣 + 规则折扣

### 整单场景
- [ ] 整单规则折扣
- [ ] 整单手动折扣 (百分比)
- [ ] 整单手动折扣 (固定)
- [ ] 商品级 + 整单级组合

### 边界场景
- [ ] 负数保护
- [ ] 同优先级处理
- [ ] 规则跳过/恢复
- [ ] 舍入精度验证
