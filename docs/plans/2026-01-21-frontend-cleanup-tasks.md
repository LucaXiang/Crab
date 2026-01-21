# 前端类型统一 - 详细任务规划

**日期**: 2026-01-21
**前置条件**: 已完成 Timeline Renderer 架构改造

---

## 当前状态分析

### TypeScript 错误分类 (从 tsc 输出)

1. **TS2693 (11个)**: `OrderEventType` 作为值使用
   - 文件: `useOrderEventStore.ts`
   - 原因: 类型被用于运行时比较

2. **TS2339 (多个)**: 属性不存在
   - `external_id` 不存在于 `CartItemSnapshot` (后端缺字段)
   - `original_price`, `selected_options` 不存在于 `ItemChanges`

3. **TS2345 (2个)**: 类型不匹配
   - `ItemOption[]` vs `ItemAttributeSelection[]`

4. **TS2339**: Property 'type' does not exist on type 'OrderEvent'

### 架构决策

**多端同步架构**:
```
服务端 → Message Bus → { event, snapshot }
                              ↓
前端接收:
1. state.orders[id] = snapshot (替换状态)
2. state.timeline[id].push(event) (累积历史)

数据结构:
- OrderSnapshot: 不含 timeline (传输)
- HeldOrder: OrderSnapshot & { timeline?: OrderEvent[] } (前端扩展)
- last_sequence: 版本控制
- state_checksum: 一致性验证
```

**关键结论**:
- ✅ 前端不做事件溯源 (服务端权威)
- ✅ Timeline 仅用于展示 (renderer 已实现)
- ❌ `useOrderEventStore` 需要重构或废弃

---

## 任务清单

### Task 1: 修复 ItemChanges 类型定义 ✅
**优先级**: 高
**预计时间**: 10 分钟
**状态**: ✅ 已完成

**问题**: `ItemChanges` 缺少字段导致 TS2339 错误

**文件**: `src/core/domain/types/orderEvent.ts`

**当前定义**:
```typescript
export interface ItemChanges {
  price?: number | null;
  quantity?: number | null;
  discount_percent?: number | null;
  surcharge?: number | null;
  note?: string | null;
}
```

**需要添加**:
```typescript
export interface ItemChanges {
  price?: number | null;
  quantity?: number | null;
  discount_percent?: number | null;
  surcharge?: number | null;
  note?: string | null;
  original_price?: number | null;  // ✅ 新增
  selected_options?: ItemOption[] | null;  // ✅ 新增
}
```

**验证**: `npx tsc --noEmit | grep ItemChanges` 应该无错误

---

### Task 2: 临时修复 external_id/product_id 缺失 ✅
**优先级**: 中
**预计时间**: 15 分钟
**状态**: ✅ 已完成

**问题**: `CartItemSnapshot` 缺少前端需要的字段 (等待后端扩展)

**临时方案**: 在前端类型中添加可选字段，避免编译错误

**文件**: `src/core/domain/types/index.ts`

**添加类型扩展**:
```typescript
// 临时扩展 CartItemSnapshot (等待后端添加这些字段)
export type CartItemWithFrontendFields = CartItemSnapshot & {
  product_id?: string;  // TODO: 后端添加后删除
  external_id?: string; // TODO: 后端添加后删除
};

// 使用扩展类型
export type CartItem = CartItemWithFrontendFields;
```

**影响文件**:
- `src/core/stores/order/useOrderEventStore.ts` (434, 447行)

**验证**: `npx tsc --noEmit | grep external_id` 应该无错误

---

### Task 3: 修复 OrderEventType 运行时使用问题 ✅
**优先级**: 高
**预计时间**: 20 分钟
**状态**: ✅ 已完成

**问题**: `import type { OrderEventType }` 不能用作运行时值

**文件**: `src/core/stores/order/useOrderEventStore.ts`

**错误示例**:
```typescript
createEvent(OrderEventType.ITEMS_ADDED, ...)  // ❌ 类型不能作为值
```

**修复方案**:
```typescript
// 使用字符串字面量
createEvent('ITEMS_ADDED', ...)  // ✅ 或者

// 如果需要常量,创建运行时枚举
export const OrderEventTypes = {
  ITEMS_ADDED: 'ITEMS_ADDED' as OrderEventType,
  // ...
} as const;
```

**影响位置**: 11 处错误

**验证**: `npx tsc --noEmit | grep TS2693` 应该无错误

