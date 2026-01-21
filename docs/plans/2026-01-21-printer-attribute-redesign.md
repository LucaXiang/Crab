# Printer & Attribute 重构实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 重构打印机和属性系统，删除 `kitchen_printer` 表，新增 `print_destination` 表，优化属性设计

**Architecture:**
- PrintDestination 作为抽象打印目的地，支持多个物理打印机配置
- Attribute 支持 global/inherited 作用域，可排除分类
- 打印时 message_bus 通过 ID 查询 DB 获取打印配置

**Tech Stack:** Rust, SurrealDB, TypeScript, React

---

## 变更概览

### 删除
| 表/字段 | 说明 |
|---------|------|
| `kitchen_printer` 表 | 用 `print_destination` 替代 |
| `product.kitchen_printer` | 改为 `print_destinations[]` |
| `product.is_kitchen_print_enabled` | 空数组 = 不打印 |
| `category.kitchen_printer` | 改为 `print_destinations[]` |
| `category.is_kitchen_print_enabled` | 空数组 = 不打印 |
| `attribute.kitchen_printer` | 改为 `show_on_kitchen_print` + `kitchen_print_name` |
| `attribute.attr_type` | 改为 `is_multi_select` |
| `attribute.is_global` | 改为 `scope` + `excluded_categories` |
| `option.value_code` | 不再需要 |
| `option.is_default` | 统一用 `default_option_idx` |

### 新增
| 表/字段 | 说明 |
|---------|------|
| `print_destination` 表 | 含嵌入式 `printers[]` |
| `product.print_destinations[]` | 打印目的地数组 |
| `category.print_destinations[]` | 打印目的地数组 |
| `attribute.scope` | "global" \| "inherited" |
| `attribute.excluded_categories[]` | 全局属性排除的分类 |
| `attribute.is_multi_select` | bool |
| `attribute.max_selections` | 多选上限 |
| `attribute.default_option_idx` | 属性级默认选项 |
| `attribute.show_on_kitchen_print` | bool |
| `attribute.kitchen_print_name` | Option<String> |
| `option.kitchen_print_name` | Option<String> |

---

## Task 1: 新增 print_destination 表 (Schema + Models)

**Files:**
- Create: `edge-server/migrations/schemas/print_destination.surql`
- Create: `edge-server/src/db/models/print_destination.rs`
- Modify: `edge-server/src/db/models/mod.rs`
- Create: `shared/src/models/print_destination.rs`
- Modify: `shared/src/models/mod.rs`

**Step 1: 创建 SurrealDB Schema**

```surql
-- Print Destination Schema
-- 打印目的地（抽象层）

DEFINE TABLE OVERWRITE print_destination TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE name ON print_destination TYPE string
    ASSERT string::len($value) > 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE description ON print_destination TYPE option<string>
    PERMISSIONS FULL;

-- 嵌入式打印机配置
DEFINE FIELD OVERWRITE printers ON print_destination TYPE array<object>
    DEFAULT []
    PERMISSIONS FULL;

-- 打印机字段定义
DEFINE FIELD OVERWRITE printers.*.printer_type ON print_destination TYPE string PERMISSIONS FULL;
DEFINE FIELD OVERWRITE printers.*.ip ON print_destination TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE printers.*.port ON print_destination TYPE option<int> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE printers.*.driver_name ON print_destination TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE printers.*.priority ON print_destination TYPE int DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD OVERWRITE printers.*.is_active ON print_destination TYPE bool DEFAULT true PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_active ON print_destination TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE print_dest_name ON print_destination FIELDS name UNIQUE;
```

**Step 2: 创建 edge-server db model**

```rust
// edge-server/src/db/models/print_destination.rs
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedPrinter {
    pub printer_type: String,  // "network" | "driver"
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub driver_name: Option<String>,
    pub priority: i32,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintDestination {
    pub id: Option<Thing>,
    pub name: String,
    pub description: Option<String>,
    pub printers: Vec<EmbeddedPrinter>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintDestinationCreate {
    pub name: String,
    pub description: Option<String>,
    pub printers: Vec<EmbeddedPrinter>,
    #[serde(default = "super::default_true")]
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintDestinationUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub printers: Option<Vec<EmbeddedPrinter>>,
    pub is_active: Option<bool>,
}
```

**Step 3: 创建 shared model**

