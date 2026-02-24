# 票据打印字段补全 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 让已定义的打印控制字段（receipt_name, kitchen_print_name, show_on_receipt, show_on_kitchen_print）在整个数据链路中生效 — 从订单快照到收据/厨房单/标签打印。

**Architecture:** 扩展 shared ItemOption 携带打印字段 → 前端构建时从目录填充 → 收据/厨房/标签渲染器检查并使用这些字段。不改数据库 schema（字段已存在），不需要向后兼容（开发阶段，数据库会清理）。

**Tech Stack:** Rust (shared, edge-server), TypeScript (red_coral React), ESC/POS (crab-printer)

**设计文档:** `docs/plans/2026-02-24-receipt-print-fields-design.md`

---

### Task 1: 扩展 Rust ItemOption 类型

**Files:**
- Modify: `shared/src/order/types.rs:169-186`

**Step 1: 添加打印字段到 ItemOption**

在 `shared/src/order/types.rs:186` 的 `quantity` 字段后添加 4 个新字段：

```rust
/// Item option selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemOption {
    pub attribute_id: i64,
    pub attribute_name: String,
    pub option_id: i64,
    pub option_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_modifier: Option<f64>,
    /// Option quantity (default: 1)
    #[serde(
        default = "default_option_quantity",
        skip_serializing_if = "is_default_quantity"
    )]
    pub quantity: i32,
    /// Receipt display name (falls back to option_name if None)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_name: Option<String>,
    /// Kitchen ticket display name (falls back to option_name if None)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kitchen_print_name: Option<String>,
    /// Whether to show this option on customer receipt
    pub show_on_receipt: bool,
    /// Whether to show this option on kitchen ticket
    pub show_on_kitchen_print: bool,
}
```

**Step 2: 验证编译**

Run: `cargo check -p shared`
Expected: FAIL — 其他 crate 中构建 ItemOption 的代码缺少新字段

**Step 3: 修复所有 ItemOption 构建点**

搜索整个 workspace 中构建 `ItemOption { ... }` 的位置并添加新字段（默认值 `receipt_name: None, kitchen_print_name: None, show_on_receipt: true, show_on_kitchen_print: true`）。

关键位置:
- `edge-server/src/orders/actions/add_items.rs` — 如果有构建 ItemOption
- `edge-server/src/orders/actions/modify_item.rs` — 同上
- `edge-server/src/orders/manager/` 中的测试代码

**Step 4: 验证编译通过**

Run: `cargo check --workspace`
Expected: PASS

**Step 5: Commit**

```bash
git add shared/src/order/types.rs
# + 所有修复的文件
git commit -m "feat(shared): add print control fields to ItemOption"
```

---

### Task 2: 扩展 TypeScript ItemOption 类型

**Files:**
- Modify: `red_coral/src/core/domain/types/orderEvent.ts:1122-1130`

**Step 1: 添加打印字段到 TS ItemOption**

```typescript
export interface ItemOption {
  attribute_id: number;
  attribute_name: string;
  option_id: number;
  option_name: string;
  price_modifier?: number | null;
  /** Option quantity (default: 1) */
  quantity?: number;
  /** Receipt display name (falls back to option_name if null) */
  receipt_name?: string | null;
  /** Kitchen ticket display name (falls back to option_name if null) */
  kitchen_print_name?: string | null;
  /** Whether to show this option on customer receipt */
  show_on_receipt: boolean;
  /** Whether to show this option on kitchen ticket */
  show_on_kitchen_print: boolean;
}
```

**Step 2: 验证 TS 类型检查**

Run: `cd red_coral && npx tsc --noEmit`
Expected: FAIL — 构建 ItemOption 的位置缺少新字段

---

### Task 3: 前端选项构建 — ProductOptionsModal

**Files:**
- Modify: `red_coral/src/presentation/components/modals/ProductOptionsModal.tsx:152-159`

**Step 1: 在构建 ItemOption 时填充打印字段**

当前代码 (行 152-159):
```typescript
result.push({
  attribute_id: attr.id,
  attribute_name: attr.name,
  option_id: optionId,
  option_name: option.name,
  price_modifier: option.price_modifier ?? null,
  quantity: qty,
});
```

改为:
```typescript
result.push({
  attribute_id: attr.id,
  attribute_name: attr.name,
  option_id: optionId,
  option_name: option.name,
  price_modifier: option.price_modifier ?? null,
  quantity: qty,
  receipt_name: option.receipt_name ?? null,
  kitchen_print_name: option.kitchen_print_name ?? null,
  show_on_receipt: attr.show_on_receipt,
  show_on_kitchen_print: attr.show_on_kitchen_print,
});
```

