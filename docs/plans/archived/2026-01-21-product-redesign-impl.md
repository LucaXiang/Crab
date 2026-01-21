# Product 重设计实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 ProductSpecification 从独立表改为嵌入式设计，简化前后端代码。

**Architecture:** 删除 `product_specification` 表，specs 嵌入到 Product 的 `specs: Vec<EmbeddedSpec>` 字段。前端删除 useSpecStore，ProductForm 内联编辑 specs。

**Tech Stack:** Rust (edge-server, shared), TypeScript (React, Zustand), SurrealDB

---

## Phase 1: 后端清理

### Task 1: 更新 shared/src/models/product.rs

**Files:**
- Modify: `shared/src/models/product.rs`

**Step 1: 读取当前文件，理解结构**

**Step 2: 删除 ProductSpecification 相关类型，简化 Product**

替换整个文件内容：

```rust
//! Product Model

use serde::{Deserialize, Serialize};

/// 嵌入式规格
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedSpec {
    pub name: String,
    #[serde(default)]
    pub price: i64,
    #[serde(default)]
    pub display_order: i32,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default = "default_true")]
    pub is_active: bool,
    pub external_id: Option<i64>,
}

fn default_true() -> bool {
    true
}

/// Product entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: Option<String>,
    pub name: String,
    pub image: String,
    /// Category reference (String ID, required)
    pub category: String,
    pub sort_order: i32,
    /// Tax rate in percentage (e.g., 10 = 10%)
    pub tax_rate: i32,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    /// Kitchen printer reference (override category setting)
    pub kitchen_printer: Option<String>,
    /// -1=inherit, 0=disabled, 1=enabled
    pub is_kitchen_print_enabled: i32,
    pub is_label_print_enabled: i32,
    pub is_active: bool,
    /// Tag references (String IDs)
    #[serde(default)]
    pub tags: Vec<String>,
    /// 嵌入式规格 (至少 1 个)
    #[serde(default)]
    pub specs: Vec<EmbeddedSpec>,
}

/// Create product payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCreate {
    pub name: String,
    pub image: Option<String>,
    pub category: String,
    pub sort_order: Option<i32>,
    pub tax_rate: Option<i32>,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub kitchen_printer: Option<String>,
    pub is_kitchen_print_enabled: Option<i32>,
    pub is_label_print_enabled: Option<i32>,
    pub tags: Option<Vec<String>>,
    /// 规格列表 (至少 1 个)
    pub specs: Vec<EmbeddedSpec>,
}

/// Update product payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductUpdate {
    pub name: Option<String>,
    pub image: Option<String>,
    pub category: Option<String>,
    pub sort_order: Option<i32>,
    pub tax_rate: Option<i32>,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub kitchen_printer: Option<String>,
    pub is_kitchen_print_enabled: Option<i32>,
    pub is_label_print_enabled: Option<i32>,
    pub is_active: Option<bool>,
    pub tags: Option<Vec<String>>,
    /// 规格列表 (更新时可选)
    pub specs: Option<Vec<EmbeddedSpec>>,
}

/// Product attribute binding with full attribute data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductAttributeBinding {
    /// Relation ID (has_attribute edge)
    pub id: Option<String>,
    /// Full attribute object
    pub attribute: super::attribute::Attribute,
    pub is_required: bool,
    pub display_order: i32,
    pub default_option_idx: Option<i32>,
}

/// Full product with all related data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductFull {
    pub id: Option<String>,
    pub name: String,
    pub image: String,
    pub category: String,
    pub sort_order: i32,
    pub tax_rate: i32,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub kitchen_printer: Option<String>,
    pub is_kitchen_print_enabled: i32,
    pub is_label_print_enabled: i32,
    pub is_active: bool,
    /// 嵌入式规格
    pub specs: Vec<EmbeddedSpec>,
    /// Attribute bindings with full attribute data
    pub attributes: Vec<ProductAttributeBinding>,
    /// Tags attached to this product
    pub tags: Vec<super::tag::Tag>,
}
```

**Step 3: 验证编译**

Run: `cargo check -p shared`
Expected: 其他 crate 会报错，这是预期的，下一步修复

**Step 4: Commit**

```bash
git add shared/src/models/product.rs
git commit -m "refactor(shared): simplify Product model, embed specs"
```

---

### Task 2: 更新 edge-server db models

