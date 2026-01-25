# Order Archive Graph Model Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement graph-based order archiving with clean data model, removing all legacy JSON storage and adaptation layers.

**Architecture:** Use SurrealDB RELATE edges for order→items, order→payments, order→events relationships. Store only essential fields. Frontend receives clean API responses without conversion.

**Tech Stack:** Rust (edge-server), SurrealDB, TypeScript (red_coral), Tauri

---

## Task 1: Update SurrealDB Schema

**Files:**
- Modify: `edge-server/migrations/schemas/order.surql`

**Step 1: Write the new schema**

Replace entire file with:

```sql
-- Order Schema (Graph Model)
-- 归档订单使用图边关系，只存储核心数据

-- =============================================================================
-- order 表
-- =============================================================================

DEFINE TABLE OVERWRITE order TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE receipt_number ON order TYPE string
    ASSERT string::len($value) > 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE zone_name ON order TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE table_name ON order TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE status ON order TYPE string
    ASSERT $value IN ["COMPLETED", "VOID", "MOVED", "MERGED"]
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_retail ON order TYPE bool
    DEFAULT false
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE guest_count ON order TYPE option<int>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE total_amount ON order TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE paid_amount ON order TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE discount_amount ON order TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE surcharge_amount ON order TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE start_time ON order TYPE datetime
    DEFAULT time::now()
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE end_time ON order TYPE option<datetime>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE operator_id ON order TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE operator_name ON order TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE related_order_id ON order TYPE option<record<order>>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE prev_hash ON order TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE curr_hash ON order TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE created_at ON order TYPE datetime
    DEFAULT time::now()
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE order_receipt ON order FIELDS receipt_number UNIQUE;
DEFINE INDEX OVERWRITE order_status ON order FIELDS status;
DEFINE INDEX OVERWRITE order_end_time ON order FIELDS end_time;
DEFINE INDEX OVERWRITE order_hash ON order FIELDS curr_hash;

-- =============================================================================
-- order_item 表
-- =============================================================================

DEFINE TABLE OVERWRITE order_item TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE spec ON order_item TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE instance_id ON order_item TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE name ON order_item TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE spec_name ON order_item TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE price ON order_item TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE quantity ON order_item TYPE int
    DEFAULT 1
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE unpaid_quantity ON order_item TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE unit_price ON order_item TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE line_total ON order_item TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE discount_amount ON order_item TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE surcharge_amount ON order_item TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE note ON order_item TYPE option<string>
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE order_item_spec ON order_item FIELDS spec;
DEFINE INDEX OVERWRITE order_item_instance ON order_item FIELDS instance_id;

-- =============================================================================
-- order_item_option 表
-- =============================================================================

DEFINE TABLE OVERWRITE order_item_option TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE attribute_name ON order_item_option TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE option_name ON order_item_option TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE price ON order_item_option TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

-- =============================================================================
-- order_payment 表
-- =============================================================================

DEFINE TABLE OVERWRITE order_payment TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE method ON order_payment TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE amount ON order_payment TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE time ON order_payment TYPE datetime
    DEFAULT time::now()
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE reference ON order_payment TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE cancelled ON order_payment TYPE bool
    DEFAULT false
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE cancel_reason ON order_payment TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE split_items ON order_payment TYPE array
    DEFAULT []
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE payment_method ON order_payment FIELDS method;
DEFINE INDEX OVERWRITE payment_time ON order_payment FIELDS time;

-- =============================================================================
-- order_event 表
-- =============================================================================

DEFINE TABLE OVERWRITE order_event TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE event_type ON order_event TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE timestamp ON order_event TYPE datetime
    DEFAULT time::now()
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE data ON order_event TYPE option<object>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE prev_hash ON order_event TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE curr_hash ON order_event TYPE string
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE event_time ON order_event FIELDS timestamp;

-- =============================================================================
-- 图边关系
-- =============================================================================

DEFINE TABLE OVERWRITE has_item TYPE RELATION
    FROM order TO order_item SCHEMAFULL
    PERMISSIONS FOR select, create
        WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE TABLE OVERWRITE has_option TYPE RELATION
    FROM order_item TO order_item_option SCHEMAFULL
    PERMISSIONS FOR select, create
        WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE TABLE OVERWRITE has_payment TYPE RELATION
    FROM order TO order_payment SCHEMAFULL
    PERMISSIONS FOR select, create
        WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE TABLE OVERWRITE has_event TYPE RELATION
    FROM order TO order_event SCHEMAFULL
    PERMISSIONS FOR select, create
        WHERE $auth.role = role:admin OR $auth.id = employee:admin;
```

