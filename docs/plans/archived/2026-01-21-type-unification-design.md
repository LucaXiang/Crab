# 类型系统统一设计方案

**日期**: 2026-01-21
**目标**: 彻底统一前后端类型系统，服务端权威，零适配层

---

## 一、核心原则

### 1.1 服务端权威（Server Authority）

**单一真理源（Single Source of Truth）**：
- Rust 后端类型是唯一的标准定义
- TypeScript 前端完全对齐后端类型
- 所有字段使用 snake_case（Rust 标准）
- 零字段名转换，零类型适配器

**架构图**：
```
┌─────────────────────────────────────┐
│   Rust Backend (唯一标准)            │
│   shared/src/                       │
│   ├── order/event.rs                │
│   ├── order/snapshot.rs             │
│   └── models/*.rs                   │
└──────────────┬──────────────────────┘
               │ (完全对齐)
               ▼
┌─────────────────────────────────────┐
│   TypeScript Frontend               │
│   src/core/domain/types/            │
│   ├── orderEvent.ts (镜像)          │
│   └── api/models.ts (镜像)          │
└─────────────────────────────────────┘
```

### 1.2 零适配层（Zero Adaptation Layer）

**删除所有适配器**：
- ✅ 已删除: `orderAdapter.ts`
- ✅ 已删除: `src/core/domain/events/` (旧事件系统)
- ✅ 已删除: `src/core/domain/types/events/` (遗留类型)
- ✅ 已删除: `src/core/domain/types/pricing/` (废弃类型)

**前端类型定义策略**：
```typescript
// ❌ 错误: 自定义类型
export interface CartItem {
  productId: string;  // camelCase
  // ...
}

// ✅ 正确: 类型别名
export type CartItem = import('./orderEvent').CartItemSnapshot;
```

---

## 二、需要统一的类型清单

### 2.1 订单系统类型（核心优先级）

| 前端类型 | 后端类型 | 状态 | 问题 |
|---------|---------|------|------|
| `CartItem` | `CartItemSnapshot` | ✅ 已别名 | 后端缺字段：`product_id`, `external_id` |
| `PaymentRecord` | `PaymentRecord` | ✅ 已别名 | 字段名不同：`id` vs `payment_id` |
| `HeldOrder` | `OrderSnapshot` | ✅ 已别名 | 添加 `timeline?: OrderEvent[]` |
| `ItemAttributeSelection` | `ItemOption` | ❌ 待删除 | 字段不兼容，需扩展后端 |
| `TimelineEvent` | `OrderEvent` | ✅ 已删除 | 改用后端类型 |

### 2.2 API 模型类型（已基本对齐）

✅ 已对齐：Product, Category, Attribute, Tag, Zone, Table, Employee, Role, PrintDestination

⚠️ 需验证细节：
- 字段类型匹配（`i64` vs `number`）
- 可选字段处理（`Option<T>` vs `T | null`）
- 数组默认值

---

## 三、实施方案

### 阶段一：扩展后端类型（1-2 天）

**目标**：让后端类型包含前端需要的所有字段

#### 1.1 扩展 CartItemSnapshot

**文件**: `shared/src/order/snapshot.rs`

```rust
pub struct CartItemSnapshot {
    pub id: String,              // 已有
    pub instance_id: String,     // 已有

    // ✅ 新增字段
    pub product_id: String,      // id 的别名（兼容前端）
    pub external_id: Option<String>,  // 外部系统 ID

    pub name: String,
    pub price: i64,
    // ...
}
```

**实现**：
- 在序列化时添加 `product_id = id` 映射
- `external_id` 从 Product 表关联查询

#### 1.2 扩展 ItemOption

**文件**: `shared/src/order/types.rs`

```rust
pub struct ItemOption {
    pub attribute_id: String,
    pub attribute_name: String,    // 已有
    pub option_idx: i32,
    pub option_name: String,       // 已有
    pub price_modifier: Option<i64>,

    // ✅ 新增字段（前端需要）
    pub attribute_receipt_name: Option<String>,
    pub kitchen_printer: Option<String>,
    pub receipt_name: Option<String>,
}
```

**数据来源**：从 Attribute 表关联查询

#### 1.3 统一 PaymentRecord.id

**方案 A**（推荐）：后端添加 `id` 别名

```rust
#[derive(Serialize)]
pub struct PaymentRecord {
    pub payment_id: String,

    #[serde(rename = "id")]  // 序列化时同时输出 id
    pub id_alias: String,
}
```

**方案 B**：前端全部改用 `payment_id`
- 工作量更大
- 但更符合"服务端权威"原则