**Files:**
- Modify: `edge-server/src/db/models/product.rs`
- Delete: `edge-server/src/db/models/product_specification.rs`
- Modify: `edge-server/src/db/models/mod.rs`

**Step 1: 更新 product.rs，删除 has_multi_spec**

在 `edge-server/src/db/models/product.rs` 中：

1. 删除 `has_multi_spec` 字段 (Product struct 约第 40 行)
2. 删除 Product::new 中的 `has_multi_spec: false,` (约第 77 行)
3. 删除 ProductCreate 中的 `has_multi_spec` 字段 (约第 99 行)
4. 删除 ProductUpdate 中的 `has_multi_spec` 字段 (约第 123 行)
5. 添加 `specs: Vec<EmbeddedSpec>` 到 ProductCreate
6. 添加 `specs: Option<Vec<EmbeddedSpec>>` 到 ProductUpdate

**Step 2: 删除 product_specification.rs**

```bash
rm edge-server/src/db/models/product_specification.rs
```

**Step 3: 更新 mod.rs**

在 `edge-server/src/db/models/mod.rs` 中：
- 删除 `pub mod product_specification;` (第 16 行)
- 删除 `pub use product_specification::{...};` (第 39 行)

**Step 4: 验证编译**

Run: `cargo check -p edge-server`
Expected: repository 和 handler 会报错，下一步修复

**Step 5: Commit**

```bash
git add -A
git commit -m "refactor(edge-server): remove ProductSpecification model"
```

---

### Task 3: 更新 edge-server repository

**Files:**
- Modify: `edge-server/src/db/repository/product.rs`
- Modify: `edge-server/src/db/repository/mod.rs`

**Step 1: 简化 product.rs**

删除整个 `ProductSpecificationRepository` 实现 (约第 215-375 行)。

更新 `ProductRepository::create` 方法，添加 specs 校验和处理：

```rust
/// Create a new product
pub async fn create(&self, data: ProductCreate) -> RepoResult<Product> {
    // 校验 specs 非空
    if data.specs.is_empty() {
        return Err(RepoError::Validation("specs cannot be empty".into()));
    }
    // 校验最多一个 default
    let default_count = data.specs.iter().filter(|s| s.is_default).count();
    if default_count > 1 {
        return Err(RepoError::Validation("only one default spec allowed".into()));
    }

    let product = Product {
        id: None,
        name: data.name,
        image: data.image.unwrap_or_default(),
        category: data.category,
        sort_order: data.sort_order.unwrap_or(0),
        tax_rate: data.tax_rate.unwrap_or(0),
        receipt_name: data.receipt_name,
        kitchen_print_name: data.kitchen_print_name,
        kitchen_printer: data.kitchen_printer,
        is_kitchen_print_enabled: data.is_kitchen_print_enabled.unwrap_or(-1),
        is_label_print_enabled: data.is_label_print_enabled.unwrap_or(-1),
        is_active: true,
        tags: data.tags.unwrap_or_default(),
        specs: data.specs,
    };

    let created: Option<Product> = self.base.db().create(PRODUCT_TABLE).content(product).await?;
    created.ok_or_else(|| RepoError::Database("Failed to create product".to_string()))
}
```

删除 `find_with_specs` 方法 (specs 已嵌入)。

删除 `delete` 中删除 specifications 的逻辑。

**Step 2: 更新 mod.rs**

在 `edge-server/src/db/repository/mod.rs` 中：
- 修改第 34 行: `pub use product::ProductRepository;` (移除 ProductSpecificationRepository)

**Step 3: 验证编译**

Run: `cargo check -p edge-server`
Expected: handler 会报错，下一步修复

**Step 4: Commit**

```bash
git add -A
git commit -m "refactor(edge-server): remove ProductSpecificationRepository"
```

---

### Task 4: 更新 edge-server API handler

**Files:**
- Modify: `edge-server/src/api/products/handler.rs`
- Modify: `edge-server/src/api/products/mod.rs`
- Modify: `edge-server/src/api/convert.rs`

**Step 1: 清理 handler.rs**

