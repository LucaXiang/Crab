# 资源模型迁移计划

> 日期: 2026-01-22
> 状态: 待审批

## 概述

全链路更新资源数据结构，涉及 6 个模型的修改。由于处于开发阶段，采用**清空重建**策略，不做数据兼容。

## 修改清单

| # | 结构 | 修改内容 |
|---|------|---------|
| 1 | `EmbeddedSpec` | +`receipt_name: Option<String>`, +`is_root: bool` |
| 2 | `Tag` | +`is_system: bool` |
| 3 | `Attribute` | 删除 `scope`, `excluded_categories` |
| 4 | `HasAttribute` | 改名 `AttributeBinding`, +`default_option_idx: Option<i32>` |
| 5 | `ProductAttributeBinding` | 改名 `AttributeBindingFull` |
| 6 | `EmployeeResponse` | 改名 `Employee`, +`display_name: String`, +`is_system: bool`, `id: String` → `id: Option<String>` |

## 迁移顺序

```
┌─────────────────┐
│  1. Database    │  SurrealDB Schema
└────────┬────────┘
         ▼
┌─────────────────┐
│  2. Shared      │  shared/src/models/*.rs
└────────┬────────┘
         ▼
┌─────────────────┐
│  3. Edge Server │  edge-server/src/db/models/*.rs
│                 │  edge-server/src/db/repository/*.rs
│                 │  edge-server/src/api/**/*.rs
└────────┬────────┘
         ▼
┌─────────────────┐
│  4. Tauri Bridge│  red_coral/src-tauri/src/**/*.rs
└────────┬────────┘
         ▼
┌─────────────────┐
│  5. Frontend    │  red_coral/src/core/domain/types/**/*.ts
│                 │  red_coral/src/core/stores/**/*.ts
│                 │  red_coral/src/screens/**/*.tsx
└─────────────────┘
```

---

## Phase 1: Database Schema

### 文件
- `edge-server/migrations/schemas/attribute.surql`
- `edge-server/migrations/schemas/tag.surql` (如存在)
- `edge-server/migrations/schemas/product.surql` (如存在)
- `edge-server/migrations/schemas/employee.surql` (如存在)

### 操作
清空数据库，重新定义 schema。

---

## Phase 2: Shared Crate (API DTOs)

### 文件清单

| 文件 | 修改 |
|-----|------|
| `shared/src/models/product.rs` | `EmbeddedSpec` +2 字段 |
| `shared/src/models/tag.rs` | `Tag` +1 字段 |
| `shared/src/models/attribute.rs` | `Attribute` -2 字段, `HasAttribute` 改名+字段, `ProductAttributeBinding` 改名 |
| `shared/src/models/employee.rs` | `EmployeeResponse` 改名+字段 |

### EmbeddedSpec 修改

```rust
// Before
pub struct EmbeddedSpec {
    pub name: String,
    pub price: f64,
    pub display_order: i32,
    pub is_default: bool,
    pub is_active: bool,
    pub external_id: Option<i64>,
}

// After
pub struct EmbeddedSpec {
    pub name: String,
    pub price: f64,
    pub display_order: i32,
    pub is_default: bool,
    pub is_active: bool,
    pub external_id: Option<i64>,
    pub receipt_name: Option<String>,  // NEW: 小票显示名称
    pub is_root: bool,                 // NEW: 根规格，不可删除
}
```

### Tag 修改

```rust
// Before
pub struct Tag {
    pub id: Option<String>,
    pub name: String,
    pub color: String,
    pub display_order: i32,
    pub is_active: bool,
}

// After
pub struct Tag {
    pub id: Option<String>,
    pub name: String,
    pub color: String,
    pub display_order: i32,
    pub is_active: bool,
    pub is_system: bool,  // NEW: 系统标签，不可删除
}
```

### Attribute 修改

