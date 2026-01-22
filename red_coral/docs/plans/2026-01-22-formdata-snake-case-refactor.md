# FormData Snake_Case 重构计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 useSettingsStore 的 FormData 完全重构为 snake_case，与后端 API 类型对齐，消除转换层。

**Architecture:** FormData 字段名直接使用后端 API 字段名，表单组件直接访问 snake_case 字段，提交时无需转换。

**Tech Stack:** TypeScript, Zustand, React

---

## 后端 API 类型参考

| Entity | API Fields (snake_case) |
|--------|-------------------------|
| **Tag** | `id`, `name`, `color`, `display_order`, `is_active` |
| **Category** | `id`, `name`, `sort_order`, `print_destinations[]`, `is_label_print_enabled`, `is_active`, `is_virtual`, `tag_ids[]`, `match_mode` |
| **Product** | `id`, `name`, `image`, `category`, `sort_order`, `tax_rate`, `receipt_name`, `kitchen_print_name`, `print_destinations[]`, `is_label_print_enabled`, `is_active`, `tags[]`, `specs[]` |
| **Zone** | `id`, `name`, `description`, `is_active` |
| **DiningTable** | `id`, `name`, `zone`, `capacity`, `is_active` |

## 字段映射表

| 旧字段 (camelCase) | 新字段 (snake_case) | 说明 |
|-------------------|---------------------|------|
| `zoneId` | `zone` | DiningTable.zone |
| `categoryId` | `category` | Product.category |
| `receiptName` | `receipt_name` | Product |
| `kitchenPrintName` | `kitchen_print_name` | Product |
| `kitchenPrinterId` | **删除** | 用 `print_destinations[0]` |
| `isKitchenPrintEnabled` | **删除** | 用 `print_destinations.length > 0` |
| `isLabelPrintEnabled` | `is_label_print_enabled` | Product/Category |
| `taxRate` | `tax_rate` | Product |
| `sortOrder` | `sort_order` | Product/Category |
| `displayOrder` | `display_order` | Tag |
| `isVirtual` | `is_virtual` | Category |
| `tagIds` | `tag_ids` | Category |
| `matchMode` | `match_mode` | Category |
| `surchargeType` | `surcharge_type` | Zone (UI only) |
| `surchargeAmount` | `surcharge_amount` | Zone (UI only) |
| `selectedAttributeIds` | `selected_attribute_ids` | UI only |
| `attributeDefaultOptions` | `attribute_default_options` | UI only |
| `tempSpecifications` | `specs` | Product.specs |
| `loadedSpecs` | **删除** | 合并到 specs |
| `selectedTagIds` | `tags` | Product.tags |
| `hasMultiSpec` | `has_multi_spec` | UI only |
| `externalId` | **删除** | 在 specs[].external_id 中 |

---

## Task 1: 重写 FormData 接口

**Files:**
- Modify: `src/core/stores/settings/useSettingsStore.ts:76-109`

**Step 1: 替换 FormData 接口定义**

```typescript
/**
 * FormData - 表单状态，字段名与后端 API 对齐 (snake_case)
 *
 * 设计原则：
 * - 字段名直接使用 API 字段名，无需转换
 * - UI-only 字段也使用 snake_case 保持一致
 */
interface FormData {
  // === Common ===
  id?: string;
  name: string;

  // === DiningTable ===
  zone?: string;           // Zone ID
  capacity?: number;

  // === Product ===
  category?: string;       // Category ID
  image?: string;
  sort_order?: number;
  tax_rate?: number;
  receipt_name?: string;
  kitchen_print_name?: string;
  print_destinations?: string[];  // PrintDestination IDs
  is_label_print_enabled?: boolean;
  tags?: string[];         // Tag IDs
  specs?: EmbeddedSpec[];  // 嵌入式规格
  has_multi_spec?: boolean; // UI only: 是否多规格

  // === Category ===
  is_virtual?: boolean;
  tag_ids?: string[];      // Virtual category tag filter
  match_mode?: 'any' | 'all';
  selected_attribute_ids?: string[];  // UI only: 已选属性
  attribute_default_options?: Record<string, string[]>;  // UI only

  // === Tag ===
  color?: string;
  display_order?: number;

  // === Zone (UI only, no API fields) ===
  description?: string;
  surcharge_type?: 'percentage' | 'fixed' | 'none';
  surcharge_amount?: number;
}
```