注意: `receipt_name` / `kitchen_print_name` 来自 `option`（AttributeOption），`show_on_receipt` / `show_on_kitchen_print` 来自 `attr`（Attribute）。

**Step 2: 验证 TS 类型检查**

Run: `cd red_coral && npx tsc --noEmit`
Expected: 可能还有其他构建 ItemOption 的位置报错

---

### Task 4: 前端 — toCartItemInput 和 receiptBuilder

**Files:**
- Modify: `red_coral/src/core/stores/order/commands/items.ts:18-25`
- Modify: `red_coral/src/core/services/order/receiptBuilder.ts:75-81`
- Modify: `red_coral/src/infrastructure/print/printService.ts:40-45`

**Step 1: 更新 toCartItemInput 传递新字段**

当前 (items.ts 行 18-25):
```typescript
const selectedOptions = item.selected_options?.map(opt => ({
  attribute_id: opt.attribute_id,
  attribute_name: opt.attribute_name,
  option_id: opt.option_id,
  option_name: opt.option_name,
  price_modifier: opt.price_modifier ?? null,
  quantity: opt.quantity ?? 1,
})) ?? null;
```

改为:
```typescript
const selectedOptions = item.selected_options?.map(opt => ({
  attribute_id: opt.attribute_id,
  attribute_name: opt.attribute_name,
  option_id: opt.option_id,
  option_name: opt.option_name,
  price_modifier: opt.price_modifier ?? null,
  quantity: opt.quantity ?? 1,
  receipt_name: opt.receipt_name ?? null,
  kitchen_print_name: opt.kitchen_print_name ?? null,
  show_on_receipt: opt.show_on_receipt,
  show_on_kitchen_print: opt.show_on_kitchen_print,
})) ?? null;
```

**Step 2: 更新 ReceiptSelectedOption 类型添加 show_on_receipt**

在 `printService.ts` 的 `ReceiptSelectedOption` 接口 (行 40) 添加:
```typescript
export interface ReceiptSelectedOption {
  attribute_name: string;
  option_name: string;
  receipt_name: string | null;
  price_modifier: number;
  show_on_receipt: boolean;   // 新增
}
```

同步更新 Rust 端 `SelectedOption` (在 `red_coral/src-tauri/src/api/printers.rs:31-36`):
```rust
pub struct SelectedOption {
    pub attribute_name: String,
    pub option_name: String,
    pub receipt_name: Option<String>,
    pub price_modifier: f64,
    pub show_on_receipt: bool,   // 新增
}
```

**Step 3: 更新 receiptBuilder 使用真实值**

当前 (receiptBuilder.ts 行 75-81):
```typescript
selected_options: item.selected_options
  ? item.selected_options.map((opt) => ({
      attribute_name: opt.attribute_name,
      option_name: opt.option_name,
      receipt_name: null,           // ← 硬编码 null！
      price_modifier: opt.price_modifier ?? 0,
    }))
  : null,
```

改为:
```typescript
selected_options: item.selected_options
  ? item.selected_options
      .filter((opt) => opt.show_on_receipt)    // 过滤不显示的选项
      .map((opt) => ({
        attribute_name: opt.attribute_name,
        option_name: opt.option_name,
        receipt_name: opt.receipt_name ?? null,  // 使用真实值
        price_modifier: opt.price_modifier ?? 0,
        show_on_receipt: opt.show_on_receipt,
      }))
  : null,
```

注意: `spec_name` 行 (83) 也要改为优先使用 receipt_name:
```typescript
spec_name: item.selected_specification?.receipt_name
  || item.selected_specification?.name
  || null,
```

**Step 4: 更新 buildArchivedReceiptData 同样的改动**

同文件 (receiptBuilder.ts) 的 `buildArchivedReceiptData` 函数中也有类似的硬编码 `receipt_name: null`，需要同样修复。

**Step 5: 验证 TS 类型检查**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

**Step 6: Commit**

```bash
git add red_coral/src/core/domain/types/orderEvent.ts \
        red_coral/src/presentation/components/modals/ProductOptionsModal.tsx \
        red_coral/src/core/stores/order/commands/items.ts \
        red_coral/src/core/services/order/receiptBuilder.ts \
        red_coral/src/infrastructure/print/printService.ts \
        red_coral/src-tauri/src/api/printers.rs
git commit -m "feat(red_coral): wire print control fields through order → receipt pipeline"
```

---

### Task 5: 收据渲染器 — 尊重 show_on_receipt