```rust
// Before
pub struct Attribute {
    pub id: Option<String>,
    pub name: String,
    pub scope: String,                      // REMOVE
    pub excluded_categories: Vec<String>,   // REMOVE
    pub is_multi_select: bool,
    pub max_selections: Option<i32>,
    pub default_option_idx: Option<i32>,
    pub display_order: i32,
    pub is_active: bool,
    pub show_on_receipt: bool,
    pub receipt_name: Option<String>,
    pub show_on_kitchen_print: bool,
    pub kitchen_print_name: Option<String>,
    pub options: Vec<AttributeOption>,
}

// After
pub struct Attribute {
    pub id: Option<String>,
    pub name: String,
    // scope 和 excluded_categories 已删除
    pub is_multi_select: bool,
    pub max_selections: Option<i32>,
    pub default_option_idx: Option<i32>,
    pub display_order: i32,
    pub is_active: bool,
    pub show_on_receipt: bool,
    pub receipt_name: Option<String>,
    pub show_on_kitchen_print: bool,
    pub kitchen_print_name: Option<String>,
    pub options: Vec<AttributeOption>,
}
```

### HasAttribute → AttributeBinding

```rust
// Before: HasAttribute
pub struct HasAttribute {
    pub id: Option<String>,
    pub from: String,
    pub to: String,
    pub is_required: bool,
    pub display_order: i32,
}

// After: AttributeBinding
pub struct AttributeBinding {
    pub id: Option<String>,
    pub from: String,
    pub to: String,
    pub is_required: bool,
    pub display_order: i32,
    pub default_option_idx: Option<i32>,  // NEW: 覆盖默认选项
}
```

### ProductAttributeBinding → AttributeBindingFull

```rust
// Before: ProductAttributeBinding
pub struct ProductAttributeBinding {
    pub id: Option<String>,
    pub attribute: Attribute,
    pub is_required: bool,
    pub display_order: i32,
}

// After: AttributeBindingFull
pub struct AttributeBindingFull {
    pub id: Option<String>,
    pub attribute: Attribute,
    pub is_required: bool,
    pub display_order: i32,
    pub default_option_idx: Option<i32>,  // NEW: 覆盖默认选项
}
```

### EmployeeResponse → Employee

```rust
// Before: EmployeeResponse
pub struct EmployeeResponse {
    pub id: String,
    pub username: String,
    pub role: String,
    pub is_active: bool,
}

// After: Employee
pub struct Employee {
    pub id: Option<String>,       // CHANGED: String → Option<String>
    pub username: String,
    pub display_name: String,     // NEW: 显示名称
    pub role: String,
    pub is_system: bool,          // NEW: 系统创建
    pub is_active: bool,
}
```

---

## Phase 3: Edge Server

### 文件清单

| 文件 | 修改 |
|-----|------|
| `edge-server/src/db/models/product.rs` | 同 shared |
| `edge-server/src/db/models/tag.rs` | 同 shared |
| `edge-server/src/db/models/attribute.rs` | 同 shared |
| `edge-server/src/db/models/employee.rs` | 同 shared |
| `edge-server/src/db/models/mod.rs` | 更新导出名称 |
| `edge-server/src/db/repository/attribute.rs` | 更新类型引用 |
| `edge-server/src/db/repository/employee.rs` | 更新类型引用 |
| `edge-server/src/api/has_attribute/handler.rs` | 更新类型引用 |
| `edge-server/src/api/has_attribute/mod.rs` | 更新类型引用 |
| `edge-server/src/api/products/handler.rs` | 更新类型引用 |
| `edge-server/src/api/employees/handler.rs` | 更新类型引用 |
| `edge-server/src/api/convert.rs` | 更新转换逻辑 |
| `edge-server/src/pricing/item_calculator.rs` | 移除 scope 相关逻辑 |

---

## Phase 4: Tauri Bridge

### 文件清单

