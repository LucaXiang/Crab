# Product 重新设计方案

> 日期: 2026-01-21
> 状态: 已确认
> 兼容性: 破坏性变更（不考虑向后兼容）

## 概述

将 `ProductSpecification` 从独立表改为嵌入式设计，简化数据模型和 API。

## 核心变化

### 删除

- `product_specification` 表
- `ProductSpecificationRepository`
- `ProductSpecification` 相关 API (`/api/product-specifications/*`)
- `has_multi_spec` 字段（用 `specs.len() > 1` 判断）
- `is_root` 字段（不再需要区分）
- 规格的 `tags` 字段（tags 只在 Product 级别）

### 保留

- `has_attribute` 图边设计（Attribute 可复用）

---

## 数据模型

### EmbeddedSpec（嵌入式规格）

```rust
// Rust (edge-server & shared)
pub struct EmbeddedSpec {
    pub name: String,           // "大杯" / "小杯"
    pub price: i64,             // 分为单位
    pub display_order: i32,     // 排序
    pub is_default: bool,       // 默认选中（快速添加用）
    pub is_active: bool,        // 是否启用
    pub external_id: Option<i64>, // 外部系统 ID
}
```

```typescript
// TypeScript (前端)
interface EmbeddedSpec {
  name: string;
  price: number;
  display_order: number;
  is_default: boolean;
  is_active: boolean;
  external_id?: number;
}
```

### Product

```rust
// Rust
pub struct Product {
    pub id: Option<ProductId>,
    pub name: String,
    pub image: String,
    pub category: Thing,
    pub sort_order: i32,
    pub tax_rate: i32,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub kitchen_printer: Option<Thing>,
    pub is_kitchen_print_enabled: i32,  // -1=继承, 0=禁用, 1=启用
    pub is_label_print_enabled: i32,
    pub is_active: bool,
    pub tags: Vec<Thing>,
    pub specs: Vec<EmbeddedSpec>,       // 嵌入式规格（至少 1 个）
}
```

```typescript
// TypeScript
interface Product {
  id: string;
  name: string;
  image: string;
  category: string;
  sort_order: number;
  tax_rate: number;
  receipt_name?: string;
  kitchen_print_name?: string;
  kitchen_printer?: string;
  is_kitchen_print_enabled: number;
  is_label_print_enabled: number;
  is_active: boolean;
  tags: string[];
  specs: EmbeddedSpec[];
}
```

### ProductFull（带展开关联）

```rust
pub struct ProductFull {
    // ...Product 所有字段...
    pub specs: Vec<EmbeddedSpec>,
    pub attributes: Vec<ProductAttributeBinding>,  // 从 has_attribute 查
    pub tags: Vec<Tag>,                            // 展开的 tag 对象
}
```

---

## 数据库 Schema

### product.surql

```surql
DEFINE TABLE product SCHEMAFULL;

DEFINE FIELD name ON product TYPE string;
DEFINE FIELD image ON product TYPE string DEFAULT "";
DEFINE FIELD category ON product TYPE record<category>;
DEFINE FIELD sort_order ON product TYPE int DEFAULT 0;
DEFINE FIELD tax_rate ON product TYPE int DEFAULT 0;
DEFINE FIELD receipt_name ON product TYPE option<string>;
DEFINE FIELD kitchen_print_name ON product TYPE option<string>;
DEFINE FIELD kitchen_printer ON product TYPE option<record<kitchen_printer>>;
DEFINE FIELD is_kitchen_print_enabled ON product TYPE int DEFAULT -1;
DEFINE FIELD is_label_print_enabled ON product TYPE int DEFAULT -1;
DEFINE FIELD is_active ON product TYPE bool DEFAULT true;
DEFINE FIELD tags ON product TYPE array<record<tag>> DEFAULT [];
DEFINE FIELD specs ON product TYPE array<object> DEFAULT [];
```

### 迁移

```surql
DROP TABLE product_specification;
```

---

## 后端校验规则

```rust
fn validate_specs(specs: &[EmbeddedSpec]) -> Result<(), RepoError> {
    // 1. specs 不能为空
    if specs.is_empty() {
        return Err(RepoError::Validation("specs cannot be empty".into()));
    }

    // 2. 最多只能有一个 is_default = true
    let default_count = specs.iter().filter(|s| s.is_default).count();
    if default_count > 1 {
        return Err(RepoError::Validation("only one default spec allowed".into()));
    }

    Ok(())
}
```

---

## API 变化

### 删除