1. 删除 import 中的 `ProductSpecificationCreate, ProductSpecificationUpdate` (第 10 行)
2. 删除 import 中的 `ProductSpecificationRepository` (第 11 行)
3. 删除 import 中的 `ProductSpecification, SpecificationData` (第 15 行)
4. 删除 `RESOURCE_SPEC` 常量 (第 18 行)
5. 删除 `get_specs` handler (第 146-154 行)
6. 删除整个 `ProductSpecification Handlers` section (第 235-338 行)
7. 更新 `get_full` 中 specs 的处理，直接使用 `product.specs`

**Step 2: 清理 mod.rs**

删除 `spec_routes()` 函数和相关路由：
- 删除 `.nest("/api/specs", spec_routes())` (第 15 行)
- 删除 `/{id}/specs` 路由 (第 23 行)
- 删除 `fn spec_routes()` 整个函数 (第 29-34 行)

**Step 3: 清理 convert.rs**

删除 `ProductSpecification` 的 From 实现 (约第 93-110 行)

**Step 4: 验证编译**

Run: `cargo check -p edge-server`
Expected: PASS

**Step 5: Commit**

```bash
git add -A
git commit -m "refactor(edge-server): remove ProductSpecification API"
```

---

### Task 5: 更新数据库 Schema

**Files:**
- Modify: `edge-server/migrations/schemas/product.surql`
- Delete: `edge-server/migrations/schemas/product_specification.surql`

**Step 1: 更新 product.surql**

删除 `has_multi_spec` 字段定义 (约第 30-33 行)

**Step 2: 删除 product_specification.surql**

```bash
rm edge-server/migrations/schemas/product_specification.surql
```

**Step 3: Commit**

```bash
git add -A
git commit -m "refactor(db): remove product_specification table"
```

---

### Task 6: 验证后端编译

**Step 1: Full workspace check**

Run: `cargo check --workspace`
Expected: 可能有 tauri 相关错误，下一阶段修复

**Step 2: 运行测试**

Run: `cargo test -p edge-server --lib`
Expected: PASS (或跳过需要 DB 的测试)

---

## Phase 2: 前端清理

### Task 7: 更新 TypeScript 类型

**Files:**
- Modify: `red_coral/src/core/domain/types/api/models.ts`

**Step 1: 更新 models.ts**

1. 找到 `Product` interface，删除 `has_multi_spec` 字段
2. 添加 `specs: EmbeddedSpec[]` 到 `Product`
3. 同样更新 `ProductCreate` 和 `ProductUpdate`
4. 删除 `ProductSpecification` interface
5. 删除 `ProductSpecificationCreate` interface
6. 删除 `ProductSpecificationUpdate` interface
7. 删除 `SpecificationData` interface (用 EmbeddedSpec 代替)
8. 更新 `ProductFull` 中的 specs 类型为 `EmbeddedSpec[]`

确保 `EmbeddedSpec` interface 存在：

```typescript
export interface EmbeddedSpec {
  name: string;
  price: number;
  display_order: number;
  is_default: boolean;
  is_active: boolean;
  external_id?: number;
}
```

**Step 2: 验证类型**

Run: `cd red_coral && npx tsc --noEmit`
Expected: 会有错误，下一步修复

**Step 3: Commit**

```bash
git add -A
git commit -m "refactor(frontend): simplify Product types"
```

---

### Task 8: 删除 useSpecStore

**Files:**
- Delete: `red_coral/src/core/stores/resources/useSpecStore.ts`
- Modify: `red_coral/src/core/stores/resources/index.ts`
- Modify: `red_coral/src/core/stores/resources/registry.ts`

**Step 1: 删除 useSpecStore.ts**

```bash
rm red_coral/src/core/stores/resources/useSpecStore.ts
```

**Step 2: 更新 index.ts**

删除 useSpecStore 相关的导出

**Step 3: 更新 registry.ts**

删除 `product_specification: useSpecStore` 注册

**Step 4: Commit**

```bash
git add -A
git commit -m "refactor(frontend): remove useSpecStore"
```

---

### Task 9: 删除 Specification 组件

**Files:**
- Delete: `red_coral/src/screens/Settings/components/SpecificationManager.tsx`
- Delete: `red_coral/src/screens/Settings/components/SpecificationManagementModal.tsx`

**Step 1: 删除文件**

```bash
rm red_coral/src/screens/Settings/components/SpecificationManager.tsx
rm red_coral/src/screens/Settings/components/SpecificationManagementModal.tsx
```

**Step 2: 搜索并清理引用**

Run: `grep -r "SpecificationManager\|SpecificationManagementModal" red_coral/src/`