**Step 2: Commit schema**

```bash
git add edge-server/migrations/schemas/order.surql
git commit -m "feat(schema): update order to graph model with RELATE edges"
```

---

## Task 2: Update Rust Order Models

**Files:**
- Modify: `edge-server/src/db/models/order.rs`

**Step 1: Replace Order model**

Replace entire file with clean models matching new schema. Key changes:
- Remove `snapshot_json`
- Add `is_retail`, `operator_name`
- Add `instance_id`, `unpaid_quantity` to OrderItem
- Add `OrderItemOption` with attribute_name/option_name
- Add `cancelled`, `cancel_reason`, `split_items` to OrderPayment
- Add API response types: `OrderSummary`, `OrderDetail`

**Step 2: Verify compilation**

```bash
cargo check -p edge-server
```

Expected: Compilation errors in archive.rs and handler.rs (will fix in next tasks)

**Step 3: Commit models**

```bash
git add edge-server/src/db/models/order.rs
git commit -m "feat(models): update Order models for graph storage"
```

---

## Task 3: Rewrite Archive Service

**Files:**
- Modify: `edge-server/src/orders/archive.rs`

**Step 1: Update archive_order_internal**

Key changes:
- Remove snapshot_json serialization
- Use RELATE to create graph edges
- Update hash calculation to include payload
- Store items with has_item edges
- Store options with has_option edges
- Store payments with has_payment edges

**Step 2: Verify compilation**

```bash
cargo check -p edge-server
```

**Step 3: Run tests**

```bash
cargo test -p edge-server --lib
```

**Step 4: Commit**

```bash
git add edge-server/src/orders/archive.rs
git commit -m "feat(archive): use RELATE edges for graph storage"
```

---

## Task 4: Update Order Repository

**Files:**
- Modify: `edge-server/src/db/repository/order.rs`

**Step 1: Update create_archived**

Remove snapshot_json, items_json, payments_json from CREATE query.

**Step 2: Add graph query helper**

Add method to fetch order with relations using graph traversal:

```rust
pub async fn get_order_detail(&self, order_id: &str) -> RepoResult<OrderDetail>
```

**Step 3: Verify and commit**

```bash
cargo check -p edge-server
git add edge-server/src/db/repository/order.rs
git commit -m "feat(repo): add graph traversal for order detail"
```

---

## Task 5: Update API Handler

**Files:**
- Modify: `edge-server/src/api/orders/handler.rs`

**Step 1: Update fetch_order_list**

Return `OrderSummary` format directly from query.

**Step 2: Rewrite get_by_id**

Use graph traversal query:

```sql
SELECT
    record::id(id) AS order_id,
    receipt_number,
    table_name,
    zone_name,
    string::uppercase(status) AS status,
    is_retail,
    guest_count,
    total_amount AS total,
    paid_amount,
    discount_amount AS total_discount,
    surcharge_amount AS total_surcharge,
    time::millis(start_time) AS start_time,
    time::millis(end_time) AS end_time,
    operator_name,
    ->has_item->order_item AS items,
    ->has_payment->order_payment AS payments,
    ->has_event->order_event AS timeline
FROM order WHERE id = $id
```

**Step 3: Verify and commit**

```bash
cargo check -p edge-server
cargo test -p edge-server --lib
git add edge-server/src/api/orders/handler.rs
git commit -m "feat(api): return graph-based order detail"
```

---

## Task 6: Update Tauri Commands

**Files:**
- Modify: `red_coral/src-tauri/src/commands/orders.rs`

**Step 1: Update response types**

Ensure Tauri commands return data matching new API format.

**Step 2: Verify compilation**

```bash
cargo check -p red-coral
```

**Step 3: Commit**

```bash
git add red_coral/src-tauri/src/commands/orders.rs
git commit -m "feat(tauri): update order commands for new API format"
```

---

## Task 7: Create Frontend Types

**Files:**
- Create: `red_coral/src/core/domain/types/archivedOrder.ts`

**Step 1: Write types**

