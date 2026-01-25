# 前端订单架构完善计划

**日期**: 2026-01-22
**状态**: 待执行

---

## 背景

完成 CommandHandler 后端迁移后，前端需要相应调整：
- 已实现 `order_sync` 推送 (event + snapshot)
- 前端 `_applyOrderSync()` 直接替换快照，无需本地计算
- 存在冗余代码和架构不一致问题

---

## 目标

1. 删除冗余代码，简化架构
2. 统一订单操作入口
3. 提升代码可维护性

---

## 任务清单

### Phase 1: 删除冗余代码 (P1)

#### Task 1.1: 删除 useOrderStore.ts
**文件**: `src/core/stores/order/useOrderStore.ts`
**原因**: 仅作为导出聚合器，不管理状态，增加复杂度
**影响**: POS 屏幕的 `useHeldOrders()`
**操作**:
- 删除文件
- 修改导入为直接使用 `useActiveOrdersStore`

#### Task 1.2: 合并 useOrderCommands + useOrderOperations
**文件**:
- `src/core/stores/order/useOrderCommands.ts`
- `src/core/stores/order/useOrderOperations.ts`

**原因**: 两个文件实现几乎相同的命令发送逻辑
**操作**:
- 保留 `useOrderCommands.ts` 作为 React Hook
- 将 `useOrderOperations.ts` 的纯函数逻辑合并
- 删除 `useOrderOperations.ts`

#### Task 1.3: 清理 orderReducer 未使用代码
**文件**: `src/core/stores/order/orderReducer.ts`
**删除**:
- `_detectLocalDrift()` - 从未使用
- `_getDriftedOrders()` - 从未使用
- `computeChecksum()` - Client Mode 不再需要（服务端权威）

**保留**:
- `applyEvent()` - Server Mode 本地事件仍需要
- `createEmptySnapshot()` - 初始化需要

---

### Phase 2: 优化架构 (P2)

#### Task 2.1: 移除 useCheckoutStore 策略模式
**文件**: `src/core/stores/order/useCheckoutStore.ts`
**删除**: `_computeStrategy` 相关代码（过度设计，从未使用）

#### Task 2.2: 审计 Split Bill 字段
**检查**: `useCheckoutStore` 中的 split bill 字段是否已迁移到后端
- `splitGuestCount`
- `payingShares`
- `splitSelections`

**如果已迁移**: 删除前端冗余字段

#### Task 2.3: 统一错误处理
**当前问题**:
- `useOrderCommands`: 返回 CommandResponse
- `useOrderOperations`: 抛异常
- `useOrderSync`: console.error + setState

**目标**: 统一使用 Result 类型或一致的错误处理模式

---

### Phase 3: 性能优化 (P3)

#### Task 3.1: 优化 useActiveOrdersStore
**当前**: `orders: Map<string, OrderSnapshot>`
**问题**: 每次 `Array.from(orders.values())` 创建新数组
**方案**:
- A: 改用 `Record<string, OrderSnapshot>`
- B: 添加缓存层避免重复计算

#### Task 3.2: 实现指数退避重连
**文件**: `src/core/stores/order/useOrderSync.ts`
**当前**: 固定 3 秒延迟，线性重试
**目标**: 指数退避 (1s → 1.5s → 2.25s ... 上限 30s)

---

## 依赖关系

```
Phase 1 (无依赖，可并行)
  ├── Task 1.1 (独立)
  ├── Task 1.2 (独立)
  └── Task 1.3 (独立)

Phase 2 (依赖 Phase 1 完成)
  ├── Task 2.1 (独立)
  ├── Task 2.2 (独立)
  └── Task 2.3 (依赖 1.2)

Phase 3 (依赖 Phase 2 完成)
  ├── Task 3.1 (独立)
  └── Task 3.2 (独立)
```

---

## 验收标准

- [ ] `npx tsc --noEmit` 无新增错误
- [ ] POS 页面订单操作正常
- [ ] Server Mode 本地事件正常
- [ ] Client Mode 多端同步正常
- [ ] 删除文件后无死代码引用

---

## 风险

1. **useOrderStore 删除影响范围**: 需要全局搜索确认所有引用
2. **orderReducer 修改**: Server Mode 仍依赖 `applyEvent()`，需保留
3. **Split Bill**: 需确认后端是否完全接管

---

## 文件清单

| 操作 | 文件 |
|------|------|
| 删除 | `src/core/stores/order/useOrderStore.ts` |
| 删除 | `src/core/stores/order/useOrderOperations.ts` |
| 修改 | `src/core/stores/order/orderReducer.ts` |
| 修改 | `src/core/stores/order/useOrderCommands.ts` |
| 修改 | `src/core/stores/order/useCheckoutStore.ts` |
| 修改 | `src/core/stores/order/useActiveOrdersStore.ts` |
| 修改 | `src/core/stores/order/useOrderSync.ts` |