**Step 2: 更新 initialFormData**

```typescript
const initialFormData: FormData = {
  id: undefined,
  name: '',
  // DiningTable
  zone: '',
  capacity: 4,
  // Product
  category: undefined,
  image: '',
  sort_order: undefined,
  tax_rate: 10,
  receipt_name: '',
  kitchen_print_name: '',
  print_destinations: [],
  is_label_print_enabled: false,
  tags: [],
  specs: [],
  has_multi_spec: false,
  // Category
  is_virtual: false,
  tag_ids: [],
  match_mode: 'any',
  selected_attribute_ids: [],
  attribute_default_options: {},
  // Tag
  color: '#3B82F6',
  display_order: 0,
  // Zone
  description: '',
  surcharge_type: 'none',
  surcharge_amount: 0,
};
```

**Step 3: 运行类型检查**

```bash
npx tsc --noEmit 2>&1 | head -50
```

Expected: 大量类型错误（表单组件仍使用旧字段名）

**Step 4: Commit**

```bash
git add src/core/stores/settings/useSettingsStore.ts
git commit -m "refactor(settings): rewrite FormData interface to snake_case

BREAKING CHANGE: FormData fields now use snake_case to match backend API

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 2: 更新 openModal 初始化逻辑

**Files:**
- Modify: `src/core/stores/settings/useSettingsStore.ts:217-292`

**Step 1: 重写 TABLE 分支**

```typescript
if (entity === 'TABLE') {
  const tableData = data as TableEditData | null;
  formData = {
    ...formData,
    name: tableData?.name || '',
    zone: tableData?.zone || tableData?.defaultZoneId || '',
    capacity: tableData?.capacity ?? 4,
  };
}
```

**Step 2: 重写 ZONE 分支**

```typescript
} else if (entity === 'ZONE') {
  const zoneData = data as ZoneEditData | null;
  formData = {
    ...formData,
    name: zoneData?.name || '',
    description: zoneData?.description || '',
    surcharge_type: zoneData?.surchargeType || 'none',
    surcharge_amount: zoneData?.surchargeAmount || 0,
  };
}
```

**Step 3: 重写 PRODUCT 分支**

```typescript
} else if (entity === 'PRODUCT') {
  const productData = data as ProductEditData | null;
  formData = {
    ...formData,
    id: productData?.id ?? undefined,
    name: productData?.name || '',
    category: productData?.category ?? productData?.defaultCategoryId,
    image: productData?.image || '',
    sort_order: productData?.sort_order,
    tax_rate: productData?.tax_rate ?? 10,
    receipt_name: productData?.receipt_name ?? '',
    kitchen_print_name: productData?.kitchen_print_name ?? '',
    print_destinations: productData?.print_destinations || [],
    is_label_print_enabled: !!productData?.is_label_print_enabled,
    tags: productData?.tags || [],
    specs: productData?.specs || [],
    has_multi_spec: (productData?.specs?.length ?? 0) > 1,
  };
}
```

**Step 4: 重写 CATEGORY 分支**

```typescript
} else if (entity === 'CATEGORY') {
  const categoryData = data as CategoryEditData | null;
  formData = {
    ...formData,
    name: categoryData?.name || '',
    sort_order: categoryData?.sort_order,
    print_destinations: categoryData?.print_destinations || [],
    is_label_print_enabled: !!categoryData?.is_label_print_enabled,
    is_virtual: categoryData?.is_virtual ?? false,
    tag_ids: categoryData?.tag_ids ?? [],
    match_mode: categoryData?.match_mode ?? 'any',
    selected_attribute_ids: categoryData?.selectedAttributeIds || [],
    attribute_default_options: categoryData?.attributeDefaultOptions || {},
  };
}
```

**Step 5: 重写 TAG 分支**

```typescript
} else if (entity === 'TAG') {
  const tagData = data as TagEditData | null;
  formData = {
    ...formData,
    name: tagData?.name || '',
    color: tagData?.color || '#3B82F6',
    display_order: tagData?.display_order ?? 0,
  };
}
```

**Step 6: Commit**

```bash
git add src/core/stores/settings/useSettingsStore.ts
git commit -m "refactor(settings): update openModal to use snake_case fields

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 3: 更新 TableForm 组件