```rust
// shared/src/models/print_destination.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedPrinter {
    pub printer_type: String,
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub driver_name: Option<String>,
    pub priority: i32,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintDestination {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub printers: Vec<EmbeddedPrinter>,
    pub is_active: bool,
}
```

**Step 4: 更新 mod.rs 导出**

**Step 5: 验证编译**
```bash
cargo check -p shared -p edge-server
```

---

## Task 2: 创建 print_destination Repository + API

**Files:**
- Create: `edge-server/src/db/repository/print_destination.rs`
- Modify: `edge-server/src/db/repository/mod.rs`
- Create: `edge-server/src/api/print_destination/mod.rs`
- Create: `edge-server/src/api/print_destination/handler.rs`
- Modify: `edge-server/src/api/mod.rs`
- Modify: `edge-server/src/api/convert.rs`

**Step 1: 创建 Repository**

标准 CRUD: list, get, create, update, delete

**Step 2: 创建 API Handler**

路由:
- GET /api/print-destinations
- GET /api/print-destinations/:id
- POST /api/print-destinations
- PUT /api/print-destinations/:id
- DELETE /api/print-destinations/:id

**Step 3: 添加类型转换 (convert.rs)**

**Step 4: 验证编译**

---

## Task 3: 更新 Product 表

**Files:**
- Modify: `edge-server/migrations/schemas/product.surql`
- Modify: `edge-server/src/db/models/product.rs`
- Modify: `edge-server/src/db/repository/product.rs`
- Modify: `edge-server/src/api/convert.rs`
- Modify: `shared/src/models/product.rs`

**变更:**
```diff
- kitchen_printer: Option<record<kitchen_printer>>
- is_kitchen_print_enabled: i32
+ print_destinations: array<record<print_destination>>
```

**Step 1: 更新 Schema**

**Step 2: 更新 db models**

**Step 3: 更新 repository**

**Step 4: 更新 convert.rs**

**Step 5: 更新 shared models**

**Step 6: 验证编译**

---

## Task 4: 更新 Category 表

**Files:**
- Modify: `edge-server/migrations/schemas/category.surql`
- Modify: `edge-server/src/db/models/category.rs`
- Modify: `edge-server/src/db/repository/category.rs`
- Modify: `edge-server/src/api/convert.rs`
- Modify: `shared/src/models/category.rs`

**变更:**
```diff
- kitchen_printer: Option<record<kitchen_printer>>
- is_kitchen_print_enabled: bool
+ print_destinations: array<record<print_destination>>
```

---

## Task 5: 更新 Attribute 表

**Files:**
- Modify: `edge-server/migrations/schemas/attribute.surql`
- Modify: `edge-server/src/db/models/attribute.rs`
- Modify: `edge-server/src/db/repository/attribute.rs`
- Modify: `edge-server/src/api/convert.rs`
- Modify: `shared/src/models/attribute.rs`

**变更:**
```diff
- attr_type: string  // "single_select" | "multi_select"
- kitchen_printer: Option<record<kitchen_printer>>
- is_global: bool
+ scope: string  // "global" | "inherited"
+ excluded_categories: array<record<category>>
+ is_multi_select: bool
+ max_selections: Option<int>
+ default_option_idx: Option<int>
+ show_on_kitchen_print: bool
+ kitchen_print_name: Option<string>
```

**EmbeddedOption 变更:**
```diff
- value_code: Option<string>
- is_default: bool
+ kitchen_print_name: Option<string>
```

---

## Task 6: 删除 kitchen_printer 表

**Files:**
- Delete: `edge-server/migrations/schemas/kitchen_printer.surql`
- Delete: `edge-server/src/db/models/kitchen_printer.rs`
- Delete: `edge-server/src/db/repository/kitchen_printer.rs`
- Delete: `edge-server/src/api/kitchen_printer/` (if exists)
- Modify: `edge-server/src/db/models/mod.rs`
- Modify: `edge-server/src/db/repository/mod.rs`
- Modify: `edge-server/src/api/mod.rs`
- Modify: `edge-server/src/api/convert.rs`
- Delete: `shared/src/models/kitchen_printer.rs` (if exists)

**Step 1: 删除文件**

**Step 2: 更新 mod.rs 导出**

**Step 3: 移除 API 路由**

**Step 4: 验证编译**

---