**Files:**
- Modify: `red_coral/src-tauri/src/utils/receipt_renderer.rs:162-196`

**Step 1: 在渲染选项时过滤 show_on_receipt**

当前收据渲染器 (行 163-196) 无条件遍历所有 options。

在 `receipt_renderer.rs` 行 166 的循环中添加过滤:
```rust
for option in options {
    if !option.show_on_receipt {
        continue;  // 跳过不需要在收据上显示的选项
    }
    let attr_name = &option.attribute_name;
    let display_name = option
        .receipt_name
        .as_deref()
        .unwrap_or(&option.option_name)
        .to_string();
    // ... 其余逻辑不变
}
```

注意: `SelectedOption` 已经在 Task 4 中添加了 `show_on_receipt` 字段。

**Step 2: 规格名使用 receipt_name**

在行 157-161 的 spec_name 显示中，优先使用 receipt_name:
```rust
// 当前:
if let Some(ref spec_name) = item.spec_name {

// 不需要改动 — spec_name 在 receiptBuilder 中已经使用了 receipt_name 优先逻辑
```

**Step 3: 验证编译**

Run: `cargo check -p red_coral`（或 `cargo check --workspace`）
Expected: PASS

**Step 4: Commit**

```bash
git add red_coral/src-tauri/src/utils/receipt_renderer.rs
git commit -m "feat(receipt): respect show_on_receipt flag and use receipt_name"
```

---

### Task 6: 厨房打印 — 尊重 show_on_kitchen_print 和 kitchen_print_name

**Files:**
- Modify: `edge-server/src/printing/service.rs:232-247`

**Step 1: 在 build_print_context 中过滤并使用厨房打印名**

当前代码 (service.rs 行 232-247):
```rust
let options: Vec<String> = item
    .selected_options
    .as_ref()
    .map(|opts| {
        opts.iter()
            .map(|opt| {
                if opt.quantity > 1 {
                    format!("{}×{}", opt.option_name, opt.quantity)
                } else {
                    opt.option_name.clone()
                }
            })
            .collect()
    })
    .unwrap_or_default();
```

改为:
```rust
let options: Vec<String> = item
    .selected_options
    .as_ref()
    .map(|opts| {
        opts.iter()
            .filter(|opt| opt.show_on_kitchen_print)
            .map(|opt| {
                let name = opt
                    .kitchen_print_name
                    .as_deref()
                    .unwrap_or(&opt.option_name);
                if opt.quantity > 1 {
                    format!("{}×{}", name, opt.quantity)
                } else {
                    name.to_string()
                }
            })
            .collect()
    })
    .unwrap_or_default();
```

**Step 2: 验证编译**

Run: `cargo check -p edge-server`
Expected: PASS

**Step 3: Commit**

```bash
git add edge-server/src/printing/service.rs
git commit -m "feat(kitchen): respect show_on_kitchen_print and use kitchen_print_name"
```

---

### Task 7: 标签打印 — 使用 receipt_name

**Files:**
- Modify: `edge-server/src/printing/service.rs` (label 部分，如有)
- Modify: `edge-server/src/printing/types.rs` (如需要)

**Step 1: 确认标签打印的数据构建**

标签打印使用同一个 `PrintItemContext`。`options` 字段已在 Task 6 中改为使用 `kitchen_print_name`。

标签应该使用 `receipt_name` 而不是 `kitchen_print_name`。需要检查标签数据流是否与厨房单共享 options 字段。

如果标签和厨房单共用 `PrintItemContext.options: Vec<String>`，则需要分成两种：
- 方案 A: 改 `PrintItemContext` 为两个 options 列表（kitchen_options + label_options）
- 方案 B: 标签打印不通过 `options` 字段，而是直接使用 `CartItemSnapshot` 的 `selected_options`

检查实际代码后决定。如果标签只打印商品名+规格+价格（不含选项），则无需改动。

**Step 2: 验证编译**

Run: `cargo check --workspace`
Expected: PASS

**Step 3: Commit（如有改动）**

```bash
git add edge-server/src/printing/
git commit -m "feat(label): use receipt_name for label printing"
```

---

### Task 8: 全栈验证

**Step 1: Rust 编译 + Clippy**

Run: `cargo clippy --workspace`
Expected: PASS (零警告)

**Step 2: TypeScript 类型检查**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

**Step 3: Rust 测试**

Run: `cargo test --workspace --lib`
Expected: PASS (可能需要修复测试中构建 ItemOption 的地方)

**Step 4: 最终 Commit**

如果有额外修复:
```bash
git add -A
git commit -m "fix: address clippy warnings and test fixes for print fields"
```