根据结果更新引用这些组件的文件。

**Step 3: Commit**

```bash
git add -A
git commit -m "refactor(frontend): remove Specification components"
```

---

### Task 10: 更新 tauri-client API

**Files:**
- Modify: `red_coral/src/infrastructure/api/tauri-client.ts`

**Step 1: 删除 spec 相关 API 方法**

删除：
- `listAllSpecs`
- `getSpec`
- `createSpec`
- `updateSpec`
- `deleteSpec`
- 其他 spec 相关方法

**Step 2: Commit**

```bash
git add -A
git commit -m "refactor(frontend): remove spec API methods"
```

---

### Task 11: 更新 Tauri commands

**Files:**
- Modify: `red_coral/src-tauri/src/commands/data.rs`

**Step 1: 删除 spec 相关 commands**

搜索并删除所有 `spec` 相关的 command 函数。

**Step 2: 验证编译**

Run: `cd red_coral && cargo check -p red_coral`

**Step 3: Commit**

```bash
git add -A
git commit -m "refactor(tauri): remove spec commands"
```

---

### Task 12: 添加 Product 辅助函数

**Files:**
- Create: `red_coral/src/utils/product.ts`

**Step 1: 创建辅助函数文件**

```typescript
import type { Product, ProductFull, EmbeddedSpec, ProductAttributeBinding } from '@/core/domain/types/api';

/** 判断是否单规格产品 */
export const isSingleSpec = (product: Product | ProductFull): boolean =>
  product.specs.filter((s) => s.is_active).length === 1;

/** 获取默认规格 */
export const getDefaultSpec = (product: Product | ProductFull): EmbeddedSpec | undefined =>
  product.specs.find((s) => s.is_default && s.is_active) ??
  product.specs.find((s) => s.is_active);

/** 获取所有激活规格 */
export const getActiveSpecs = (product: Product | ProductFull): EmbeddedSpec[] =>
  product.specs.filter((s) => s.is_active);

/** 判断是否可快速添加 */
export const canQuickAdd = (product: ProductFull): boolean => {
  const hasDefaultSpec =
    product.specs.some((s) => s.is_default && s.is_active) ||
    product.specs.filter((s) => s.is_active).length === 1;

  const requiredAttrs = product.attributes.filter((a) => a.is_required);
  const allAttrsHaveDefault = requiredAttrs.every((a) => a.default_option_idx != null);

  return hasDefaultSpec && allAttrsHaveDefault;
};

/** 获取默认属性选项 */
export const getDefaultAttributeOptions = (
  attributes: ProductAttributeBinding[]
): Array<{ attribute_id: string; option_idx: number }> =>
  attributes
    .filter((a) => a.is_required && a.default_option_idx != null)
    .map((a) => ({
      attribute_id: a.attribute.id!,
      option_idx: a.default_option_idx!,
    }));
```

**Step 2: Commit**

```bash
git add -A
git commit -m "feat(frontend): add product utility functions"
```

---

### Task 13: 修复剩余编译错误

**Step 1: 运行类型检查**

Run: `cd red_coral && npx tsc --noEmit 2>&1 | head -100`

**Step 2: 逐个修复错误**

根据错误信息修复：
- 替换 `has_multi_spec` 引用为 `specs.length > 1`
- 替换 `ProductSpecification` 类型为 `EmbeddedSpec`
- 更新组件中的 spec 使用方式

**Step 3: 验证**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

**Step 4: Commit**

```bash
git add -A
git commit -m "fix(frontend): resolve type errors after spec refactor"
```

---

## Phase 3: 验证

### Task 14: 全量编译验证

**Step 1: 后端**

Run: `cargo build --workspace`
Expected: PASS

**Step 2: 前端**

Run: `cd red_coral && npm run build`
Expected: PASS

**Step 3: Commit (如有修复)**

```bash
git add -A
git commit -m "fix: final cleanup after product redesign"
```

---

## 完成检查清单

- [ ] shared/src/models/product.rs 简化完成
- [ ] ProductSpecification 表删除
- [ ] ProductSpecificationRepository 删除
- [ ] /api/specs 路由删除
- [ ] useSpecStore 删除
- [ ] SpecificationManager 组件删除
- [ ] 前端类型更新完成
- [ ] 辅助函数添加完成
- [ ] 全量编译通过
