# Console CRUD Parity Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Bring crab-console's 9 existing CRUD entities to feature parity with red_coral POS, including full bidirectional sync.

**Architecture:** Phase 1 adds missing crab-cloud API endpoints (batch sort, bulk delete, option CRUD) with StoreOp push to edge + edge-server handler. Phase 2 enhances all console frontend Modals and list views to use existing + new APIs.

**Tech Stack:** Rust (Axum, SQLx, PostgreSQL), TypeScript (React 19, Zustand, Tailwind CSS, @dnd-kit)

---

## Phase 1: crab-cloud API + Edge Sync

### Task 1: Batch Sort Order — Products

**Files:**
- Modify: `shared/src/cloud/store_op.rs` — add `BatchUpdateProductSortOrder` variant
- Modify: `crab-cloud/src/api/store/product.rs` — add handler
- Modify: `crab-cloud/src/api/mod.rs:96-102` — add route
- Modify: `crab-cloud/src/db/store/product.rs` — add DB function
- Modify: `edge-server/src/cloud/rpc_executor.rs` — add match arm
- Create: `edge-server/src/cloud/ops/batch.rs` (or add to existing catalog ops)

**Step 1: Add StoreOp variant**

In `shared/src/cloud/store_op.rs`, after `DeleteProduct` (line 38):

```rust
BatchUpdateProductSortOrder {
    items: Vec<SortOrderItem>,
},
```