## Task 7: 验证后端编译

```bash
cargo check -p shared -p edge-server
cargo check -p red-coral
```

---

## Task 8: 更新前端类型定义

**Files:**
- Modify: `red_coral/src/core/domain/types/api/models.ts`

**变更:**

```typescript
// 新增
export interface EmbeddedPrinter {
  printer_type: 'network' | 'driver';
  ip?: string;
  port?: number;
  driver_name?: string;
  priority: number;
  is_active: boolean;
}

export interface PrintDestination {
  id?: string;
  name: string;
  description?: string;
  printers: EmbeddedPrinter[];
  is_active: boolean;
}

// 更新 Product
export interface Product {
  // ...
  print_destinations: string[];  // 替代 kitchen_printer
  // 删除 is_kitchen_print_enabled
}

// 更新 Category
export interface Category {
  // ...
  print_destinations: string[];
  // 删除 is_kitchen_print_enabled
}

// 更新 Attribute
export interface Attribute {
  // ...
  scope: 'global' | 'inherited';
  excluded_categories: string[];
  is_multi_select: boolean;
  max_selections?: number;
  default_option_idx?: number;
  show_on_kitchen_print: boolean;
  kitchen_print_name?: string;
  // 删除 attr_type, kitchen_printer, is_global
}

// 更新 EmbeddedOption (原 AttributeOption)
export interface EmbeddedOption {
  name: string;
  receipt_name?: string;
  kitchen_print_name?: string;
  price_modifier: number;
  display_order: number;
  is_active: boolean;
  // 删除 value_code, is_default
}

// 删除
// - KitchenPrinter
// - KitchenPrinterCreate
// - KitchenPrinterUpdate
```

---

## Task 9: 更新前端 Stores

**Files:**
- Delete: `red_coral/src/core/stores/resources/useKitchenPrinterStore.ts`
- Create: `red_coral/src/core/stores/resources/usePrintDestinationStore.ts`
- Modify: `red_coral/src/core/stores/resources/useAttributeStore.ts`
- Modify: `red_coral/src/core/stores/resources/useCategoryStore.ts`
- Modify: `red_coral/src/core/stores/resources/index.ts`
- Modify: `red_coral/src/core/stores/resources/registry.ts`

---

## Task 10: 更新前端组件

**Files:**
- Modify: `red_coral/src/screens/Settings/components/printer/` (全部)
- Modify: `red_coral/src/screens/Settings/AttributeManagement.tsx`
- Modify: `red_coral/src/screens/Settings/CategoryManagement.tsx`
- Modify: `red_coral/src/screens/Settings/ProductManagement.tsx`
- Modify: `red_coral/src/screens/Settings/forms/AttributeForm.tsx`
- Modify: `red_coral/src/screens/Settings/forms/CategoryForm.tsx`
- Modify: `red_coral/src/screens/Settings/forms/ProductForm.tsx`
- Modify: `red_coral/src/presentation/components/form/FormField/KitchenPrinterSelector.tsx` → rename to PrintDestinationSelector

---

## Task 11: 更新 Tauri Commands

**Files:**
- Modify: `red_coral/src-tauri/src/commands/data.rs`
- Modify: `red_coral/src-tauri/src/core/response.rs`
- Modify: `red_coral/src-tauri/src/lib.rs`

**变更:**
- 删除 kitchen_printer 相关命令
- 新增 print_destination 相关命令

---

## Task 12: 前端 TypeScript 编译验证

```bash
cd red_coral && npx tsc --noEmit
```

---

## Task 13: 全量编译验证

```bash
cargo check --workspace
cd red_coral && npm run build
```

---

## 继承逻辑参考

### 产品打印目的地解析
```
1. 产品有 print_destinations → 使用产品的
2. 产品没有 → 使用分类的 print_destinations
3. 都没有 → 不打印
```

### 产品属性解析
```
1. 产品直接绑定的 (has_attribute 边)
2. 产品分类绑定的 (has_attribute 边)
3. 全局属性 (scope=global 且分类不在 excluded_categories)

去重 + 优先级: 产品 > 分类 > 全局
```

### 属性默认值解析
```
1. 产品有 has_attribute 边 → 用边的 default_option_idx
2. 分类有 has_attribute 边 → 用边的 default_option_idx
3. 都没有 → 用属性本身的 default_option_idx
```