**Files:**
- Modify: `src/screens/Settings/forms/TableForm.tsx`

**Step 1: 替换字段访问**

| 旧 | 新 |
|---|---|
| `formData.zoneId` | `formData.zone` |
| `setFormField('zoneId', ...)` | `setFormField('zone', ...)` |

**Step 2: 运行类型检查确认**

```bash
npx tsc --noEmit 2>&1 | grep TableForm
```

Expected: 无错误

**Step 3: Commit**

```bash
git add src/screens/Settings/forms/TableForm.tsx
git commit -m "refactor(TableForm): use snake_case field names

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 4: 更新 ProductForm 组件

**Files:**
- Modify: `src/screens/Settings/forms/ProductForm.tsx`

**Step 1: 替换字段访问**

| 旧 | 新 |
|---|---|
| `formData.categoryId` | `formData.category` |
| `formData.taxRate` | `formData.tax_rate` |
| `formData.receiptName` | `formData.receipt_name` |
| `formData.kitchenPrintName` | `formData.kitchen_print_name` |
| `formData.kitchenPrinterId` | `formData.print_destinations?.[0]` |
| `formData.isKitchenPrintEnabled` | `(formData.print_destinations?.length ?? 0) > 0` |
| `formData.isLabelPrintEnabled` | `formData.is_label_print_enabled` |

**Step 2: 更新 setFormField 调用**

所有 `setFormField('categoryId', ...)` → `setFormField('category', ...)`
等等...

**Step 3: Commit**

```bash
git add src/screens/Settings/forms/ProductForm.tsx
git commit -m "refactor(ProductForm): use snake_case field names

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 5: 更新 CategoryForm 组件

**Files:**
- Modify: `src/screens/Settings/forms/CategoryForm.tsx`

**Step 1: 替换字段访问**

| 旧 | 新 |
|---|---|
| `formData.kitchenPrinterId` | `formData.print_destinations?.[0]` |
| `formData.isKitchenPrintEnabled` | `(formData.print_destinations?.length ?? 0) > 0` |
| `formData.isLabelPrintEnabled` | `formData.is_label_print_enabled` |
| `formData.isVirtual` | `formData.is_virtual` |
| `formData.tagIds` | `formData.tag_ids` |
| `formData.matchMode` | `formData.match_mode` |

**Step 2: Commit**

```bash
git add src/screens/Settings/forms/CategoryForm.tsx
git commit -m "refactor(CategoryForm): use snake_case field names

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 6: 更新 TagForm 组件

**Files:**
- Modify: `src/screens/Settings/forms/TagForm.tsx`

**Step 1: 替换字段访问**

| 旧 | 新 |
|---|---|
| `formData.displayOrder` | `formData.display_order` |

**Step 2: Commit**

```bash
git add src/screens/Settings/forms/TagForm.tsx
git commit -m "refactor(TagForm): use snake_case field names

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 7: 更新 ZoneForm 组件

**Files:**
- Modify: `src/screens/Settings/forms/ZoneForm.tsx`

