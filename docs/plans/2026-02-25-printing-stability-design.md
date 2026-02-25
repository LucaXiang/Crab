# Printing Architecture Refactor

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 彻底整理打印模块架构，消除幽灵配置、修复配置失同步、增加错误反馈。

**Architecture:** 两条独立打印路径保持不变 — 本地终端打印(收据/日报/本地标签)走 localStorage + Tauri，服务端分发打印(厨房小票/奶茶标签)走 edge-server print_destination + EventRouter。重构聚焦在服务端打印的配置体系和前端 UI 清理。

**Tech Stack:** Rust (edge-server), React/TypeScript (red_coral), SQLite

---

## 问题根源

1. **幽灵配置**: `usePrinterStore.kitchenPrinter` (localStorage `printer_kitchen`) 在 UI 作为"厨房打印启用开关"，但实际厨房打印从未使用它
2. **系统级开关脆弱**: `is_kitchen_print_enabled()` 只检查 `print_defaults.kitchen_destination.is_some()`。如果 default 被清了但还有其他 active destination，整个厨房打印被短路
3. **`resolve_print_enabled` 三方不一致**: Product 用 i32 三态(-1/0/1)，Category 用 bool 二态(true/false)，System 用 Option<String>。Product 继承(-1) + Category 默认(false) = 不打印，即使系统有站点
4. **删除站点无反馈**: `KitchenPrinterList.remove()` 没有 `.catch()`，`PrintDestinationInUse` 错误被静默吞掉
5. **打印失败静默**: 厨房打印失败只写 tracing log，前端无感知

---

### Task 1: 删除幽灵配置 `kitchenPrinter`

**Files:**
- Modify: `red_coral/src/core/stores/printer/usePrinterStore.ts`
- Modify: `red_coral/src/core/stores/printer/index.ts`
- Modify: `red_coral/src/core/stores/ui/index.ts`

**Step 1: 从 usePrinterStore 删除 kitchenPrinter**

从 `usePrinterStore.ts` 删除:
- `kitchenPrinter` state 字段和初始值 `getItem('printer_kitchen')`
- `setKitchenPrinter` action
- `useKitchenPrinter` selector export
- `usePrinterActions` 中的 `setKitchenPrinter`

**Step 2: 更新 re-export 文件**

`red_coral/src/core/stores/printer/index.ts` — 删除 `useKitchenPrinter` export

`red_coral/src/core/stores/ui/index.ts` — 删除 `useKitchenPrinter` re-export

**Step 3: TypeScript 类型检查**

Run: `cd red_coral && npx tsc --noEmit`
Expected: HardwareSettings.tsx 编译错误（引用了被删除的 kitchenPrinter）

**Step 4: Commit**

```
feat(printer): remove ghost kitchenPrinter from localStorage store
```

---

### Task 2: 重构 HardwareSettings — 厨房打印区域

**Files:**
- Modify: `red_coral/src/screens/Settings/components/printer/HardwareSettings.tsx`

**Step 1: 移除厨房打印机的幽灵开关和 PrinterSelect**

当前 HardwareSettings 的厨房区域有:
- 一个 toggle 开关（由 `!!kitchenPrinter` 控制）
- 一个 `PrinterSelect`（选择"默认全局厨房打印机" → localStorage，从未被使用）
- `KitchenPrinterList`（真正的 edge-server 打印站点）

重构为:
- **删除** toggle 开关和 `PrinterSelect`
- **直接显示** `KitchenPrinterList`（始终可见，不需要 toggle）
- 如果没有站点，`KitchenPrinterList` 自身已有空状态 UI（"点击添加"）

具体删除:
- `import { useKitchenPrinter }` 和相关 hooks
- `isKitchenPrintEnabled` 计算
- `kitchenPrinter` 变量
- 包裹 KitchenPrinterList 的条件渲染 `{isKitchenPrintEnabled ? (...) : (disabled placeholder)}`
- "默认全局打印机" 的 `PrinterSelect` 组件

保留:
- 厨房打印标题栏（ChefHat icon + title）
- Info Banner（配置层级说明）
- `KitchenPrinterList` 组件

**Step 2: TypeScript 类型检查**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

**Step 3: Commit**

```
refactor(printer): remove ghost toggle from kitchen printing settings
```

---

### Task 3: 重构系统级开关 — 用实际 destination 存在性替代 print_config 单例

**Files:**
- Modify: `edge-server/src/services/catalog_service.rs`
- Modify: `edge-server/src/printing/service.rs`

**Step 1: 修改 `is_kitchen_print_enabled` / `is_label_print_enabled`**

当前（脆弱）:
```rust
pub fn is_kitchen_print_enabled(&self) -> bool {
    let defaults = self.print_defaults.read();
    defaults.kitchen_destination.is_some()
}
```

修改为 — 检查是否有任何 active kitchen destination（不依赖 print_config 单例）:
```rust
pub fn is_kitchen_print_enabled(&self) -> bool {
    // 有默认站点 OR 有任何 active kitchen destination
    let defaults = self.print_defaults.read();
    if defaults.kitchen_destination.is_some() {
        return true;
    }
    // Fallback: check if any category has kitchen destinations configured
    let categories = self.categories.read();
    categories.values().any(|c| !c.kitchen_print_destinations.is_empty())
}
```

