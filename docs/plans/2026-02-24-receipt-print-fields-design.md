# 票据打印字段补全设计

**日期**: 2026-02-24
**状态**: 已批准
**范围**: 客户收据 + 厨房单 + 标签

## 问题

数据库和管理 UI 已定义了完整的打印控制字段（receipt_name, kitchen_print_name, show_on_receipt, show_on_kitchen_print），但这些字段在订单快照 → 打印渲染的链路上断裂：

| 字段 | 存储 | 传递 | 打印使用 | 状态 |
|------|------|------|---------|------|
| `attribute.show_on_receipt` | ✅ | - | ❌ 无条件打印 | 未实现 |
| `attribute.show_on_kitchen_print` | ✅ | - | ❌ 无条件打印 | 未实现 |
| `attribute_option.receipt_name` | ✅ | ❌ ItemOption 无此字段 | ❌ | 断裂 |
| `attribute_option.kitchen_print_name` | ✅ | ❌ | ❌ | 死字段 |
| `attribute.receipt_name` | ✅ | ❌ | ❌ | 死字段 |
| `attribute.kitchen_print_name` | ✅ | ❌ | ❌ | 死字段 |
| `product.is_kitchen_print_enabled` | ✅ | ✅ | ✅ | 正常 |
| `product.is_label_print_enabled` | ✅ | ✅ | ✅ | 正常 |

## 方案：最小链路修复

快照时携带打印字段，不改数据库 schema，不需要兼容性（开发阶段，老数据库会清理）。

## 数据模型变更

### ItemOption 扩展 (shared/src/order/types.rs)

```rust
pub struct ItemOption {
    pub attribute_id: i64,
    pub attribute_name: String,
    pub option_id: i64,
    pub option_name: String,
    pub price_modifier: Option<f64>,
    pub quantity: i32,
    // --- 新增 ---
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub show_on_receipt: bool,
    pub show_on_kitchen_print: bool,
}
```

### SpecificationInfo 扩展

确认是否已携带 `receipt_name`，如果没有则添加。

## 数据写入链路

### 前端 AddItems 构建 (red_coral/src/core/stores/order/commands/items.ts)

从商品目录 store 中读取 attribute/option 的打印配置，写入 AddItems 命令 payload：

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
  show_on_receipt: attribute.show_on_receipt,
  show_on_kitchen_print: attribute.show_on_kitchen_print,
}))
```

## 打印渲染链路

### 客户收据 (red_coral/src-tauri/src/utils/receipt_renderer.rs)

- 渲染选项时：检查 `show_on_receipt`，为 `false` 则跳过
- 显示名：优先 `receipt_name`，回退 `option_name`
- 规格名：优先 spec 的 `receipt_name`，回退 `spec_name`

### 厨房单 (edge-server/src/printing/service.rs + renderer.rs)

- `PrintItemContext.options` 构建时：检查 `show_on_kitchen_print`，为 `false` 则跳过
- 选项名：优先 `kitchen_print_name`，回退 `option_name`

### 标签 (edge-server/src/printing/service.rs)

- 标签数据源中使用 `receipt_name` 作为选项显示名
- 遵循与收据相同的 `show_on_receipt` 逻辑

## 字段映射表

| 来源 | 字段 | 收据 | 厨房单 | 标签 |
|------|------|------|--------|------|
| Product | `receipt_name` | 产品显示名 | - | 产品显示名 |
| Product | `kitchen_print_name` | - | 产品显示名 | - |
| ProductSpec | `receipt_name` | 规格显示名 | - | 规格显示名 |
| Attribute | `show_on_receipt` | 控制选项是否显示 | - | 同收据 |
| Attribute | `show_on_kitchen_print` | - | 控制选项是否显示 | - |
| AttrOption | `receipt_name` | 选项显示名 | - | 选项显示名 |
| AttrOption | `kitchen_print_name` | - | 选项显示名 | - |
| PriceRule | `receipt_name` | 折扣/附加费显示名 | - | - |

## 改动文件

| 文件 | 改动 |
|------|------|
| `shared/src/order/types.rs` | 扩展 ItemOption (+ SpecificationInfo) |
| `red_coral/src/core/stores/order/commands/items.ts` | 构建 AddItems 时携带打印字段 |
| `red_coral/src-tauri/src/utils/receipt_renderer.rs` | 检查 show_on_receipt，使用 receipt_name |
| `edge-server/src/printing/service.rs` | 检查 show_on_kitchen_print，使用 kitchen_print_name |
| `edge-server/src/printing/renderer.rs` | 厨房票据渲染器适配 |
| `edge-server/src/printing/types.rs` | PrintItemContext 可能需要扩展 |

**不需要改动**：数据库 schema、管理 UI、同步系统（字段已存在）。