| 文件 | 修改 |
|-----|------|
| `red_coral/src-tauri/src/core/response.rs` | 更新类型定义 |
| `red_coral/src-tauri/src/commands/data.rs` | 更新类型引用 |
| `red_coral/src-tauri/src/commands/system.rs` | 更新类型引用 |

---

## Phase 5: Frontend TypeScript

### 文件清单

| 文件 | 修改 |
|-----|------|
| `red_coral/src/core/domain/types/api/models.ts` | 更新接口定义 |
| `red_coral/src/core/domain/types/api/index.ts` | 更新导出 |
| `red_coral/src/core/stores/resources/useAttributeStore.ts` | 移除 scope 相关逻辑 |
| `red_coral/src/core/stores/resources/useEmployeeStore.ts` | 更新类型 |
| `red_coral/src/core/stores/auth/useAuthStore.ts` | 更新类型 |
| `red_coral/src/infrastructure/api/tauri-client.ts` | 更新类型 |
| `red_coral/src/screens/Settings/forms/ProductForm.tsx` | 更新表单字段 |
| `red_coral/src/screens/Settings/forms/TagForm.tsx` | 添加 is_system 字段 |
| `red_coral/src/screens/Settings/TagManagement.tsx` | 系统标签不可删除 |
| `red_coral/src/presentation/components/modals/*.tsx` | 更新类型引用 |

### TypeScript 类型修改

```typescript
// EmbeddedSpec
interface EmbeddedSpec {
  name: string;
  price: number;
  display_order: number;
  is_default: boolean;
  is_active: boolean;
  external_id?: number;
  receipt_name?: string;  // NEW
  is_root: boolean;       // NEW
}

// Tag
interface Tag {
  id?: string;
  name: string;
  color: string;
  display_order: number;
  is_active: boolean;
  is_system: boolean;     // NEW
}

// Attribute (移除 scope, excluded_categories)
interface Attribute {
  id?: string;
  name: string;
  // scope 和 excluded_categories 已删除
  is_multi_select: boolean;
  max_selections?: number;
  default_option_idx?: number;
  display_order: number;
  is_active: boolean;
  show_on_receipt: boolean;
  receipt_name?: string;
  show_on_kitchen_print: boolean;
  kitchen_print_name?: string;
  options: AttributeOption[];
}

// AttributeBinding (原 HasAttribute)
interface AttributeBinding {
  id?: string;
  from: string;
  to: string;
  is_required: boolean;
  display_order: number;
  default_option_idx?: number;  // NEW
}

// AttributeBindingFull (原 ProductAttributeBinding)
interface AttributeBindingFull {
  id?: string;
  attribute: Attribute;
  is_required: boolean;
  display_order: number;
  default_option_idx?: number;  // NEW
}

// Employee (原 EmployeeResponse)
interface Employee {
  id?: string;
  username: string;
  display_name: string;   // NEW
  role: string;
  is_system: boolean;     // NEW
  is_active: boolean;
}
```

---

## 验证清单

- [ ] `cargo check --workspace` 无错误
- [ ] `cargo test --workspace --lib` 通过
- [ ] `cargo clippy --workspace` 无警告
- [ ] `npm run typecheck` (前端) 无错误
- [ ] `npm run build` (前端) 成功
- [ ] 手动测试：创建商品、添加属性、员工管理

---

## 风险

1. **数据丢失**: 清空重建会丢失所有测试数据 → 已确认接受
2. **编译错误**: 类型重命名可能导致大量编译错误 → 按顺序修改，逐步编译验证
3. **前后端不同步**: 类型定义不一致 → 严格按计划同步修改

---

## 预估工作量

| Phase | 文件数 | 预估 |
|-------|-------|------|
| Database | 2-4 | 简单 |
| Shared | 4 | 中等 |
| Edge Server | 12 | 较多 |
| Tauri Bridge | 3 | 简单 |
| Frontend | 10+ | 较多 |
| **总计** | ~30 | - |
