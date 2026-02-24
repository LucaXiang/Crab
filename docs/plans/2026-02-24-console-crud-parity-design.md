# Console CRUD Parity Design

**日期**: 2026-02-24
**目标**: 将 crab-console 已有的 9 个 CRUD 实体增强到与 red_coral 同等深度

## 策略

**先补 API，再统一做前端。** 分两个 Phase 执行。

---

## Phase 1: crab-cloud API 补全

### 1.1 Batch Sort Order

**新增端点:**

```
PATCH /api/tenant/stores/{id}/products/sort-order
PATCH /api/tenant/stores/{id}/categories/sort-order
```

**请求体:**
```json
{ "items": [{ "id": 123, "sort_order": 0 }, { "id": 456, "sort_order": 1 }] }
```

**后端逻辑:** 单事务批量 UPDATE → 递增 catalog_version → 推 StoreOp 到 edge

**涉及文件:**
- `shared/src/cloud/store_op.rs` — 新增 `BatchUpdateProductSortOrder` / `BatchUpdateCategorySortOrder` op
- `crab-cloud/src/api/store/product.rs` — 新增 handler
- `crab-cloud/src/api/store/category.rs` — 新增 handler
- `crab-cloud/src/db/store/product.rs` — 新增 DB 函数
- `crab-cloud/src/db/store/category.rs` — 新增 DB 函数
- `edge-server` 对应的 StoreOp handler

### 1.2 Bulk Delete (Products)

**新增端点:**

```
POST /api/tenant/stores/{id}/products/bulk-delete
```

**请求体:**
```json
{ "ids": [123, 456, 789] }
```

**后端逻辑:** 验证所有 ID 归属 store → 批量删除 → 每个 ID 推一个 DeleteProduct StoreOp

**涉及文件:**
- `crab-cloud/src/api/store/product.rs` — 新增 handler
- `crab-cloud/src/db/store/product.rs` — 新增 DB 函数

### 1.3 Attribute Option 独立 CRUD

**新增端点:**

```
POST   /api/tenant/stores/{id}/attributes/{aid}/options          — 新增选项
PUT    /api/tenant/stores/{id}/attributes/{aid}/options/{oid}    — 更新选项
DELETE /api/tenant/stores/{id}/attributes/{aid}/options/{oid}    — 删除选项
PATCH  /api/tenant/stores/{id}/attributes/{aid}/options/sort-order — 批量排序
```

**涉及文件:**
- `shared/src/cloud/store_op.rs` — 新增 Option 级别的 StoreOp variants
- `crab-cloud/src/api/store/attribute.rs` — 新增 handlers
- `crab-cloud/src/db/store/attribute.rs` — 新增 DB 函数
- `edge-server` 对应的 StoreOp handler

---

## Phase 2: Console 前端全面增强

### 2.1 Product Modal

**当前状态:** name, category, tax_rate, sort_order, specs (name+price)

**新增:**

| 区域 | 字段/功能 |
|------|----------|
| 基础信息 | `receipt_name`, `kitchen_print_name` |
| 图片 | 图片上传 (调用 `/api/tenant/images`) |
| Specs | 每个 spec 增加 `is_default`, `is_active`, `display_order` |
| Tag 绑定 | 多选 tag 选择器 (数据源: 已有 tags 列表) |
| Attribute 绑定 | 已绑定属性列表 + 绑定/解绑操作 (bind/unbind API) |
| 列表 | 批量选择 + 批量删除、拖拽排序 |

**涉及文件:**
- `crab-console/src/features/product/ProductManagement.tsx` — 列表增强
- `crab-console/src/features/product/ProductModal.tsx` (或内部 form) — Modal 增强
- `crab-console/src/infrastructure/api/store.ts` — 新增 API 调用
- `crab-console/src/core/types/store.ts` — 类型补全

### 2.2 Category Modal

**当前状态:** name, sort_order, is_virtual, is_display

**新增:**

| 区域 | 字段/功能 |
|------|----------|
| Tag 关联 | 多选 tag 选择器 (`tag_ids`, 虚拟分类用) |
| Attribute 绑定 | 绑定属性 + 设置默认选项值 |
| Print Destination | `kitchen_print_destinations` + `label_print_destinations` 多选 |
| 列表 | 拖拽排序 |

**涉及文件:**
- `crab-console/src/features/category/CategoryManagement.tsx`
- `crab-console/src/infrastructure/api/store.ts`
- `crab-console/src/core/types/store.ts`

### 2.3 Attribute Modal

**当前状态:** name, is_multi_select, max_selections + options 整体提交

**新增:**

| 区域 | 字段/功能 |
|------|----------|
| 选项管理 | 独立选项 CRUD — 内联编辑 name, price_modifier, display_order |
| 选项排序 | 拖拽排序选项 (新 sort-order API) |
| 选项操作 | 单独添加/删除选项按钮 |

**涉及文件:**
- `crab-console/src/features/attribute/AttributeManagement.tsx`
- `crab-console/src/infrastructure/api/store.ts` — 新增 option CRUD API 调用
- `crab-console/src/core/types/store.ts`

### 2.4 Employee Modal

**当前状态:** username, password, display_name, role_id

**新增:**

| 区域 | 字段/功能 |
|------|----------|
| 密码重置 | 独立"重置密码"按钮 (通过 update API) |
| 启用/禁用 | `is_active` toggle (列表内 + Modal 内) |
| 角色展示 | 角色名称 (Admin/Manager/User) |

**涉及文件:**
- `crab-console/src/features/employee/EmployeeManagement.tsx`
- `crab-console/src/infrastructure/api/management.ts`

### 2.5 Price Rule Modal

**当前状态:** 单页表单

**新增:**

| 区域 | 字段/功能 |
|------|----------|
| 多步骤向导 | Step 1: 基础信息 → Step 2: 范围与金额 → Step 3: 时间约束 |
| 时间窗口 | `active_days` (星期多选) + `active_start_time`/`active_end_time` |
| Zone 范围 | zone scope 选择器 (全部/堂食/指定区域) |
| 有效期 | `valid_from` / `valid_until` 日期选择器 |

**涉及文件:**
- `crab-console/src/features/price-rule/PriceRuleManagement.tsx`
- `crab-console/src/infrastructure/api/store.ts`
- `crab-console/src/core/types/store.ts`

### 2.6 Label Template

Console 已有 LabelEditorScreen，与 red_coral 基本对齐。如有细节差距在实施时补。

### 2.7 Tag / Zone / Table

已基本对齐，无需大改。

---

## 共享组件需求

| 组件 | 用途 |
|------|------|
| `TagSelector` | Product/Category Modal 中的 tag 多选组件 |
| `AttributeBindingPanel` | Product/Category Modal 中的属性绑定/解绑面板 |
| `SortableList` | 拖拽排序通用组件 (基于 @dnd-kit，已有依赖) |
| `BulkActionBar` | 列表批量操作栏 (选中数 + 批量删除按钮) |
| `ImageUploader` | 图片上传组件 (上传 + 预览 + S3 URL) |
| `StepWizard` | Price Rule 多步骤向导容器 |

---

## 不在范围内

- 新增实体 (Marketing Groups, Members, Shifts 等)
- Print Destinations 管理 (设备相关，不适合云端管理)
- 非 CRUD 功能 (Live Orders, Stats, Red Flags 等已有的只读功能)