| 端点 | 说明 |
|------|------|
| `GET /api/product-specifications` | 不再需要 |
| `POST /api/product-specifications` | 不再需要 |
| `PUT /api/product-specifications/:id` | 不再需要 |
| `DELETE /api/product-specifications/:id` | 不再需要 |

### 修改

| 端点 | 变化 |
|------|------|
| `POST /api/products` | `specs` 必填，至少 1 个 |
| `PUT /api/products/:id` | 可更新 `specs` 数组 |
| `GET /api/products/:id` | 直接返回嵌入的 `specs` |

---

## 前端变化

### 删除的组件/文件

- `SpecificationManagementModal.tsx` - 合并到 ProductForm
- `SpecificationManager.tsx` - 删除

### 修改的组件

- `ProductForm.tsx` - 添加 specs 内联编辑
- `ProductOptionsModal.tsx` - 使用 `product.specs`
- `ItemConfiguratorModal.tsx` - 使用 `product.specs`

### 辅助函数

```typescript
// utils/product.ts

/** 判断是否单规格产品 */
const isSingleSpec = (product: Product) =>
  product.specs.filter(s => s.is_active).length === 1;

/** 获取默认规格 */
const getDefaultSpec = (product: Product): EmbeddedSpec | undefined =>
  product.specs.find(s => s.is_default && s.is_active)
    ?? product.specs.find(s => s.is_active);

/** 获取所有激活规格 */
const getActiveSpecs = (product: Product) =>
  product.specs.filter(s => s.is_active);

/** 判断是否可快速添加 */
const canQuickAdd = (product: ProductFull): boolean => {
  const hasDefaultSpec = product.specs.some(s => s.is_default && s.is_active)
    || product.specs.filter(s => s.is_active).length === 1;

  const requiredAttrs = product.attributes.filter(a => a.is_required);
  const allAttrsHaveDefault = requiredAttrs.every(
    a => a.default_option_idx != null
  );

  return hasDefaultSpec && allAttrsHaveDefault;
};
```

---

## POS 交互逻辑

### 点击产品

```typescript
const handleProductClick = (product: ProductFull) => {
  const activeSpecs = product.specs.filter(s => s.is_active);
  const requiredAttrs = product.attributes.filter(a => a.is_required);
  const allAttrsHaveDefault = requiredAttrs.every(
    a => a.default_option_idx != null
  );

  if (activeSpecs.length === 1 && allAttrsHaveDefault) {
    // 单规格 + 所有必选属性有默认值：直接加购
    addToCartWithDefaults(product, activeSpecs[0], requiredAttrs);
  } else {
    // 需要选择：弹出配置框
    openItemConfigurator(product);
  }
};
```

### 快速添加（长按/双击）

```typescript
const handleQuickAdd = (product: ProductFull) => {
  const defaultSpec = getDefaultSpec(product);
  const requiredAttrs = product.attributes.filter(a => a.is_required);
  const allAttrsHaveDefault = requiredAttrs.every(
    a => a.default_option_idx != null
  );

  if (defaultSpec && allAttrsHaveDefault) {
    addToCartWithDefaults(product, defaultSpec, requiredAttrs);
  } else {
    openItemConfigurator(product);
  }
};
```

### 使用默认值加购

```typescript
const addToCartWithDefaults = (
  product: ProductFull,
  spec: EmbeddedSpec,
  requiredAttrs: ProductAttributeBinding[]
) => {
  const selectedOptions = requiredAttrs.map(attr => ({
    attribute_id: attr.attribute.id,
    option_idx: attr.default_option_idx!,
  }));

  addToCart(product, spec, selectedOptions);
};
```

---

## 实施步骤

1. **后端 - shared 模块**
   - 更新 `shared/src/models/product.rs`
   - 删除 `ProductSpecification` 相关类型

2. **后端 - edge-server**
   - 更新 `db/models/product.rs`（已部分完成）
   - 删除 `db/models/product_specification.rs`
   - 更新 `db/repository/product.rs`，删除 `ProductSpecificationRepository`
   - 更新 API handler
   - 更新数据库 migration

3. **前端 - 类型**
   - 更新 `models.ts`
   - 添加 `utils/product.ts` 辅助函数

4. **前端 - 组件**
   - 删除 `SpecificationManagementModal.tsx`
   - 删除 `SpecificationManager.tsx`
   - 更新 `ProductForm.tsx`
   - 更新 POS 相关组件

5. **清理**
   - 删除 `product_specification` 表
   - 删除无用代码