**选择**：方案 A（减少前端改动量）

---

### 阶段二：前端类型清理（1 天）

**目标**：删除所有自定义类型，使用后端类型别名

#### 2.1 删除自定义类型

**文件**: `src/core/domain/types/index.ts`

```typescript
// ❌ 删除这些定义
export interface CartItem { ... }
export interface PaymentRecord { ... }
export interface ItemAttributeSelection { ... }
export interface TimelineEvent { ... }

// ✅ 改成别名
export type CartItem = import('./orderEvent').CartItemSnapshot;
export type PaymentRecord = import('./orderEvent').PaymentRecord;
export type ItemOption = import('./orderEvent').ItemOption;

// ❌ 删除 ItemAttributeSelection（改用 ItemOption）

export type HeldOrder = import('./orderEvent').OrderSnapshot & {
  key?: string;  // 别名
  id?: string;   // 别名
  timeline?: import('./orderEvent').OrderEvent[];  // 服务端类型
};
```

#### 2.2 删除遗留模块

✅ 已删除：
- `src/core/domain/events/` (旧事件系统)
- `src/core/domain/types/events/`
- `src/core/domain/types/pricing/`
- `src/core/services/order/eventReducer.ts`

---

### 阶段三：修复编译错误（2-3 天）

**当前状态**: 91 个 TypeScript 错误

#### 3.1 修复模块引用（8 个 TS2307 错误）

**错误类型**：找不到已删除的模块

**修复方案**：
```typescript
// ❌ 旧的 import
import { OrderEvent, OrderEventType } from '@/core/domain/events';
import { reduceOrderEvents } from '@/core/services/order/eventReducer';

// ✅ 新的 import
import type { OrderEvent, OrderEventType } from '@/core/domain/types/orderEvent';
// reduceOrderEvents 删除（前端不需要事件溯源了）
```

**影响文件**：
- `src/core/stores/order/useOrderEventStore.ts`
- `src/core/services/order/eventPersistence.ts`
- `src/hooks/useHistoryOrderDetail.ts`
- `src/presentation/components/OrderSidebar.tsx`
- `src/presentation/components/modals/QuickAddModal.tsx`

#### 3.2 修复属性访问（41 个 TS2339 错误）

**错误类型**：属性不存在（`external_id`, `product_id` 等）

**根本原因**：CartItemSnapshot 缺少这些字段

**临时方案**（等待后端扩展）：
```typescript
// 使用可选链
item.external_id?.slice(-5)
// 改成
item.id.slice(-5)

// 或者添加类型断言
(item as CartItem & { external_id?: string }).external_id
```

**最终方案**：后端添加这些字段后，删除临时代码

#### 3.3 修复类型不匹配（15 个 TS2345/TS2322 错误）

**主要问题**：
- `ItemAttributeSelection[]` vs `ItemOption[]`
- `TimelineEvent[]` vs `OrderEvent[]`

**修复策略**：
- 全部改用后端类型
- 删除 `ItemAttributeSelection` 的所有引用
- `timeline` 改用 `OrderEvent[]`

---

### 阶段四：Timeline 组件改造（✅ 已完成）

**新架构**：

```
┌──────────────────────────────────────────────┐
│  数据层: OrderEvent[] (服务端权威类型)        │
└───────────────┬──────────────────────────────┘
                │
                ▼
┌──────────────────────────────────────────────┐
│  渲染策略层: renderers.ts                     │
│  - 每个 Payload 有独立 Renderer              │
│  - EventRenderer<T> interface (类似 trait)  │
│  - EVENT_RENDERERS 注册表（无 switch）       │
└───────────────┬──────────────────────────────┘
                │
                ▼
┌──────────────────────────────────────────────┐
│  UI 层: TimelineList + TimelineItem          │
│  - 纯展示组件，不知道业务逻辑                │
└──────────────────────────────────────────────┘
```

**示例 Renderer**：

```typescript
const ItemsAddedRenderer: EventRenderer<ItemsAddedPayload> = {
  render(event, payload, t) {
    return {
      title: t('timeline.addItems'),
      summary: `${payload.items.length} items`,
      details: payload.items.map(item => `${item.name} x${item.quantity}`),
      icon: ShoppingBag,
      colorClass: 'bg-orange-500',
      timestamp: event.timestamp,
    };
  }
};
```

**优势**：
- ✅ 职责分离：数据/策略/UI 三层独立
- ✅ 无 switch case：通过注册表映射
- ✅ 易扩展：新增事件只需添加 Renderer
- ✅ 类型安全：从强类型 Payload 提取数据
- ✅ 服务端权威：存储原始 OrderEvent，UI 层按需格式化