Add the shared type (at bottom of file, before `StoreOpResult`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortOrderItem {
    pub id: i64,
    pub sort_order: i32,
}
```

**Step 2: Add cloud DB function**

In `crab-cloud/src/db/store/product.rs`, add:

```rust
pub async fn batch_update_sort_order(
    pool: &PgPool,
    edge_server_id: i64,
    items: &[shared::cloud::store_op::SortOrderItem],
) -> Result<(), BoxError> {
    let now = shared::util::now_millis();
    let mut tx = pool.begin().await?;
    for item in items {
        sqlx::query(
            "UPDATE store_products SET sort_order = $1, updated_at = $2 \
             WHERE edge_server_id = $3 AND source_id = $4"
        )
        .bind(item.sort_order)
        .bind(now)
        .bind(edge_server_id)
        .bind(item.id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
```

**Step 3: Add cloud API handler**

In `crab-cloud/src/api/store/product.rs`, add:

```rust
pub async fn batch_update_product_sort_order(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(req): Json<BatchSortOrderRequest>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    store::batch_update_sort_order_products(&state.pool, store_id, &req.items)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;
    push_to_edge(
        &state,
        store_id,
        StoreOp::BatchUpdateProductSortOrder { items: req.items },
    )
    .await;
    Ok(Json(StoreOpResult::ok()))
}
```

Add request type in same file:

```rust
#[derive(serde::Deserialize)]
pub struct BatchSortOrderRequest {
    pub items: Vec<shared::cloud::store_op::SortOrderItem>,
}
```

**Step 4: Register route**

In `crab-cloud/src/api/mod.rs`, after product routes (line 102):

```rust
.route(
    "/api/tenant/stores/{id}/products/sort-order",
    patch(store::batch_update_product_sort_order),
)
```

**Step 5: Add edge-server handler**

In `edge-server/src/cloud/rpc_executor.rs`, add match arm:

```rust
StoreOp::BatchUpdateProductSortOrder { items } => {
    catalog::batch_update_product_sort_order(state, items.clone()).await
}
```

Implement in edge-server catalog ops (e.g. `edge-server/src/cloud/ops/catalog.rs`):

```rust
pub async fn batch_update_product_sort_order(
    state: &ServerState,
    items: Vec<SortOrderItem>,
) -> StoreOpResult {
    for item in &items {
        if let Err(e) = sqlx::query("UPDATE product SET sort_order = ? WHERE id = ?")
            .bind(item.sort_order)
            .bind(item.id)
            .execute(&*state.pool)
            .await
        {
            return StoreOpResult::err(e.to_string());
        }
    }
    state.broadcast_sync::<()>(
        SyncResource::Product,
        SyncChangeType::Updated,
        "batch-sort",
        None,
        true,
    ).await;
    StoreOpResult::ok()
}
```

**Step 6: Verify**

```bash
cargo check --workspace
cargo clippy --workspace
```

**Step 7: Commit**

```
feat(cloud): add batch sort order endpoint for products
```

---

### Task 2: Batch Sort Order — Categories

Same pattern as Task 1, but for categories.

**Files:**
- Modify: `shared/src/cloud/store_op.rs` — add `BatchUpdateCategorySortOrder` variant
- Modify: `crab-cloud/src/api/store/category.rs` — add handler
- Modify: `crab-cloud/src/api/mod.rs` — add route
- Modify: `crab-cloud/src/db/store/category.rs` — add DB function
- Modify: `edge-server/src/cloud/rpc_executor.rs` — add match arm

**Implementation:** Identical structure to Task 1, replacing `product` with `category`, `store_products` with `store_categories`.

**Route:** `PATCH /api/tenant/stores/{id}/categories/sort-order`

**Commit:** `feat(cloud): add batch sort order endpoint for categories`

---

### Task 3: Bulk Delete — Products

**Files:**
- Modify: `crab-cloud/src/api/store/product.rs` — add handler
- Modify: `crab-cloud/src/api/mod.rs` — add route
- Modify: `crab-cloud/src/db/store/product.rs` — add DB function

**Step 1: Add cloud DB function**

In `crab-cloud/src/db/store/product.rs`:

```rust
pub async fn bulk_delete_products(
    pool: &PgPool,
    edge_server_id: i64,
    source_ids: &[i64],
) -> Result<u64, BoxError> {
    if source_ids.is_empty() {
        return Ok(0);
    }
    // Build placeholders: $2, $3, $4, ...
    let placeholders: Vec<String> = (2..=source_ids.len() + 1)
        .map(|i| format!("${i}"))
        .collect();
    let sql = format!(
        "DELETE FROM store_products WHERE edge_server_id = $1 AND source_id IN ({})",
        placeholders.join(", ")
    );
    let mut q = sqlx::query(&sql).bind(edge_server_id);
    for id in source_ids {
        q = q.bind(id);
    }
    let result = q.execute(pool).await?;
    Ok(result.rows_affected())
}
```

**Step 2: Add handler**

In `crab-cloud/src/api/store/product.rs`:

```rust
#[derive(serde::Deserialize)]
pub struct BulkDeleteRequest {
    pub ids: Vec<i64>,
}

pub async fn bulk_delete_products(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(req): Json<BulkDeleteRequest>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    store::bulk_delete_products(&state.pool, store_id, &req.ids)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;
    // Push individual DeleteProduct ops to edge
    for id in &req.ids {
        push_to_edge(&state, store_id, StoreOp::DeleteProduct { id: *id }).await;
    }
    Ok(Json(StoreOpResult::ok()))
}
```

**Step 3: Register route**

In `crab-cloud/src/api/mod.rs`, after product sort-order route:

```rust
.route(
    "/api/tenant/stores/{id}/products/bulk-delete",
    post(store::bulk_delete_products),
)
```

**Step 4: Verify & commit**

```bash
cargo check --workspace && cargo clippy --workspace
```

```
feat(cloud): add bulk delete endpoint for products
```

---

### Task 4: Attribute Option Independent CRUD

**Files:**
- Modify: `shared/src/cloud/store_op.rs` — add 3 new variants
- Modify: `shared/src/models/attribute.rs` — add `AttributeOptionCreate`, `AttributeOptionUpdate` types
- Modify: `crab-cloud/src/api/store/attribute.rs` — add 4 handlers
- Modify: `crab-cloud/src/api/mod.rs` — add 4 routes
- Modify: `crab-cloud/src/db/store/attribute.rs` — add 4 DB functions
- Modify: `edge-server/src/cloud/rpc_executor.rs` — add 3 match arms
- Modify: `edge-server/src/cloud/ops/attribute.rs` — add 3 op handlers
- Modify: `edge-server/src/db/repository/attribute.rs` — add 3 repo functions

**Step 1: Add shared types**

In `shared/src/models/attribute.rs`, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeOptionCreate {
    pub name: String,
    #[serde(default)]
    pub price_modifier: f64,
    #[serde(default)]
    pub display_order: i32,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    #[serde(default)]
    pub enable_quantity: bool,
    pub max_quantity: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeOptionUpdate {
    pub name: Option<String>,
    pub price_modifier: Option<f64>,
    pub display_order: Option<i32>,
    pub is_active: Option<bool>,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub enable_quantity: Option<bool>,
    pub max_quantity: Option<i32>,
}
```

**Step 2: Add StoreOp variants**

In `shared/src/cloud/store_op.rs`, after `DeleteAttribute` (line 66):

```rust
// ── Attribute Option ──
CreateAttributeOption {
    attribute_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<i64>,
    data: crate::models::attribute::AttributeOptionCreate,
},
UpdateAttributeOption {
    id: i64,
    data: crate::models::attribute::AttributeOptionUpdate,
},
DeleteAttributeOption {
    id: i64,
},
```

**Step 3: Add cloud DB functions**

In `crab-cloud/src/db/store/attribute.rs`, add functions for:
- `create_option_direct(pool, edge_server_id, attribute_source_id, data) -> Result<i64, BoxError>` — INSERT into `store_attribute_options`, return new `source_id`
- `update_option_direct(pool, edge_server_id, option_source_id, data) -> Result<(), BoxError>` — COALESCE UPDATE
- `delete_option_direct(pool, edge_server_id, option_source_id) -> Result<(), BoxError>` — DELETE
- `batch_update_option_sort_order(pool, edge_server_id, items) -> Result<(), BoxError>` — batch UPDATE

Each follows the existing pattern: lookup PG id via `source_id`, transaction, snowflake_id for create.

**Step 4: Add cloud API handlers**

In `crab-cloud/src/api/store/attribute.rs`, add 4 handlers:
- `create_attribute_option` — POST `/stores/{id}/attributes/{aid}/options`
- `update_attribute_option` — PUT `/stores/{id}/attributes/{aid}/options/{oid}`
- `delete_attribute_option` — DELETE `/stores/{id}/attributes/{aid}/options/{oid}`
- `batch_update_option_sort_order` — PATCH `/stores/{id}/attributes/{aid}/options/sort-order`

Each follows: `verify_store → db_func → increment_version → push_to_edge → Ok(StoreOpResult)`.

**Step 5: Register routes**

In `crab-cloud/src/api/mod.rs`, after attribute unbind route (line 134):

```rust
.route(
    "/api/tenant/stores/{id}/attributes/{aid}/options",
    post(store::create_attribute_option),
)
.route(
    "/api/tenant/stores/{id}/attributes/{aid}/options/{oid}",
    put(store::update_attribute_option).delete(store::delete_attribute_option),
)
.route(
    "/api/tenant/stores/{id}/attributes/{aid}/options/sort-order",
    patch(store::batch_update_option_sort_order),
)
```

**Step 6: Add edge-server handlers**

In `edge-server/src/cloud/rpc_executor.rs`, add match arms:

```rust
StoreOp::CreateAttributeOption { attribute_id, id, data } =>
    attribute::create_option(state, *attribute_id, *id, data.clone()).await,
StoreOp::UpdateAttributeOption { id, data } =>
    attribute::update_option(state, *id, data.clone()).await,
StoreOp::DeleteAttributeOption { id } =>
    attribute::delete_option(state, *id).await,
```

In `edge-server/src/cloud/ops/attribute.rs`, implement:
- `create_option` — INSERT into `attribute_option`, broadcast sync
- `update_option` — UPDATE `attribute_option`, broadcast sync
- `delete_option` — DELETE from `attribute_option`, broadcast sync

In `edge-server/src/db/repository/attribute.rs`, add:
- `create_option(pool, attribute_id, assigned_id, data) -> RepoResult<AttributeOption>`
- `update_option(pool, id, data) -> RepoResult<AttributeOption>`
- `delete_option(pool, id) -> RepoResult<()>`

**Step 7: Verify & commit**

```bash
cargo check --workspace && cargo clippy --workspace
```

```
feat(cloud+edge): add independent attribute option CRUD with bidirectional sync
```

---

## Phase 2: Console Frontend Enhancement

### Task 5: TypeScript Types + API Client 补全

**Files:**
- Modify: `crab-console/src/core/types/store.ts` — add missing types
- Modify: `crab-console/src/infrastructure/api/store.ts` — add new API functions

**Step 1: Add missing types**

In `crab-console/src/core/types/store.ts`:

```typescript
// Batch sort order
export interface SortOrderItem {
  id: number;
  sort_order: number;
}

// Attribute option independent CRUD
export interface AttributeOptionCreate {
  name: string;
  price_modifier?: number;
  display_order?: number;
  receipt_name?: string;
  kitchen_print_name?: string;
  enable_quantity?: boolean;
  max_quantity?: number;
}

export interface AttributeOptionUpdate {
  name?: string;
  price_modifier?: number;
  display_order?: number;
  is_active?: boolean;
  receipt_name?: string;
  kitchen_print_name?: string;
  enable_quantity?: boolean;
  max_quantity?: number;
}

// Attribute binding
export interface BindAttributeRequest {
  owner: { type: 'Product' | 'Category'; id: number };
  attribute_id: number;
  is_required?: boolean;
  display_order?: number;
  default_option_ids?: number[];
}

// Extend existing ProductCreate/ProductUpdate with missing fields
// (add to existing interfaces)
// receipt_name?: string;
// kitchen_print_name?: string;
// tags?: number[];

// Extend CategoryCreate/CategoryUpdate with missing fields
// tag_ids?: number[];
// kitchen_print_destinations?: number[];
// label_print_destinations?: number[];
```

**Step 2: Add API functions**

In `crab-console/src/infrastructure/api/store.ts`:

```typescript
// Batch sort order
export async function batchUpdateProductSortOrder(token: string, storeId: number, items: SortOrderItem[]) {
  return request<StoreOpResult>('PATCH', `/api/tenant/stores/${storeId}/products/sort-order`, { items }, token);
}
export async function batchUpdateCategorySortOrder(token: string, storeId: number, items: SortOrderItem[]) {
  return request<StoreOpResult>('PATCH', `/api/tenant/stores/${storeId}/categories/sort-order`, { items }, token);
}

// Bulk delete
export async function bulkDeleteProducts(token: string, storeId: number, ids: number[]) {
  return request<StoreOpResult>('POST', `/api/tenant/stores/${storeId}/products/bulk-delete`, { ids }, token);
}

// Attribute binding
export async function bindAttribute(token: string, storeId: number, data: BindAttributeRequest) {
  return request<StoreOpResult>('POST', `/api/tenant/stores/${storeId}/attributes/bind`, data, token);
}
export async function unbindAttribute(token: string, storeId: number, bindingId: number) {
  return request<StoreOpResult>('POST', `/api/tenant/stores/${storeId}/attributes/unbind`, { binding_id: bindingId }, token);
}

// Attribute option CRUD
export async function createAttributeOption(token: string, storeId: number, attrId: number, data: AttributeOptionCreate) {
  return request<StoreOpResult>('POST', `/api/tenant/stores/${storeId}/attributes/${attrId}/options`, data, token);
}
export async function updateAttributeOption(token: string, storeId: number, attrId: number, optId: number, data: AttributeOptionUpdate) {
  return request<StoreOpResult>('PUT', `/api/tenant/stores/${storeId}/attributes/${attrId}/options/${optId}`, data, token);
}
export async function deleteAttributeOption(token: string, storeId: number, attrId: number, optId: number) {
  return request<StoreOpResult>('DELETE', `/api/tenant/stores/${storeId}/attributes/${attrId}/options/${optId}`, undefined, token);
}
export async function batchUpdateOptionSortOrder(token: string, storeId: number, attrId: number, items: SortOrderItem[]) {
  return request<StoreOpResult>('PATCH', `/api/tenant/stores/${storeId}/attributes/${attrId}/options/sort-order`, { items }, token);
}
```

**Step 3: Verify & commit**

```bash
cd red_coral && npx tsc --noEmit  # (if shared types affect red_coral)
```

```
feat(console): add TypeScript types and API client for new endpoints
```

---

### Task 6: Product Modal Enhancement

**Files:**
- Modify: `crab-console/src/features/product/ProductManagement.tsx`
- Modify: `crab-console/src/core/types/store.ts` (if not done in Task 5)

**Enhancements:**

1. **New form fields in Modal:**
   - `receipt_name` (text input)
   - `kitchen_print_name` (text input)
   - Image upload (call `uploadImage` → display preview → set `image` field)
   - `is_active` toggle in edit mode

2. **Specs enhancement:**
   - Add `display_order` (number) per spec
   - Show `is_active` toggle per spec (not just in create)

3. **Tag assignment:**
   - Add tag multi-select section in Modal
   - Load tags list on mount (use `listTags` API)
   - Display as clickable pills/badges, selected ones highlighted
   - Send `tags: number[]` in create/update payload

4. **Attribute binding panel (read-only display + bind/unbind):**
   - Add "Attributes" section in Modal (only in edit mode)
   - Load attribute bindings for this product (from product's `attribute_bindings` field or separate query)
   - "Bind Attribute" button → shows available attributes dropdown → calls `bindAttribute` API
   - "Unbind" button per bound attribute → calls `unbindAttribute` API

5. **List enhancements:**
   - Batch select mode (DataTable already supports `selectable` + `onBatchDelete`)
   - Wire `bulkDeleteProducts` API to batch delete handler
   - Drag-and-drop sort (use @dnd-kit, call `batchUpdateProductSortOrder` on drop)

**Commit:** `feat(console): enhance Product modal with tags, attributes, image, batch ops`

---

### Task 7: Category Modal Enhancement

**Files:**
- Modify: `crab-console/src/features/category/CategoryManagement.tsx`

**Enhancements:**

1. **New form fields in Modal:**
   - `tag_ids` — multi-select tag picker (same component as Product)
   - `kitchen_print_destinations` — multi-select print destination IDs
   - `label_print_destinations` — multi-select print destination IDs

2. **Attribute binding panel:**
   - Same as Product Task 6 but with `BindingOwner::Category`
   - Include `default_option_ids` selector per binding

3. **List enhancements:**
   - Drag-and-drop sort (call `batchUpdateCategorySortOrder`)

**Commit:** `feat(console): enhance Category modal with tags, print destinations, attribute binding`

---

### Task 8: Attribute Modal Enhancement

**Files:**
- Modify: `crab-console/src/features/attribute/AttributeManagement.tsx`

**Enhancements:**

1. **Attribute-level fields:**
   - `show_on_receipt` checkbox
   - `receipt_name` text input
   - `show_on_kitchen_print` checkbox
   - `kitchen_print_name` text input

2. **Option independent CRUD (replace current inline array):**
   - In edit mode: show options as a list with inline edit buttons
   - Each option: `name`, `price_modifier`, `display_order`, `receipt_name`, `kitchen_print_name`, `enable_quantity`, `max_quantity`
   - "Add Option" → calls `createAttributeOption` API → refreshes list
   - Edit icon per option → inline edit → calls `updateAttributeOption`
   - Delete icon per option → confirm → calls `deleteAttributeOption`
   - Drag-and-drop → calls `batchUpdateOptionSortOrder`

3. **In create mode:** Keep current behavior (options submitted with attribute create).
   In edit mode: Switch to independent option CRUD.

**Commit:** `feat(console): enhance Attribute modal with option CRUD and print fields`

---

### Task 9: Employee Modal Enhancement

**Files:**
- Modify: `crab-console/src/features/employee/EmployeeManagement.tsx`

**Enhancements:**

1. **is_active toggle:**
   - Add toggle in DataTable (direct API call on toggle)
   - Add toggle in edit Modal

2. **Password reset:**
   - Separate "Reset Password" button in edit Modal
   - Opens sub-form with new password field
   - Calls `updateEmployee` with only `password` field

3. **Role display enhancement:**
   - Show role name in Modal header when editing
   - Consider showing permissions summary (read-only)

**Commit:** `feat(console): enhance Employee modal with active toggle and password reset`

---

### Task 10: Price Rule Modal Enhancement

**Files:**
- Modify: `crab-console/src/features/price-rule/PriceRuleManagement.tsx`

**Enhancements:**

1. **Multi-step wizard:**
   - Step 1: Basic info (name, display_name, receipt_name, description)
   - Step 2: Rule config (rule_type, product_scope, target_id, adjustment_type, adjustment_value, is_stackable, is_exclusive)
   - Step 3: Scheduling (valid_from, valid_until, active_days, active_start_time, active_end_time, zone_scope)
   - Navigation: Back/Next buttons, step indicator

2. **target_id field:**
   - Conditional on product_scope:
     - GLOBAL: hidden
     - CATEGORY: category dropdown
     - TAG: tag dropdown
     - PRODUCT: product dropdown
   - Load corresponding list on scope change

3. **Scheduling fields:**
   - `valid_from` / `valid_until`: date picker (`<input type="date">`)
   - `active_days`: 7 checkboxes (Mon-Sun), value `number[]`
   - `active_start_time` / `active_end_time`: time picker (`<input type="time">`)

4. **zone_scope:**
   - SelectField: "All" | "Retail" | specific zone (load zones list)

**Commit:** `feat(console): enhance Price Rule modal with wizard, scheduling, and zone scope`

---

## Shared Components to Build

### Task 11: Reusable Components

**Files:**
- Create: `crab-console/src/shared/components/TagSelector.tsx`
- Create: `crab-console/src/shared/components/AttributeBindingPanel.tsx`
- Create: `crab-console/src/shared/components/ImageUploader.tsx`
- Create: `crab-console/src/shared/components/SortableList.tsx`
- Create: `crab-console/src/shared/components/StepWizard.tsx`

**Note:** Build these as needed during Tasks 6-10. Create each component when first used, then reuse in subsequent tasks.

1. **TagSelector** — Multi-select pill/badge component for tag assignment
2. **AttributeBindingPanel** — Displays bound attributes, add/remove buttons, used in Product + Category Modals
3. **ImageUploader** — Upload + preview + S3 URL, used in Product Modal
4. **SortableList** — @dnd-kit wrapper for drag-and-drop sorting, used in Product/Category lists + Attribute options
5. **StepWizard** — Step indicator + Back/Next navigation, used in Price Rule Modal

**Commit:** `feat(console): add shared components (TagSelector, AttributeBindingPanel, etc.)`

---

## Execution Order

```
Task 1  → Batch sort (products)          [API + edge]
Task 2  → Batch sort (categories)        [API + edge]
Task 3  → Bulk delete (products)         [API only]
Task 4  → Attribute option CRUD          [API + edge + shared types]
─── Phase 1 complete: cargo check --workspace ───
Task 5  → TS types + API client          [console]
Task 11 → Shared components              [console, build incrementally]
Task 6  → Product Modal                  [console]
Task 7  → Category Modal                 [console]
Task 8  → Attribute Modal                [console]
Task 9  → Employee Modal                 [console]
Task 10 → Price Rule Modal               [console]
─── Phase 2 complete: npx tsc --noEmit ───
```

## Verification

After all tasks:
```bash
cargo check --workspace
cargo clippy --workspace
cargo test --workspace --lib
cd crab-console && npx tsc --noEmit
```