**Step 1: 替换字段访问**

| 旧 | 新 |
|---|---|
| `formData.surchargeType` | `formData.surcharge_type` |
| `formData.surchargeAmount` | `formData.surcharge_amount` |

**Step 2: Commit**

```bash
git add src/screens/Settings/forms/ZoneForm.tsx
git commit -m "refactor(ZoneForm): use snake_case field names

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 8: 更新 EntityFormModal 提交逻辑

**Files:**
- Modify: `src/screens/Settings/EntityFormModal.tsx`

**Step 1: 更新 TABLE 提交**

```typescript
// 旧
const tablePayload = { name: formData.name.trim(), zone: formData.zoneId, capacity: formData.capacity };
// 新
const tablePayload = { name: formData.name.trim(), zone: formData.zone, capacity: formData.capacity };
```

**Step 2: 更新 PRODUCT 提交**

```typescript
// 直接使用 formData 字段，无需转换
const productPayload = {
  name: formData.name.trim(),
  category: formData.category,
  image: formData.image,
  sort_order: formData.sort_order,
  tax_rate: formData.tax_rate,
  receipt_name: formData.receipt_name,
  kitchen_print_name: formData.kitchen_print_name,
  print_destinations: formData.print_destinations,
  is_label_print_enabled: formData.is_label_print_enabled,
  tags: formData.tags,
  specs: formData.specs,
};
```

**Step 3: 更新 CATEGORY 提交**

```typescript
const categoryPayload = {
  name: formData.name.trim(),
  sort_order: formData.sort_order,
  print_destinations: formData.print_destinations,
  is_label_print_enabled: formData.is_label_print_enabled,
  is_virtual: formData.is_virtual,
  tag_ids: formData.tag_ids,
  match_mode: formData.match_mode,
};
```

**Step 4: 更新 TAG 提交**

```typescript
const tagPayload = {
  name: formData.name.trim(),
  color: formData.color,
  display_order: formData.display_order,
};
```

**Step 5: Commit**

```bash
git add src/screens/Settings/EntityFormModal.tsx
git commit -m "refactor(EntityFormModal): use snake_case fields directly

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 9: 更新验证函数

**Files:**
- Modify: `src/core/stores/settings/useSettingsStore.ts` (validateSettingsForm)

**Step 1: 更新字段名**

```typescript
function validateSettingsForm(entity: ModalEntity, formData: FormData): Record<string, string | undefined> {
  const errors: Record<string, string | undefined> = {};
  if (entity === 'TABLE') {
    if (!formData.name?.trim()) errors.name = 'settings.errors.table.nameRequired';
    if (!formData.zone?.trim()) errors.zone = 'settings.errors.table.zoneRequired';  // 改 zoneId → zone
    if ((formData.capacity ?? 0) < 1) errors.capacity = 'settings.errors.table.capacityMin';
  }
  // ... 其他实体
  return errors;
}
```

**Step 2: Commit**

```bash
git add src/core/stores/settings/useSettingsStore.ts
git commit -m "refactor(settings): update validation to use snake_case

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 10: 运行完整类型检查

**Step 1: TypeScript 检查**

```bash
npx tsc --noEmit
```

Expected: 0 errors

**Step 2: 测试应用**

```bash
npm run tauri:dev
```

手动测试：
- [ ] 创建/编辑 Table
- [ ] 创建/编辑 Product
- [ ] 创建/编辑 Category
- [ ] 创建/编辑 Tag
- [ ] 创建/编辑 Zone

**Step 3: Final commit**

```bash
git add -A
git commit -m "test: verify snake_case refactor works correctly

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## 注意事项

1. **不保留兼容性** - 直接删除旧字段，不做别名
2. **严格类型检查** - TypeScript 会在编译期捕获所有字段名错误
3. **无转换层** - FormData 字段直接用于 API 调用，无需 camelCase ↔ snake_case 转换
