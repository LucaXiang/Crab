# Order Layer Recovery After Database Loss

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ensure rebind after database loss restores all counters (receipt number, invoice number, huella chain) and inserts a BREAK chain entry, while separating invoice layer from system_state.

**Architecture:** Cloud returns `RecoveryState` alongside `CatalogSyncData`. Edge restores counters in both redb (receipt daily_count) and SQLite (invoice_counter, system_state), then inserts a BREAK chain_entry. Invoice layer (`last_huella`) is separated from `system_state` into `invoice_counter`.

**Tech Stack:** Rust, SQLite (sqlx), redb, PostgreSQL, WebSocket (shared CloudMessage protocol)

---

## Problem

When Edge's local SQLite + redb are deleted and the server re-binds:

1. **Receipt number collision** (critical): redb `daily_count` resets → `01-20260306-0001` duplicated
2. **Missing chain break marker**: New hash chain from `prev_hash="genesis"` indistinguishable from first activation
3. **Invoice number collision**: `invoice_counter` resets → Verifactu invoice numbers duplicated
4. **Huella chain break**: `last_huella` lost, but cloud has complete data — chain should NOT break

**Safety guarantee:** Edge MUST have catalog to serve orders → catalog only comes from cloud → recovery happens at CatalogSyncData time → no offline gap.

## Key Discovery: Counter Locations

| Counter | Storage | Used By | Reset |
|---------|---------|---------|-------|
| `daily_count` | **redb** `SEQUENCE_TABLE["daily_count"]` | `next_chain_number()` → receipt_number `01-YYYYMMDD-NNNN` | Daily |
| `system_state.order_count` | SQLite | `generate_next_receipt_number()` — **DEAD CODE** | Never |
| `invoice_counter.last_number` | SQLite | `next_invoice_number()` → `SERIE-YYYYMMDD-NNNN` | Per serie/date |
| `system_state.last_huella` | SQLite | `InvoiceService` huella chain | Never |
| `system_state.last_chain_hash` | SQLite | Archive service hash chain | Never |

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Detection | Edge catalog empty → requests CatalogSyncData → cloud always includes recovery_state | Simple; no new message type |
| BREAK entry | Edge inserts locally | Offline-first; syncs to cloud normally |
| Transport | Piggyback on CatalogSyncData response | Reuses existing flow |
| Order chain | Do NOT restore `last_chain_hash`; new genesis + BREAK | Honestly reflects data loss |
| Invoice huella | Restore `last_huella`; chain continues unbroken | Verifactu compliance; cloud has complete data |
| Historical data | Do NOT pull back orders/invoices | Edge lightweight; Console is authority |
| Dead code | Clean up `system_state.order_count` + `generate_next_receipt_number()` | Avoid confusion |

## RecoveryState

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecoveryState {
    /// Max daily receipt count for current business day (restore to redb daily_count)
    /// Parsed from receipt_number pattern: "NN-YYYYMMDD-CCCC" → CCCC
    pub daily_receipt_count: i64,
    /// Business date string "YYYYMMDD" for the daily_count
    pub business_date: String,
    /// Last chain_entry curr_hash (for BREAK entry prev_hash)
    pub last_chain_hash: Option<String>,
    /// Last invoice huella (restore to invoice_counter.last_huella)
    pub last_huella: Option<String>,
    /// Last invoice number e.g. "A-20260306-0003" (restore invoice_counter)
    pub last_invoice_number: Option<String>,
}
```

## Data Flow

```
Edge (DB + redb lost, re-binds)
  │ catalog empty → RequestCatalogSync
  ▼
Cloud
  │ Always builds recovery_state for the store:
  │   - Parse max daily receipt count from store_archived_orders.receipt_number
  │   - Last store_chain_entries.curr_hash
  │   - Last store_invoices.huella + invoice_number
  │ Returns: CatalogSyncData { catalog, recovery_state }
  ▼
Edge (apply_catalog_sync_data)
  │ 1. Apply catalog (existing logic)
  │ 2. If recovery_state has data (daily_receipt_count > 0 or last_chain_hash.is_some()):
  │    a. Restore redb daily_count + daily_date
  │    b. Restore invoice_counter (last_huella + last_number)
  │    c. Insert BREAK chain_entry:
  │       entry_type="BREAK", prev_hash=last_chain_hash, curr_hash="recovery"
  │    d. last_chain_hash stays NULL → next order prev_hash="genesis"
  ▼
Normal operation
```

## Changes Required

### Task 1: Separate invoice layer — move last_huella from system_state to invoice_counter

**Files:**
- Modify: `edge-server/migrations/0001_initial.up.sql` — add `last_huella` column to `invoice_counter`
- Modify: `shared/src/models/system_state.rs` — remove `last_huella` field
- Modify: `edge-server/src/db/repository/system_state.rs` — remove last_huella from queries
- Modify: `edge-server/src/db/repository/invoice.rs` — add last_huella read/write on invoice_counter
- Modify: `edge-server/src/archiving/invoice.rs` — use invoice_counter for huella instead of system_state

### Task 2: Clean up dead code — remove unused order_count receipt generation

**Files:**
- Modify: `edge-server/src/archiving/service.rs` — remove `generate_next_receipt_number()`
- Modify: `edge-server/src/db/repository/system_state.rs` — remove `get_next_order_number()` if truly unused

### Task 3: Add RecoveryState to shared protocol

**Files:**
- Modify: `shared/src/cloud/ws.rs` — add `recovery_state` to CatalogSyncData variant, add RecoveryState struct

### Task 4: Cloud builds recovery_state

**Files:**
- Modify: `crab-cloud/src/api/store/data_transfer.rs` — `build_catalog_export()` returns recovery data
- Modify: `crab-cloud/src/api/ws.rs` — include recovery_state in CatalogSyncData response
- May need new query functions in `crab-cloud/src/db/sync_store.rs`

### Task 5: Edge applies recovery_state

**Files:**
- Modify: `edge-server/src/cloud/worker.rs` — after CatalogSyncData, apply recovery
- Modify: `edge-server/src/cloud/ops/provisioning.rs` — extend apply_catalog_sync_data
- Modify: `edge-server/src/orders/storage.rs` — add method to set daily_count in redb
- Modify: `edge-server/src/db/repository/invoice.rs` — add method to restore invoice_counter

### Task 6: Verify and test

- `cargo check --workspace`
- `cargo clippy --workspace`
- Manual test: create orders → delete DB → rebind → verify counters restored