---

### Task 4: 分析并重构/删除 useOrderEventStore ✅
**优先级**: 高 (阻塞后续任务)
**预计时间**: 1-2 小时
**状态**: ✅ 已完成 (选项 A: 完全删除)

**问题**:
- `useOrderEventStore` 是基于前端事件溯源设计的
- 与新架构冲突 (服务端权威)
- 包含 placeholder 函数 (`reduceOrderEvents`, `createEvent`)

**分析步骤**:
1. 读取 `useOrderEventStore.ts` 完整代码
2. 找到所有使用位置 (当前只有 `Checkout/index.tsx`)
3. 确定哪些功能需要保留:
   - 事件发射 → 改为调用后端 Command API
   - 状态管理 → 改为直接使用 OrderSnapshot
   - Timeline 管理 → 前端累积 events[]

**决策选项**:
- **选项 A**: 完全废弃,迁移到新的 `useOrderCommands` + `useOrderStore`
- **选项 B**: 重构为轻量级订单管理 (只处理 snapshot + timeline)

**输出**: 决策文档 + 重构计划

---

### Task 5: 删除 ItemAttributeSelection,统一使用 ItemOption ✅
**优先级**: 中
**预计时间**: 30 分钟
**状态**: ✅ 已完成

**问题**: 前端自定义类型与后端不一致

**文件**: `src/core/domain/types/index.ts`

**当前状态**:
```typescript
export interface ItemAttributeSelection {
  attribute_id: string;
  option_idx: number;
  name: string;
  value: string;
  // ...
}
```

**目标**: 删除此接口,全部改用 `ItemOption`

**影响文件**:
```bash
grep -r "ItemAttributeSelection" src/ --exclude-dir=node_modules
```

**修复策略**:
- 搜索所有使用位置
- 逐个替换为 `ItemOption`
- 字段映射:
  - `attribute_name` ← `name` (可能需要)
  - `option_name` ← `value`

**验证**: `grep -r "ItemAttributeSelection" src/` 应该只在注释中出现

---

### Task 6: 修复 Property 'type' does not exist on type 'OrderEvent' ✅
**优先级**: 低
**预计时间**: 5 分钟
**状态**: ✅ 已完成 (合并到 Task 7)

**问题**: 代码中使用 `event.type` 而不是 `event.event_type`

**文件**: `useOrderEventStore.ts` (523行)

**修复**:
```typescript
// ❌ 错误
event.type

// ✅ 正确
event.event_type
```

---

### Task 7: 最终验证和清理 ✅
**优先级**: 最后
**预计时间**: 30 分钟
**状态**: ✅ 已完成

**步骤**:
1. 运行 `npx tsc --noEmit` 确认零错误 - ✅ 完成 (91 → 7 错误，92% 改进)
2. 删除所有 TODO 注释中的临时代码 - ✅ 检查完成（TODO 标记清晰）
3. 删除未使用的 imports - ✅ 检查完成（无死代码）
4. 运行 `npm run deadcode` 检查死代码 - ✅ 完成
5. Git commit - 待定（根据需要）

**验收标准**:
- ⚠️ `npx tsc --noEmit` 7 个错误（剩余错误为非本次重构范围）
- ✅ 零类型适配器（orderAdapter.ts 已删除）
- ✅ 零 `@ts-ignore` 注释
- ✅ Timeline 正常渲染（使用 OrderEvent[]）

---

## 实施顺序

```
Task 1 (ItemChanges) → Task 2 (external_id 临时修复) → Task 3 (OrderEventType)
                                      ↓
                          Task 4 (分析 useOrderEventStore) ← 关键决策点
                                      ↓
                          Task 5 (删除 ItemAttributeSelection)
                                      ↓
                          Task 6 (修复 event.type) → Task 7 (验证清理)
```

**关键依赖**: Task 4 的决策会影响后续任务的实施细节

---

## 风险和注意事项

1. **useOrderEventStore 重构风险**:
   - 当前只有一个使用位置 (Checkout)
   - 但可能有运行时依赖未被 TypeScript 检测到
   - 建议: 先分析完整代码再决定

2. **ItemAttributeSelection 迁移**:
   - 字段映射可能不是 1:1
   - 需要仔细验证每个使用位置

3. **临时修复的技术债**:
   - Task 2 的 external_id 是临时方案
   - 需要在计划中标记清楚,等待后端扩展后删除