---

## 四、实施步骤

### 第一步：后端扩展（优先级最高）

**责任人**: 后端开发

**任务列表**：
1. [ ] CartItemSnapshot 添加 `product_id`, `external_id`
2. [ ] ItemOption 添加显示字段（`attribute_receipt_name` 等）
3. [ ] PaymentRecord 添加 `id` 别名（或决定方案 B）
4. [ ] 更新 TypeScript 类型定义（`orderEvent.ts`）
5. [ ] 测试序列化输出

**预计工期**: 1-2 天

---

### 第二步：前端清理

**责任人**: 前端开发

**已完成**：
1. [x] 删除旧事件系统 (fd59e83)
2. [x] Timeline 组件改造 - Renderer 架构 (fd59e83)

**进行中** (详见 `2026-01-21-frontend-cleanup-tasks.md`):
3. [ ] Task 1: 修复 ItemChanges 类型定义
4. [ ] Task 2: 临时修复 external_id/product_id 缺失
5. [ ] Task 3: 修复 OrderEventType 运行时使用问题
6. [ ] Task 4: 分析并重构 useOrderEventStore (关键决策点)
7. [ ] Task 5: 删除 ItemAttributeSelection，改用 ItemOption
8. [ ] Task 6: 修复 event.type → event.event_type
9. [ ] Task 7: 最终验证和清理

**预计剩余工期**: 1-2 天

---

### 第三步：联调测试

**责任人**: 全栈

**任务列表**：
1. [ ] 后端字段验证（确认所有字段都有数据）
2. [ ] Timeline 显示验证
3. [ ] 订单流程端到端测试
4. [ ] 删除所有临时代码
5. [ ] 性能测试（确保无性能退化）

**预计工期**: 1 天

---

## 五、验收标准

### 5.1 代码质量

- [ ] `npx tsc --noEmit` 无错误
- [ ] 零类型适配器（无 `toXXX`, `fromXXX` 函数）
- [ ] 零字段名转换（全部 snake_case）
- [ ] 删除所有 `@ts-ignore` 注释

### 5.2 类型覆盖

- [ ] 所有 API 响应类型使用后端定义
- [ ] 所有 Event Payload 使用后端定义
- [ ] 所有 Model 类型使用后端定义

### 5.3 功能完整

- [ ] Timeline 正常显示
- [ ] 订单创建/修改/支付/完成流程正常
- [ ] 历史订单查看正常
- [ ] 打印功能正常（如使用 item 字段）

---

## 六、风险与缓解

### 6.1 后端扩展延期

**风险**: 后端字段扩展需要时间，影响前端进度

**缓解**：
- 前端使用类型断言临时绕过
- 后端优先实现 `product_id` 别名（最简单）
- 并行开发，前端先做不依赖新字段的部分

### 6.2 大量编译错误

**风险**: 91 个错误可能揭示更多问题

**缓解**：
- 分模块修复（orders -> timeline -> components）
- 使用 `// @ts-expect-error` 标记临时问题
- 每修复一个模块就提交，避免大爆炸合并

### 6.3 运行时类型不匹配

**风险**: 编译通过但运行时数据格式不对

**缓解**：
- 添加运行时类型验证（Zod schemas）
- 端到端测试覆盖主要流程
- 灰度发布，先内部测试

---

## 七、长期维护

### 7.1 类型同步机制

**工具**: 考虑使用 `typeshare` 或 `ts-rs` 自动生成 TypeScript 类型

**流程**：
```
Rust types (修改)
    ↓
自动生成工具
    ↓
TypeScript types (自动更新)
    ↓
CI 检查类型一致性
```

### 7.2 编码规范

**强制规则**：
- ✅ 所有新类型必须使用后端定义
- ✅ 禁止创建自定义转换函数
- ✅ 禁止使用 camelCase 字段名
- ✅ PR 必须通过 `tsc --noEmit`

---

## 八、总结

**核心原则**: 服务端权威，零适配层

**关键成果**：
- 统一类型系统（Rust ↔ TypeScript）
- Timeline Renderer 架构（类似 Rust trait）
- 删除所有遗留代码

**总工期**: 4-6 天

**风险等级**: 中（需要前后端紧密配合）

**预期收益**：
- ✅ 类型安全：消除运行时类型错误
- ✅ 开发效率：无需维护转换层
- ✅ 代码质量：职责清晰，易于维护
- ✅ 性能提升：减少数据转换开销
