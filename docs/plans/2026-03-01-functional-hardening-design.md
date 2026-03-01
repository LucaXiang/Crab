# Functional Hardening Design

**Goal:** Plug POS feature gaps, strengthen data correctness, and enhance Console management capabilities.

**Context:** Full audit confirmed 8 actionable items across 3 phases. All POS items have backend support already — only frontend wiring needed. Data correctness items prevent production drift/data-loss. Console items unblock bulk operations.

---

## Phase 1: POS Feature Completion

### 1.1 Order Note Entry

**Problem:** `AddOrderNote` command fully implemented in edge-server (`orders/actions/add_order_note.rs`), event type `ORDER_NOTE_ADDED` renders in Timeline, but no frontend command function or UI entry point.

**Solution:**
- Add `addOrderNote(orderId, note)` to `red_coral/src/core/stores/order/commands/`
- Add note button in POS order sidebar (OrderDetailMode or ItemActionPanel)
- Simple text input modal, max 200 chars

### 1.2 Comp Item Shortcut

**Problem:** `CompItem` / `UncompItem` commands work, but only accessible through Checkout flow (`CompItemMode.tsx`). In high-traffic dine-in, waiters must enter checkout to comp an item.

**Solution:**
- Add comp/uncomp buttons to `ItemActionPanel` (long-press menu on cart items)
- Reuse existing `useOrderCommands().compItem()` / `uncompItem()`
- Toggle behavior: if item already comped, show "uncomp" button instead
- Requires `orders:comp` permission (already defined)

---

## Phase 2: Data Correctness & Reliability

### 2.1 state_checksum Enhancement

**Problem:** `OrderSnapshot::compute_checksum()` in `shared/src/order/snapshot.rs` only hashes 5 fields: `items.len()`, `total`, `paid_amount`, `last_sequence`, `status`. Two orders with same total/count but different items produce identical checksums — drift detection misses item-level mutations.

**Solution:**
- Mix each `CartItemSnapshot.instance_id` (u64) into the FNV-1a hash
- Optionally mix `quantity` and `unit_price` for stronger coverage
- This is a non-breaking change (checksum is recomputed on every event apply)

### 2.2 Background Task Panic Recovery

**Problem:** `edge-server/src/core/tasks.rs` — `catch_unwind` captures panics but only logs, task permanently stops. Critical tasks (ArchiveWorker, CloudSyncWorker, VerifyScheduler) become inoperable.

**Solution:**
- Add restart loop with exponential backoff (1s → 2s → 4s → ... → 60s cap)
- Max restart attempts per window (e.g., 5 restarts in 10 minutes, then give up and emit system_issue)
- Log each restart attempt at WARN level

### 2.3 DB Query Error Propagation

**Problem:** `edge-server/src/api/orders/handler.rs` has multiple `unwrap_or_default()` calls that silently swallow database errors, making issues invisible.

**Solution:**
- Replace `unwrap_or_default()` with `.map_err(|e| AppError::database(e.to_string()))?`
- Audit other handlers for same pattern

---

## Phase 3: Console Management Enhancement

### 3.1 Sort-Order APIs

**Problem:** Console product/category management can't reorder items. Cloud API missing `PATCH /api/tenant/stores/{id}/products/sort-order` and equivalent for categories.

**Solution:**
- Add `update_sort_order` endpoint accepting `Vec<{id, sort_order}>` body
- Single transaction `UPDATE ... SET sort_order = CASE ... END WHERE id IN (...)`
- Push `StoreOp::UpdateProductSortOrder` to edge via WebSocket

### 3.2 Bulk Delete API

**Problem:** No `POST /api/tenant/stores/{id}/products/bulk-delete` API. Console only supports one-by-one deletion.

**Solution:**
- Accept `{ ids: Vec<i64> }` body
- Transaction: delete all, push individual `StoreOp::DeleteProduct` per item
- Return count of deleted items

### 3.3 Ghost Printer Config Cleanup

**Problem:** `usePrinterStore.kitchenPrinter` (localStorage key `printer_kitchen`) appears as toggle in HardwareSettings but has no effect on actual kitchen printing (which uses `print_destinations` from server).

**Solution:**
- Remove `kitchenPrinter` field from `usePrinterStore`
- Remove the toggle from HardwareSettings
- Show `KitchenPrinterList` directly (already exists)

---

## Verification

After all phases:
- `cargo clippy --workspace -- -D warnings` zero warnings
- `cd red_coral && npx tsc --noEmit` zero errors
- `cargo test --workspace --lib` all pass