同理修改 `is_label_print_enabled`。

**Step 2: 修改 `resolve_print_enabled` — 增加系统级 fallback**

当前:
```rust
fn resolve_print_enabled(product_flag: i32, category_flag: Option<bool>, ) -> bool {
    match product_flag {
        1 => true,
        0 => false,
        _ => category_flag.unwrap_or(false),
    }
}
```

不改这个函数。问题不在这里 — category 是 `bool`，总是 `Some(true)` 或 `Some(false)`，`unwrap_or` 分支走不到。真正的问题是 category 默认值 `false` + 系统级短路。

**Step 3: Commit**

```
fix(printer): use destination existence check instead of print_config singleton
```

---

### Task 4: 修复 KitchenPrinterList 删除反馈

**Files:**
- Modify: `red_coral/src/screens/Settings/components/printer/KitchenPrinterList.tsx`

**Step 1: 添加确认对话框和错误处理**

当前:
```tsx
onClick={() => remove(dest.id)}
```

修改为:
```tsx
onClick={async () => {
  // 简单确认
  if (!confirm(t('settings.printer.kitchen_station.confirm_delete', { name: dest.name }))) return;
  try {
    await remove(dest.id);
    toast.success(t('settings.printer.kitchen_station.deleted'));
  } catch (error) {
    const message = getErrorMessage(error);
    toast.error(message);
  }
}}
```

添加 import: `toast` from Toast, `getErrorMessage` from error utils。

**Step 2: 添加 i18n key**

在 `zh-CN.json`, `es-ES.json`, `en.json` 中添加:
- `settings.printer.kitchen_station.confirm_delete`: "确定要删除站点「{name}」吗？"
- `settings.printer.kitchen_station.deleted`: "打印站点已删除"

**Step 3: TypeScript 类型检查**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

**Step 4: Commit**

```
fix(printer): add delete confirmation and error feedback for kitchen stations
```

---

### Task 5: 收据打印错误反馈审计

**Files:**
- Modify: `red_coral/src/screens/Checkout/payment/PaymentSuccessModal.tsx` (if needed)
- Audit: `red_coral/src/screens/History/index.tsx`
- Audit: `red_coral/src/core/services/order/paymentService.ts`

**Step 1: 审计所有 printReceipt 调用链**

1. `PaymentSuccessModal.onPrint()` → 调用 `printOrderReceipt().catch(() => toast.error(...))`
   - 在 `usePaymentActions.ts:42`: `.catch(() => toast.error(...))` ✅ 已有

2. `HistoryScreen.handleReprint()` → `reprintArchivedReceipt().catch(toast.error)`
   - 在 `History/index.tsx:58`: 在 try/catch 中 ✅ 已有

3. `handlePrintPrePayment()` → `printPrePaymentReceipt()`
   - 在 `usePaymentActions.ts:161`: 在 try/catch 中 ✅ 已有

**Step 2: 确认 `receiptPrinter` 为 null 时的 UX**

- `buildPrintHandler`: `if (!receiptPrinter) return undefined` → SuccessModal 不显示打印按钮。这个逻辑正确，但用户可能不知道为什么没有打印按钮。
- 在 SuccessModal 中，当 `onPrint` 为 undefined 时，不显示打印按钮也不提示。可以考虑在没有配置打印机时显示一个轻提示。

**Step 3: 可选 — 添加未配置打印机提示**

在 `PaymentSuccessModal` 中，当 `!onPrint` 且收据打印机未配置时，底部加一行小字提示 "未配置收据打印机"。但这需要额外 prop，可以作为后续优化。

**Step 4: Commit (如有改动)**

```
fix(printer): audit receipt printing error feedback paths
```

---

### Task 6: Clippy + TSC 全量验证

**Step 1: Rust 检查**

Run: `cargo clippy --workspace`
Expected: 零警告零错误（除已知 dead_code warnings）

**Step 2: TypeScript 检查**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

**Step 3: 测试**

Run: `cargo test --workspace --lib`
Expected: PASS

**Step 4: Final Commit**

```
chore(printer): printing architecture cleanup complete
```

---

## Files Summary

| File | Change |
|------|--------|
| `red_coral/src/core/stores/printer/usePrinterStore.ts` | 删除 kitchenPrinter 相关 |
| `red_coral/src/core/stores/printer/index.ts` | 删除 useKitchenPrinter export |
| `red_coral/src/core/stores/ui/index.ts` | 删除 useKitchenPrinter re-export |
| `red_coral/src/screens/Settings/components/printer/HardwareSettings.tsx` | 删除幽灵 toggle + PrinterSelect，直接展示 KitchenPrinterList |
| `red_coral/src/screens/Settings/components/printer/KitchenPrinterList.tsx` | 添加删除确认 + 错误处理 |
| `edge-server/src/services/catalog_service.rs` | 修改系统级 enabled 检查逻辑 |
| i18n files (zh-CN, es-ES, en) | 添加删除确认/成功 key |