```typescript
// 列表项
export interface OrderSummary {
  order_id: string;
  receipt_number: string;
  table_name: string | null;
  status: 'COMPLETED' | 'VOID' | 'MOVED' | 'MERGED';
  is_retail: boolean;
  total: number;
  guest_count: number;
  start_time: number;
  end_time: number;
}

// 订单项选项
export interface OrderItemOption {
  attribute_name: string;
  option_name: string;
  price_modifier: number;
}

// 订单项
export interface OrderItemDetail {
  id: string;
  instance_id: string;
  name: string;
  spec_name: string | null;
  price: number;
  quantity: number;
  unpaid_quantity: number;
  unit_price: number;
  line_total: number;
  discount_amount: number;
  surcharge_amount: number;
  note: string | null;
  selected_options: OrderItemOption[];
}

// 分单明细
export interface SplitItem {
  instance_id: string;
  name: string;
  quantity: number;
}

// 支付
export interface OrderPaymentDetail {
  method: string;
  amount: number;
  timestamp: number;
  note: string | null;
  cancelled: boolean;
  cancel_reason: string | null;
  split_items: SplitItem[];
}

// 事件
export interface OrderEventDetail {
  event_id: string;
  event_type: string;
  timestamp: number;
  payload: unknown;
}

// 详情
export interface OrderDetail {
  order_id: string;
  receipt_number: string;
  table_name: string | null;
  zone_name: string | null;
  status: string;
  is_retail: boolean;
  guest_count: number;
  total: number;
  paid_amount: number;
  total_discount: number;
  total_surcharge: number;
  start_time: number;
  end_time: number;
  operator_name: string | null;
  items: OrderItemDetail[];
  payments: OrderPaymentDetail[];
  timeline: OrderEventDetail[];
}
```

**Step 2: Export from index**

Add to `red_coral/src/core/domain/types/index.ts`:
```typescript
export * from './archivedOrder';
```

**Step 3: Commit**

```bash
git add red_coral/src/core/domain/types/archivedOrder.ts
git add red_coral/src/core/domain/types/index.ts
git commit -m "feat(types): add clean archived order types"
```

---

## Task 8: Simplify useHistoryOrderList

**Files:**
- Modify: `red_coral/src/hooks/useHistoryOrderList.ts`

**Step 1: Update to use OrderSummary**

Remove all conversion code, use backend response directly.

**Step 2: Type check**

```bash
cd red_coral && npx tsc --noEmit
```

**Step 3: Commit**

```bash
git add red_coral/src/hooks/useHistoryOrderList.ts
git commit -m "refactor(hooks): simplify useHistoryOrderList"
```

---

## Task 9: Simplify useHistoryOrderDetail

**Files:**
- Modify: `red_coral/src/hooks/useHistoryOrderDetail.ts`

**Step 1: Update to use OrderDetail**

Remove all conversion code, fallback logic, use backend response directly.

**Step 2: Type check**

```bash
cd red_coral && npx tsc --noEmit
```

**Step 3: Commit**

```bash
git add red_coral/src/hooks/useHistoryOrderDetail.ts
git commit -m "refactor(hooks): simplify useHistoryOrderDetail"
```

---

## Task 10: Cleanup Deprecated Code

**Files:**
- Modify: `red_coral/src/core/domain/types/index.ts` (remove old exports)
- Delete: Any unused conversion functions
- Modify: `shared/src/models/order.rs` (if needed)

**Step 1: Search for dead code**

```bash
grep -r "HeldOrder\|OrderSnapshot\|convertArchived" red_coral/src/
```

**Step 2: Remove unused imports and types**

**Step 3: Final type check**

```bash
cd red_coral && npx tsc --noEmit
cargo check --workspace
```

**Step 4: Commit cleanup**

```bash
git add -A
git commit -m "chore: remove deprecated order types and conversion code"
```

---

## Task 11: Integration Test

**Step 1: Run all backend tests**

```bash
cargo test --workspace --lib
```

**Step 2: Run frontend type check**

```bash
cd red_coral && npx tsc --noEmit
```

**Step 3: Manual test**

1. Start dev server: `cd red_coral && npm run tauri:dev`
2. Create and complete an order
3. Check history page - verify list loads
4. Click order - verify detail loads with items/payments/timeline

**Step 4: Final commit**

```bash
git add -A
git commit -m "test: verify graph-based order archiving"
```
