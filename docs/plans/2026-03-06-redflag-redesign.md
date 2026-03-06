# RedFlags Redesign Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Redesign RedFlags from summary-only counts into grouped summaries + traceable event log with order numbers, across edge-server, crab-cloud, red_coral POS, and crab-console.

**Architecture:** Three-group flag structure (item/order/payment) with two APIs: summary endpoint (grouped counts + operator breakdown) and event log endpoint (paginated, filterable, with receipt_number tracing). No backward compatibility — full rewrite of all layers.

**Tech Stack:** Rust (Axum, SQLite, PostgreSQL), React 19 + TypeScript, Tailwind CSS, Zustand

---

### Task 1: Edge Server — Rewrite RedFlags Summary API

**Files:**
- Modify: `edge-server/src/api/statistics/handler.rs:651-793` (replace old RedFlags structs + handler)

**Step 1: Replace RedFlags response structs**

Delete old `RedFlagsSummary`, `OperatorRedFlags`, `RedFlagsResponse` structs (lines 653-677) and `get_red_flags` handler (lines 679-793). Replace with:

```rust
// ============================================================================
// Red Flags — Grouped Summary
// ============================================================================

const RED_FLAG_EVENT_TYPES: &str = "'ITEM_REMOVED','ITEM_COMPED','ITEM_UNCOMPED','ITEM_MODIFIED','ORDER_VOIDED','ORDER_DISCOUNT_APPLIED','ORDER_SURCHARGE_APPLIED','RULE_SKIP_TOGGLED','PAYMENT_CANCELLED'";

#[derive(Debug, Serialize)]
pub struct ItemFlags {
    pub removals: i64,
    pub comps: i64,
    pub uncomps: i64,
    pub price_modifications: i64,
}

#[derive(Debug, Serialize)]
pub struct OrderFlags {
    pub voids: i64,
    pub discounts: i64,
    pub surcharges: i64,
    pub rule_skips: i64,
}

#[derive(Debug, Serialize)]
pub struct PaymentFlags {
    pub cancellations: i64,
    pub refund_count: i64,
    pub refund_amount: f64,
}

#[derive(Debug, Serialize)]
pub struct OperatorRedFlags {
    pub operator_id: i64,
    pub operator_name: String,
    pub removals: i64,
    pub comps: i64,
    pub uncomps: i64,
    pub price_modifications: i64,
    pub voids: i64,
    pub discounts: i64,
    pub surcharges: i64,
    pub rule_skips: i64,
    pub cancellations: i64,
    pub refund_count: i64,
    pub refund_amount: f64,
    pub total_flags: i64,
}

#[derive(Debug, Serialize)]
pub struct RedFlagsResponse {
    pub item_flags: ItemFlags,
    pub order_flags: OrderFlags,
    pub payment_flags: PaymentFlags,
    pub operator_breakdown: Vec<OperatorRedFlags>,
}
```

**Step 2: Implement the new `get_red_flags` handler**

```rust
/// GET /api/statistics/red-flags
pub async fn get_red_flags(
    State(state): State<ServerState>,
    Query(query): Query<StatisticsQuery>,
) -> AppResult<Json<RedFlagsResponse>> {
    let cutoff = store_info::get(&state.pool)
        .await
        .ok()
        .flatten()
        .map(|s| s.business_day_cutoff)
        .unwrap_or(0);

    let (start, end) = if let (Some(from), Some(to)) = (query.from, query.to) {
        (from, to)
    } else {
        let time_range = query.time_range.as_deref().unwrap_or("today");
        calculate_time_range(
            time_range, cutoff,
            query.start_date.as_deref(), query.end_date.as_deref(),
            state.config.timezone,
        )
    };

    // 1. Event summary by type
    let summary_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT ae.event_type, COUNT(*) as cnt
         FROM archived_order_event ae
         JOIN archived_order o ON ae.order_pk = o.id
         WHERE o.end_time >= ?1 AND o.end_time < ?2
           AND ae.event_type IN ('ITEM_REMOVED','ITEM_COMPED','ITEM_UNCOMPED','ITEM_MODIFIED','ORDER_VOIDED','ORDER_DISCOUNT_APPLIED','ORDER_SURCHARGE_APPLIED','RULE_SKIP_TOGGLED','PAYMENT_CANCELLED')
         GROUP BY ae.event_type"
    )
    .bind(start).bind(end)
    .fetch_all(&state.pool).await
    .map_err(|e| AppError::database(e.to_string()))?;

    let mut item_flags = ItemFlags { removals: 0, comps: 0, uncomps: 0, price_modifications: 0 };
    let mut order_flags = OrderFlags { voids: 0, discounts: 0, surcharges: 0, rule_skips: 0 };
    let mut payment_flags = PaymentFlags { cancellations: 0, refund_count: 0, refund_amount: 0.0 };

    for (event_type, count) in &summary_rows {
        match event_type.as_str() {
            "ITEM_REMOVED" => item_flags.removals = *count,
            "ITEM_COMPED" => item_flags.comps = *count,
            "ITEM_UNCOMPED" => item_flags.uncomps = *count,
            "ITEM_MODIFIED" => item_flags.price_modifications = *count,
            "ORDER_VOIDED" => order_flags.voids = *count,
            "ORDER_DISCOUNT_APPLIED" => order_flags.discounts = *count,
            "ORDER_SURCHARGE_APPLIED" => order_flags.surcharges = *count,
            "RULE_SKIP_TOGGLED" => order_flags.rule_skips = *count,
            "PAYMENT_CANCELLED" => payment_flags.cancellations = *count,
            _ => {}
        }
    }

    // 2. Refund summary from credit_note
    let refund_rows: Vec<(i64, String, i64, f64)> = sqlx::query_as(
        "SELECT COALESCE(operator_id, 0), COALESCE(operator_name, ''),
                COUNT(*), COALESCE(SUM(total_credit), 0.0)
         FROM credit_note
         WHERE created_at >= ?1 AND created_at < ?2
         GROUP BY operator_id, operator_name"
    )
    .bind(start).bind(end)
    .fetch_all(&state.pool).await
    .map_err(|e| AppError::database(e.to_string()))?;

    for (_, _, count, amount) in &refund_rows {
        payment_flags.refund_count += count;
        payment_flags.refund_amount += amount;
    }

    // 3. Operator breakdown from events
    let operator_rows: Vec<(i64, String, String, i64)> = sqlx::query_as(
        "SELECT COALESCE(ae.operator_id, 0), COALESCE(ae.operator_name, ''),
                ae.event_type, COUNT(*) as cnt
         FROM archived_order_event ae
         JOIN archived_order o ON ae.order_pk = o.id
         WHERE o.end_time >= ?1 AND o.end_time < ?2
           AND ae.event_type IN ('ITEM_REMOVED','ITEM_COMPED','ITEM_UNCOMPED','ITEM_MODIFIED','ORDER_VOIDED','ORDER_DISCOUNT_APPLIED','ORDER_SURCHARGE_APPLIED','RULE_SKIP_TOGGLED','PAYMENT_CANCELLED')
         GROUP BY ae.operator_id, ae.operator_name, ae.event_type"
    )
    .bind(start).bind(end)
    .fetch_all(&state.pool).await
    .map_err(|e| AppError::database(e.to_string()))?;

    let mut op_map: std::collections::HashMap<i64, OperatorRedFlags> =
        std::collections::HashMap::new();

    for (op_id, op_name, event_type, count) in operator_rows {
        let entry = op_map.entry(op_id).or_insert_with(|| OperatorRedFlags {
            operator_id: op_id, operator_name: op_name.clone(),
            removals: 0, comps: 0, uncomps: 0, price_modifications: 0,
            voids: 0, discounts: 0, surcharges: 0, rule_skips: 0,
            cancellations: 0, refund_count: 0, refund_amount: 0.0, total_flags: 0,
        });
        match event_type.as_str() {
            "ITEM_REMOVED" => entry.removals = count,
            "ITEM_COMPED" => entry.comps = count,
            "ITEM_UNCOMPED" => entry.uncomps = count,
            "ITEM_MODIFIED" => entry.price_modifications = count,
            "ORDER_VOIDED" => entry.voids = count,
            "ORDER_DISCOUNT_APPLIED" => entry.discounts = count,
            "ORDER_SURCHARGE_APPLIED" => entry.surcharges = count,
            "RULE_SKIP_TOGGLED" => entry.rule_skips = count,
            "PAYMENT_CANCELLED" => entry.cancellations = count,
            _ => {}
        }
    }

    // Merge refund data into operator map
    for (op_id, op_name, count, amount) in refund_rows {
        let entry = op_map.entry(op_id).or_insert_with(|| OperatorRedFlags {
            operator_id: op_id, operator_name: op_name.clone(),
            removals: 0, comps: 0, uncomps: 0, price_modifications: 0,
            voids: 0, discounts: 0, surcharges: 0, rule_skips: 0,
            cancellations: 0, refund_count: 0, refund_amount: 0.0, total_flags: 0,
        });
        entry.refund_count = count;
        entry.refund_amount = amount;
    }

    // Calculate total_flags and sort
    let mut operator_breakdown: Vec<OperatorRedFlags> = op_map.into_values().map(|mut op| {
        op.total_flags = op.removals + op.comps + op.uncomps + op.price_modifications
            + op.voids + op.discounts + op.surcharges + op.rule_skips
            + op.cancellations + op.refund_count;
        op
    }).collect();
    operator_breakdown.sort_by(|a, b| b.total_flags.cmp(&a.total_flags));

    Ok(Json(RedFlagsResponse { item_flags, order_flags, payment_flags, operator_breakdown }))
}
```

**Step 3: Verify compilation**

Run: `cargo check -p edge-server`
Expected: PASS (no errors)

**Step 4: Commit**

```bash
git add edge-server/src/api/statistics/handler.rs
git commit -m "refactor(edge): rewrite red-flags API with grouped structure"
```

---

### Task 2: Edge Server — Add Red Flags Event Log API

**Files:**
- Modify: `edge-server/src/api/statistics/handler.rs` (add new handler + query struct)
- Modify: `edge-server/src/api/statistics/mod.rs:19` (add route)

**Step 1: Add log query params and response structs**

After the `RedFlagsResponse` struct in `handler.rs`, add:

```rust
// ============================================================================
// Red Flags — Event Log
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct RedFlagLogQuery {
    pub from: i64,
    pub to: i64,
    pub event_type: Option<String>,
    pub operator_id: Option<i64>,
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(rename = "perPage", default = "default_per_page")]
    pub per_page: i32,
}

fn default_per_page() -> i32 { 50 }

#[derive(Debug, Serialize)]
pub struct RedFlagLogEntry {
    pub timestamp: i64,
    pub event_type: String,
    pub operator_id: i64,
    pub operator_name: String,
    pub receipt_number: String,
    pub order_id: i64,
    pub detail: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RedFlagLogResponse {
    pub entries: Vec<RedFlagLogEntry>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
}
```

**Step 2: Implement `get_red_flag_log` handler**

```rust
/// GET /api/statistics/red-flags/log
pub async fn get_red_flag_log(
    State(state): State<ServerState>,
    Query(query): Query<RedFlagLogQuery>,
) -> AppResult<Json<RedFlagLogResponse>> {
    let per_page = query.per_page.clamp(1, 100);
    let offset = (query.page.max(1) - 1) * per_page;

    // Build WHERE clause dynamically
    let event_filter = query.event_type.as_deref().unwrap_or("");
    let operator_filter = query.operator_id;

    // Combine events + refunds via UNION ALL, then paginate
    // For simplicity, use two queries: events + refunds, merge in Rust
    // This avoids complex UNION with SQLite type issues

    let mut entries: Vec<RedFlagLogEntry> = Vec::new();

    // 1. Order events
    let event_sql = if event_filter == "REFUND" {
        // Only refunds requested, skip events query
        String::new()
    } else {
        let mut sql = String::from(
            "SELECT ae.timestamp, ae.event_type,
                    COALESCE(ae.operator_id, 0), COALESCE(ae.operator_name, ''),
                    COALESCE(o.receipt_number, ''), o.order_id,
                    ae.data
             FROM archived_order_event ae
             JOIN archived_order o ON ae.order_pk = o.id
             WHERE o.end_time >= ?1 AND o.end_time < ?2
               AND ae.event_type IN ('ITEM_REMOVED','ITEM_COMPED','ITEM_UNCOMPED','ITEM_MODIFIED','ORDER_VOIDED','ORDER_DISCOUNT_APPLIED','ORDER_SURCHARGE_APPLIED','RULE_SKIP_TOGGLED','PAYMENT_CANCELLED')"
        );
        if !event_filter.is_empty() {
            sql.push_str(&format!(" AND ae.event_type = '{event_filter}'"));
        }
        if let Some(op_id) = operator_filter {
            sql.push_str(&format!(" AND ae.operator_id = {op_id}"));
        }
        sql
    };

    if !event_sql.is_empty() {
        let rows: Vec<(i64, String, i64, String, String, i64, Option<String>)> =
            sqlx::query_as(&event_sql)
                .bind(query.from).bind(query.to)
                .fetch_all(&state.pool).await
                .map_err(|e| AppError::database(e.to_string()))?;

        for (ts, etype, op_id, op_name, receipt, order_id, data) in rows {
            entries.push(RedFlagLogEntry {
                timestamp: ts, event_type: etype,
                operator_id: op_id, operator_name: op_name,
                receipt_number: receipt, order_id, detail: data,
            });
        }
    }

    // 2. Refunds (unless filtering by a specific non-REFUND event type)
    if event_filter.is_empty() || event_filter == "REFUND" {
        let mut refund_sql = String::from(
            "SELECT cn.created_at, cn.operator_id, cn.operator_name,
                    COALESCE(o.receipt_number, ''), o.order_id,
                    cn.total_credit, cn.reason
             FROM credit_note cn
             JOIN archived_order o ON cn.original_order_pk = o.id
             WHERE cn.created_at >= ?1 AND cn.created_at < ?2"
        );
        if let Some(op_id) = operator_filter {
            refund_sql.push_str(&format!(" AND cn.operator_id = {op_id}"));
        }

        let refund_rows: Vec<(i64, i64, String, String, i64, f64, String)> =
            sqlx::query_as(&refund_sql)
                .bind(query.from).bind(query.to)
                .fetch_all(&state.pool).await
                .map_err(|e| AppError::database(e.to_string()))?;

        for (ts, op_id, op_name, receipt, order_id, amount, reason) in refund_rows {
            entries.push(RedFlagLogEntry {
                timestamp: ts, event_type: "REFUND".to_string(),
                operator_id: op_id, operator_name: op_name,
                receipt_number: receipt, order_id,
                detail: Some(format!("{:.2} - {}", amount, reason)),
            });
        }
    }

    // Sort by timestamp DESC, then paginate
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    let total = entries.len() as i64;
    let paginated: Vec<RedFlagLogEntry> = entries
        .into_iter()
        .skip(offset as usize)
        .take(per_page as usize)
        .collect();

    Ok(Json(RedFlagLogResponse {
        entries: paginated,
        total,
        page: query.page.max(1),
        per_page,
    }))
}
```

**Step 3: Register the route in `mod.rs`**

In `edge-server/src/api/statistics/mod.rs`, add the new route:

```rust
fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::get_statistics))
        .route("/sales-report", get(handler::get_sales_report))
        .route("/red-flags", get(handler::get_red_flags))
        .route("/red-flags/log", get(handler::get_red_flag_log))
        .route("/invoices", get(handler::list_invoices))
        .layer(middleware::from_fn(require_permission("reports:view")))
}
```

**Step 4: Verify compilation**

Run: `cargo check -p edge-server`
Expected: PASS

**Step 5: Commit**

```bash
git add edge-server/src/api/statistics/handler.rs edge-server/src/api/statistics/mod.rs
git commit -m "feat(edge): add red-flags event log API with pagination and filters"
```

---

### Task 3: Tauri Bridge — Update RedFlags Types + Add Log Command

**Files:**
- Modify: `red_coral/src-tauri/src/commands/statistics.rs:218-262` (replace old types + add log command)
- Modify: `red_coral/src-tauri/src/lib.rs:338` (register new command)

**Step 1: Replace RedFlags types and update command**

In `statistics.rs`, replace the old Red Flags section (lines 218-262) with:

```rust
// ============================================================================
// Red Flags
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemFlags {
    pub removals: i64,
    pub comps: i64,
    pub uncomps: i64,
    pub price_modifications: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFlags {
    pub voids: i64,
    pub discounts: i64,
    pub surcharges: i64,
    pub rule_skips: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentFlags {
    pub cancellations: i64,
    pub refund_count: i64,
    pub refund_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorRedFlags {
    pub operator_id: i64,
    pub operator_name: String,
    pub removals: i64,
    pub comps: i64,
    pub uncomps: i64,
    pub price_modifications: i64,
    pub voids: i64,
    pub discounts: i64,
    pub surcharges: i64,
    pub rule_skips: i64,
    pub cancellations: i64,
    pub refund_count: i64,
    pub refund_amount: f64,
    pub total_flags: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedFlagsResponse {
    pub item_flags: ItemFlags,
    pub order_flags: OrderFlags,
    pub payment_flags: PaymentFlags,
    pub operator_breakdown: Vec<OperatorRedFlags>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedFlagLogEntry {
    pub timestamp: i64,
    pub event_type: String,
    pub operator_id: i64,
    pub operator_name: String,
    pub receipt_number: String,
    pub order_id: i64,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedFlagLogResponse {
    pub entries: Vec<RedFlagLogEntry>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
}

#[tauri::command]
pub async fn get_red_flags(
    bridge: State<'_, Arc<ClientBridge>>,
    from: i64,
    to: i64,
) -> Result<ApiResponse<RedFlagsResponse>, String> {
    let path = format!("/api/statistics/red-flags?from={}&to={}", from, to);
    match bridge.get::<RedFlagsResponse>(&path).await {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => {
            warn!(from, to, error = %e, "get_red_flags failed");
            Ok(ApiResponse::from_bridge_error(e))
        }
    }
}

#[tauri::command]
pub async fn get_red_flag_log(
    bridge: State<'_, Arc<ClientBridge>>,
    from: i64,
    to: i64,
    eventType: Option<String>,
    operatorId: Option<i64>,
    page: Option<i32>,
) -> Result<ApiResponse<RedFlagLogResponse>, String> {
    let mut path = format!("/api/statistics/red-flags/log?from={}&to={}", from, to);
    if let Some(et) = eventType {
        path.push_str(&format!("&event_type={}", et));
    }
    if let Some(op) = operatorId {
        path.push_str(&format!("&operator_id={}", op));
    }
    if let Some(p) = page {
        path.push_str(&format!("&page={}", p));
    }
    match bridge.get::<RedFlagLogResponse>(&path).await {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => {
            warn!(from, to, error = %e, "get_red_flag_log failed");
            Ok(ApiResponse::from_bridge_error(e))
        }
    }
}
```

**Step 2: Register command in `lib.rs`**

In `red_coral/src-tauri/src/lib.rs`, line 338, add `get_red_flag_log` next to `get_red_flags`:

```rust
            commands::get_red_flags,
            commands::get_red_flag_log,
```

**Step 3: Verify compilation**

Run: `cd red_coral && cargo check -p red-coral`
Expected: PASS

**Step 4: Commit**

```bash
git add red_coral/src-tauri/src/commands/statistics.rs red_coral/src-tauri/src/lib.rs
git commit -m "feat(tauri): update red-flags types + add event log command"
```

---

### Task 4: red_coral Frontend — Remove Old RedFlags from OverviewTab

**Files:**
- Modify: `red_coral/src/screens/Statistics/components/OverviewTab.tsx` (remove RedFlagsBar import + state + rendering)
- Delete: `red_coral/src/screens/Statistics/components/RedFlagsBar.tsx`
- Modify: `red_coral/src/screens/Statistics/components/Overview.tsx` (remove RedFlagsBar import + prop)

**Step 1: Clean OverviewTab**

In `OverviewTab.tsx`:
- Remove line 7: `import { RedFlagsBar, type RedFlagsData } from './RedFlagsBar';`
- Remove line 32: `const [redFlags, setRedFlags] = useState<RedFlagsData | null>(null);`
- Remove line 51: the `invokeApi<RedFlagsData>('get_red_flags', ...)` call from Promise.all (adjust destructuring from `[current, prevResult, lwResult, flags]` to `[current, prevResult, lwResult]`)
- Remove line 57: `setRedFlags(flags);`
- Remove line 72: `{redFlags && <RedFlagsBar data={redFlags} />}`

**Step 2: Clean Overview.tsx**

In `Overview.tsx`:
- Remove line 3: `import { RedFlagsBar, type RedFlagsData } from './RedFlagsBar';`
- Remove line 11: `redFlags?: RedFlagsData | null;` from interface
- Remove line 23: `{redFlags && <RedFlagsBar data={redFlags} />}`

**Step 3: Delete RedFlagsBar.tsx**

Delete `red_coral/src/screens/Statistics/components/RedFlagsBar.tsx`

**Step 4: Verify compilation**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

**Step 5: Commit**

```bash
git add -u red_coral/src/screens/Statistics/components/
git commit -m "refactor(pos): remove old RedFlagsBar from OverviewTab"
```

---

### Task 5: red_coral Frontend — Add RedFlags Tab + Sidebar Entry

**Files:**
- Modify: `red_coral/src/core/domain/types/index.ts:96` (update ActiveTab)
- Modify: `red_coral/src/screens/Statistics/components/Sidebar.tsx` (add menu item)
- Modify: `red_coral/src/screens/Statistics/index.tsx` (add tab rendering)
- Create: `red_coral/src/screens/Statistics/components/RedFlagsTab.tsx`

**Step 1: Update ActiveTab type**

In `red_coral/src/core/domain/types/index.ts`, line 96:
```ts
export type ActiveTab = 'overview' | 'invoices' | 'reports_shifts' | 'red_flags' | 'audit_log';
```

**Step 2: Add Sidebar entry**

In `Sidebar.tsx`, add `AlertTriangle` to lucide imports (line 2) and add to `menuItems` array (line 26, before audit_log):
```tsx
import { ArrowLeft, Activity, TrendingUp, FileText, ClipboardList, ShieldCheck, AlertTriangle } from 'lucide-react';

// in menuItems:
    { id: 'overview' as const, icon: TrendingUp, label: t('statistics.sidebar.overview') },
    { id: 'invoices' as const, icon: FileText, label: t('statistics.sidebar.invoices') },
    { id: 'reports_shifts' as const, icon: ClipboardList, label: t('statistics.sidebar.reports_shifts') },
    { id: 'red_flags' as const, icon: AlertTriangle, label: t('statistics.sidebar.red_flags') },
    { id: 'audit_log' as const, icon: ShieldCheck, label: t('statistics.sidebar.audit_log') },
```

**Step 3: Add tab rendering in StatisticsScreen**

In `index.tsx`, add import and rendering:
```tsx
import { RedFlagsTab } from './components/RedFlagsTab';

// In the h1 block, add:
{activeTab === 'red_flags' && t('statistics.sidebar.red_flags')}

// In the rendering block, add:
{activeTab === 'red_flags' && <RedFlagsTab />}
```

**Step 4: Create RedFlagsTab component**

Create `red_coral/src/screens/Statistics/components/RedFlagsTab.tsx`:

```tsx
import React, { useState, useEffect, useCallback } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { logger } from '@/utils/logger';
import { toast } from '@/presentation/components/Toast';
import { formatCurrency } from '@/utils/currency/formatCurrency';
import { TimeRangeSelector, useTimeRange } from './TimeRangeSelector';
import { Package, ClipboardList, CreditCard, ChevronDown, ChevronUp, Filter } from 'lucide-react';

// ── Types ──

interface ItemFlags {
  removals: number;
  comps: number;
  uncomps: number;
  price_modifications: number;
}
interface OrderFlags {
  voids: number;
  discounts: number;
  surcharges: number;
  rule_skips: number;
}
interface PaymentFlags {
  cancellations: number;
  refund_count: number;
  refund_amount: number;
}
interface OperatorRedFlags {
  operator_id: number;
  operator_name: string;
  removals: number;
  comps: number;
  uncomps: number;
  price_modifications: number;
  voids: number;
  discounts: number;
  surcharges: number;
  rule_skips: number;
  cancellations: number;
  refund_count: number;
  refund_amount: number;
  total_flags: number;
}
interface RedFlagsResponse {
  item_flags: ItemFlags;
  order_flags: OrderFlags;
  payment_flags: PaymentFlags;
  operator_breakdown: OperatorRedFlags[];
}
interface LogEntry {
  timestamp: number;
  event_type: string;
  operator_id: number;
  operator_name: string;
  receipt_number: string;
  order_id: number;
  detail: string | null;
}
interface LogResponse {
  entries: LogEntry[];
  total: number;
  page: number;
  per_page: number;
}

// ── Event type labels + colors ──

const EVENT_TYPES = [
  'ITEM_REMOVED', 'ITEM_COMPED', 'ITEM_UNCOMPED', 'ITEM_MODIFIED',
  'ORDER_VOIDED', 'ORDER_DISCOUNT_APPLIED', 'ORDER_SURCHARGE_APPLIED', 'RULE_SKIP_TOGGLED',
  'PAYMENT_CANCELLED', 'REFUND',
] as const;

const EVENT_COLORS: Record<string, string> = {
  ITEM_REMOVED: 'bg-red-100 text-red-700',
  ITEM_COMPED: 'bg-emerald-100 text-emerald-700',
  ITEM_UNCOMPED: 'bg-teal-100 text-teal-700',
  ITEM_MODIFIED: 'bg-orange-100 text-orange-700',
  ORDER_VOIDED: 'bg-red-100 text-red-700',
  ORDER_DISCOUNT_APPLIED: 'bg-amber-100 text-amber-700',
  ORDER_SURCHARGE_APPLIED: 'bg-purple-100 text-purple-700',
  RULE_SKIP_TOGGLED: 'bg-sky-100 text-sky-700',
  PAYMENT_CANCELLED: 'bg-rose-100 text-rose-700',
  REFUND: 'bg-violet-100 text-violet-700',
};

// ── Main component ──

export const RedFlagsTab: React.FC = () => {
  const { t } = useI18n();
  const [range, setRange] = useTimeRange();
  const [summary, setSummary] = useState<RedFlagsResponse | null>(null);
  const [log, setLog] = useState<LogResponse | null>(null);
  const [logPage, setLogPage] = useState(1);
  const [eventFilter, setEventFilter] = useState('');
  const [operatorFilter, setOperatorFilter] = useState<number | ''>('');
  const [expandedOps, setExpandedOps] = useState(false);

  const loadSummary = useCallback(async () => {
    try {
      const data = await invokeApi<RedFlagsResponse>('get_red_flags', { from: range.from, to: range.to });
      setSummary(data);
    } catch (e) {
      logger.error('Failed to load red flags summary', e);
      toast.error(t('statistics.error.load'));
    }
  }, [range, t]);

  const loadLog = useCallback(async (page: number) => {
    try {
      const params: Record<string, unknown> = { from: range.from, to: range.to, page };
      if (eventFilter) params.eventType = eventFilter;
      if (operatorFilter !== '') params.operatorId = operatorFilter;
      const data = await invokeApi<LogResponse>('get_red_flag_log', params);
      setLog(data);
    } catch (e) {
      logger.error('Failed to load red flags log', e);
    }
  }, [range, eventFilter, operatorFilter]);

  useEffect(() => { loadSummary(); }, [loadSummary]);
  useEffect(() => { setLogPage(1); loadLog(1); }, [loadLog]);

  const handleLoadMore = () => {
    const nextPage = logPage + 1;
    setLogPage(nextPage);
    invokeApi<LogResponse>('get_red_flag_log', {
      from: range.from, to: range.to, page: nextPage,
      ...(eventFilter ? { eventType: eventFilter } : {}),
      ...(operatorFilter !== '' ? { operatorId: operatorFilter } : {}),
    }).then(data => {
      setLog(prev => prev ? { ...data, entries: [...prev.entries, ...data.entries] } : data);
    }).catch(() => {});
  };

  const et = (key: string) => t(`statistics.red_flags.events.${key}`);
  const formatTime = (ts: number) => new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

  const operators = summary?.operator_breakdown ?? [];

  return (
    <>
      <TimeRangeSelector value={range} onChange={setRange} />

      {/* ── Summary Cards ── */}
      {summary && (
        <div className="grid grid-cols-3 gap-4 mt-4">
          {/* Item Flags */}
          <div className="bg-white rounded-xl border border-orange-200 p-4">
            <h3 className="text-sm font-semibold text-orange-700 flex items-center gap-2 mb-3">
              <Package className="w-4 h-4" />{t('statistics.red_flags.group_items')}
            </h3>
            <div className="space-y-1.5 text-sm">
              {summary.item_flags.removals > 0 && <FlagRow label={et('ITEM_REMOVED')} count={summary.item_flags.removals} />}
              {summary.item_flags.comps > 0 && <FlagRow label={et('ITEM_COMPED')} count={summary.item_flags.comps} />}
              {summary.item_flags.uncomps > 0 && <FlagRow label={et('ITEM_UNCOMPED')} count={summary.item_flags.uncomps} />}
              {summary.item_flags.price_modifications > 0 && <FlagRow label={et('ITEM_MODIFIED')} count={summary.item_flags.price_modifications} />}
              {(summary.item_flags.removals + summary.item_flags.comps + summary.item_flags.uncomps + summary.item_flags.price_modifications) === 0 && (
                <p className="text-slate-400 text-xs">{t('statistics.red_flags.no_flags')}</p>
              )}
            </div>
          </div>

          {/* Order Flags */}
          <div className="bg-white rounded-xl border border-red-200 p-4">
            <h3 className="text-sm font-semibold text-red-700 flex items-center gap-2 mb-3">
              <ClipboardList className="w-4 h-4" />{t('statistics.red_flags.group_orders')}
            </h3>
            <div className="space-y-1.5 text-sm">
              {summary.order_flags.voids > 0 && <FlagRow label={et('ORDER_VOIDED')} count={summary.order_flags.voids} />}
              {summary.order_flags.discounts > 0 && <FlagRow label={et('ORDER_DISCOUNT_APPLIED')} count={summary.order_flags.discounts} />}
              {summary.order_flags.surcharges > 0 && <FlagRow label={et('ORDER_SURCHARGE_APPLIED')} count={summary.order_flags.surcharges} />}
              {summary.order_flags.rule_skips > 0 && <FlagRow label={et('RULE_SKIP_TOGGLED')} count={summary.order_flags.rule_skips} />}
              {(summary.order_flags.voids + summary.order_flags.discounts + summary.order_flags.surcharges + summary.order_flags.rule_skips) === 0 && (
                <p className="text-slate-400 text-xs">{t('statistics.red_flags.no_flags')}</p>
              )}
            </div>
          </div>

          {/* Payment Flags */}
          <div className="bg-white rounded-xl border border-purple-200 p-4">
            <h3 className="text-sm font-semibold text-purple-700 flex items-center gap-2 mb-3">
              <CreditCard className="w-4 h-4" />{t('statistics.red_flags.group_payments')}
            </h3>
            <div className="space-y-1.5 text-sm">
              {summary.payment_flags.cancellations > 0 && <FlagRow label={et('PAYMENT_CANCELLED')} count={summary.payment_flags.cancellations} />}
              {summary.payment_flags.refund_count > 0 && (
                <div className="flex justify-between">
                  <span className="text-slate-600">{et('REFUND')}</span>
                  <span className="font-semibold tabular-nums">
                    {summary.payment_flags.refund_count}
                    <span className="text-xs text-slate-400 ml-1">({formatCurrency(summary.payment_flags.refund_amount)})</span>
                  </span>
                </div>
              )}
              {(summary.payment_flags.cancellations + summary.payment_flags.refund_count) === 0 && (
                <p className="text-slate-400 text-xs">{t('statistics.red_flags.no_flags')}</p>
              )}
            </div>
          </div>
        </div>
      )}

      {/* ── Operator Breakdown (collapsible) ── */}
      {operators.length > 0 && (
        <div className="mt-4 bg-white rounded-xl border border-slate-200 overflow-hidden">
          <button
            onClick={() => setExpandedOps(!expandedOps)}
            className="w-full px-4 py-3 flex items-center justify-between text-sm font-semibold text-slate-700 hover:bg-slate-50"
          >
            {t('statistics.red_flags.operator_breakdown')} ({operators.length})
            {expandedOps ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
          </button>
          {expandedOps && (
            <div className="overflow-x-auto border-t border-slate-100">
              <table className="w-full text-xs">
                <thead>
                  <tr className="bg-slate-50 text-slate-500">
                    <th className="px-3 py-2 text-left font-medium">{t('statistics.red_flags.operator')}</th>
                    <th className="px-2 py-2 text-center font-medium">{et('ITEM_REMOVED')}</th>
                    <th className="px-2 py-2 text-center font-medium">{et('ITEM_COMPED')}</th>
                    <th className="px-2 py-2 text-center font-medium">{et('ITEM_UNCOMPED')}</th>
                    <th className="px-2 py-2 text-center font-medium">{et('ITEM_MODIFIED')}</th>
                    <th className="px-2 py-2 text-center font-medium">{et('ORDER_VOIDED')}</th>
                    <th className="px-2 py-2 text-center font-medium">{et('ORDER_DISCOUNT_APPLIED')}</th>
                    <th className="px-2 py-2 text-center font-medium">{et('ORDER_SURCHARGE_APPLIED')}</th>
                    <th className="px-2 py-2 text-center font-medium">{et('RULE_SKIP_TOGGLED')}</th>
                    <th className="px-2 py-2 text-center font-medium">{et('PAYMENT_CANCELLED')}</th>
                    <th className="px-2 py-2 text-center font-medium">{et('REFUND')}</th>
                    <th className="px-2 py-2 text-center font-medium">{t('statistics.red_flags.total')}</th>
                  </tr>
                </thead>
                <tbody>
                  {operators.map(op => (
                    <tr key={op.operator_id} className="border-t border-slate-50 hover:bg-slate-50">
                      <td className="px-3 py-2 font-medium text-slate-800">{op.operator_name || `#${op.operator_id}`}</td>
                      <td className="px-2 py-2 text-center tabular-nums">{op.removals || '-'}</td>
                      <td className="px-2 py-2 text-center tabular-nums">{op.comps || '-'}</td>
                      <td className="px-2 py-2 text-center tabular-nums">{op.uncomps || '-'}</td>
                      <td className="px-2 py-2 text-center tabular-nums">{op.price_modifications || '-'}</td>
                      <td className="px-2 py-2 text-center tabular-nums">{op.voids || '-'}</td>
                      <td className="px-2 py-2 text-center tabular-nums">{op.discounts || '-'}</td>
                      <td className="px-2 py-2 text-center tabular-nums">{op.surcharges || '-'}</td>
                      <td className="px-2 py-2 text-center tabular-nums">{op.rule_skips || '-'}</td>
                      <td className="px-2 py-2 text-center tabular-nums">{op.cancellations || '-'}</td>
                      <td className="px-2 py-2 text-center tabular-nums">{op.refund_count || '-'}</td>
                      <td className="px-2 py-2 text-center font-bold tabular-nums">{op.total_flags}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}

      {/* ── Event Log ── */}
      <div className="mt-4 bg-white rounded-xl border border-slate-200 overflow-hidden">
        <div className="px-4 py-3 border-b border-slate-100 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-slate-700">{t('statistics.red_flags.event_log')}</h3>
          <div className="flex gap-2">
            <select
              value={eventFilter}
              onChange={e => setEventFilter(e.target.value)}
              className="text-xs border border-slate-200 rounded-lg px-2 py-1.5"
            >
              <option value="">{t('statistics.red_flags.all_types')}</option>
              {EVENT_TYPES.map(type => (
                <option key={type} value={type}>{et(type)}</option>
              ))}
            </select>
            {operators.length > 0 && (
              <select
                value={operatorFilter}
                onChange={e => setOperatorFilter(e.target.value ? Number(e.target.value) : '')}
                className="text-xs border border-slate-200 rounded-lg px-2 py-1.5"
              >
                <option value="">{t('statistics.red_flags.all_operators')}</option>
                {operators.map(op => (
                  <option key={op.operator_id} value={op.operator_id}>{op.operator_name}</option>
                ))}
              </select>
            )}
          </div>
        </div>

        {log && log.entries.length > 0 ? (
          <div className="divide-y divide-slate-50">
            {log.entries.map((entry, i) => (
              <div key={i} className="px-4 py-2.5 flex items-center gap-3 text-sm hover:bg-slate-50">
                <span className="text-xs text-slate-400 tabular-nums w-12 shrink-0">{formatTime(entry.timestamp)}</span>
                <span className="text-slate-600 w-20 shrink-0 truncate">{entry.operator_name}</span>
                <span className={`text-xs px-2 py-0.5 rounded-full font-medium shrink-0 ${EVENT_COLORS[entry.event_type] ?? 'bg-slate-100 text-slate-600'}`}>
                  {et(entry.event_type)}
                </span>
                <span className="text-slate-800 font-mono text-xs truncate">{entry.receipt_number}</span>
                {entry.detail && <span className="text-slate-400 text-xs truncate ml-auto">{entry.detail}</span>}
              </div>
            ))}
            {log.entries.length < log.total && (
              <button
                onClick={handleLoadMore}
                className="w-full py-3 text-sm text-primary-600 font-medium hover:bg-primary-50 transition-colors"
              >
                {t('statistics.red_flags.load_more')} ({log.entries.length}/{log.total})
              </button>
            )}
          </div>
        ) : (
          <div className="px-4 py-8 text-center text-slate-400 text-sm">
            {t('statistics.red_flags.no_flags')}
          </div>
        )}
      </div>
    </>
  );
};

const FlagRow: React.FC<{ label: string; count: number }> = ({ label, count }) => (
  <div className="flex justify-between">
    <span className="text-slate-600">{label}</span>
    <span className="font-semibold tabular-nums">{count}</span>
  </div>
);
```

**Step 5: Verify compilation**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

**Step 6: Commit**

```bash
git add red_coral/src/core/domain/types/index.ts \
        red_coral/src/screens/Statistics/components/Sidebar.tsx \
        red_coral/src/screens/Statistics/index.tsx \
        red_coral/src/screens/Statistics/components/RedFlagsTab.tsx
git commit -m "feat(pos): add RedFlags tab with grouped summary + event log"
```

---

### Task 6: red_coral i18n — Add New Translation Keys

**Files:**
- Modify: `red_coral/src/infrastructure/i18n/locales/zh-CN.json` (red_flags section)
- Modify: `red_coral/src/infrastructure/i18n/locales/es-ES.json` (red_flags section)

**Step 1: Update zh-CN red_flags**

Replace the existing `red_flags` block (line 826-833) with:

```json
"red_flags": {
  "group_items": "商品操作",
  "group_orders": "订单操作",
  "group_payments": "支付/退款",
  "operator_breakdown": "操作员明细",
  "operator": "操作员",
  "total": "总计",
  "no_flags": "无异常记录",
  "event_log": "事件日志",
  "all_types": "全部类型",
  "all_operators": "全部操作员",
  "load_more": "加载更多",
  "events": {
    "ITEM_REMOVED": "删菜",
    "ITEM_COMPED": "赠送",
    "ITEM_UNCOMPED": "取消赠送",
    "ITEM_MODIFIED": "改价",
    "ORDER_VOIDED": "作废",
    "ORDER_DISCOUNT_APPLIED": "整单折扣",
    "ORDER_SURCHARGE_APPLIED": "整单附加费",
    "RULE_SKIP_TOGGLED": "跳过规则",
    "PAYMENT_CANCELLED": "取消支付",
    "REFUND": "退款"
  }
},
```

Also add sidebar key. Find `"statistics"` > `"sidebar"` section, add:
```json
"red_flags": "异常监控",
```

**Step 2: Update es-ES red_flags**

Replace the existing `red_flags` block with:

```json
"red_flags": {
  "group_items": "Operaciones de productos",
  "group_orders": "Operaciones de pedidos",
  "group_payments": "Pagos / Devoluciones",
  "operator_breakdown": "Detalle por operador",
  "operator": "Operador",
  "total": "Total",
  "no_flags": "Sin alertas",
  "event_log": "Registro de eventos",
  "all_types": "Todos los tipos",
  "all_operators": "Todos los operadores",
  "load_more": "Cargar mas",
  "events": {
    "ITEM_REMOVED": "Eliminado",
    "ITEM_COMPED": "Invitacion",
    "ITEM_UNCOMPED": "Des-invitar",
    "ITEM_MODIFIED": "Modificado",
    "ORDER_VOIDED": "Anulado",
    "ORDER_DISCOUNT_APPLIED": "Descuento",
    "ORDER_SURCHARGE_APPLIED": "Recargo",
    "RULE_SKIP_TOGGLED": "Regla omitida",
    "PAYMENT_CANCELLED": "Pago cancelado",
    "REFUND": "Devolucion"
  }
},
```

Also add sidebar key `"red_flags": "Alertas"` in the statistics sidebar section.

**Step 3: Verify compilation**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

**Step 4: Commit**

```bash
git add red_coral/src/infrastructure/i18n/locales/zh-CN.json \
        red_coral/src/infrastructure/i18n/locales/es-ES.json
git commit -m "feat(pos): add i18n keys for RedFlags tab"
```

---

### Task 7: crab-cloud — Rewrite RedFlags Summary API

**Files:**
- Modify: `crab-cloud/src/db/tenant_queries.rs:1616-1720` (replace structs + query)
- Modify: `crab-cloud/src/api/tenant/analytics.rs:118-141` (update handler)

**Step 1: Replace structs and query in `tenant_queries.rs`**

Delete old structs `RedFlagsSummary`, `OperatorRedFlags`, `RedFlagsResponse` and `get_red_flags()` function (lines 1616-1720+). Replace with new grouped structure matching edge-server:

```rust
// ── Red Flags 监控 (Grouped) ──

#[derive(Debug, serde::Serialize)]
pub struct ItemFlags {
    pub removals: i64,
    pub comps: i64,
    pub uncomps: i64,
    pub price_modifications: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct OrderFlags {
    pub voids: i64,
    pub discounts: i64,
    pub surcharges: i64,
    pub rule_skips: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct PaymentFlags {
    pub cancellations: i64,
    pub refund_count: i64,
    pub refund_amount: f64,
}

#[derive(Debug, serde::Serialize)]
pub struct OperatorRedFlags {
    pub operator_id: Option<i64>,
    pub operator_name: Option<String>,
    pub removals: i64,
    pub comps: i64,
    pub uncomps: i64,
    pub price_modifications: i64,
    pub voids: i64,
    pub discounts: i64,
    pub surcharges: i64,
    pub rule_skips: i64,
    pub cancellations: i64,
    pub refund_count: i64,
    pub refund_amount: f64,
    pub total_flags: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct RedFlagsResponse {
    pub item_flags: ItemFlags,
    pub order_flags: OrderFlags,
    pub payment_flags: PaymentFlags,
    pub operator_breakdown: Vec<OperatorRedFlags>,
}

pub async fn get_red_flags(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    from: i64,
    to: i64,
) -> Result<RedFlagsResponse, BoxError> {
    // Event counts per operator
    #[derive(sqlx::FromRow)]
    struct Row {
        operator_id: Option<i64>,
        operator_name: Option<String>,
        removals: i64,
        comps: i64,
        uncomps: i64,
        price_modifications: i64,
        voids: i64,
        discounts: i64,
        surcharges: i64,
        rule_skips: i64,
        cancellations: i64,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT
            e.operator_id,
            e.operator_name,
            COUNT(*) FILTER (WHERE e.event_type = 'ITEM_REMOVED') AS removals,
            COUNT(*) FILTER (WHERE e.event_type = 'ITEM_COMPED') AS comps,
            COUNT(*) FILTER (WHERE e.event_type = 'ITEM_UNCOMPED') AS uncomps,
            COUNT(*) FILTER (WHERE e.event_type = 'ITEM_MODIFIED') AS price_modifications,
            COUNT(*) FILTER (WHERE e.event_type = 'ORDER_VOIDED') AS voids,
            COUNT(*) FILTER (WHERE e.event_type = 'ORDER_DISCOUNT_APPLIED') AS discounts,
            COUNT(*) FILTER (WHERE e.event_type = 'ORDER_SURCHARGE_APPLIED') AS surcharges,
            COUNT(*) FILTER (WHERE e.event_type = 'RULE_SKIP_TOGGLED') AS rule_skips,
            COUNT(*) FILTER (WHERE e.event_type = 'PAYMENT_CANCELLED') AS cancellations
        FROM store_order_events e
        JOIN store_archived_orders o ON o.id = e.order_id
        WHERE o.store_id = $1 AND o.tenant_id = $2
            AND o.end_time >= $3 AND o.end_time < $4
            AND e.event_type IN ('ITEM_REMOVED','ITEM_COMPED','ITEM_UNCOMPED','ITEM_MODIFIED','ORDER_VOIDED','ORDER_DISCOUNT_APPLIED','ORDER_SURCHARGE_APPLIED','RULE_SKIP_TOGGLED','PAYMENT_CANCELLED')
        GROUP BY e.operator_id, e.operator_name
        ORDER BY COUNT(*) DESC
        "#,
    )
    .bind(store_id).bind(tenant_id).bind(from).bind(to)
    .fetch_all(pool).await?;

    // Refund counts per operator
    #[derive(sqlx::FromRow)]
    struct RefundRow {
        operator_name: Option<String>,
        cnt: i64,
        total: rust_decimal::Decimal,
    }

    let refund_rows: Vec<RefundRow> = sqlx::query_as(
        r#"
        SELECT cn.operator_name, COUNT(*) as cnt, COALESCE(SUM(cn.total_credit), 0) as total
        FROM store_credit_notes cn
        WHERE cn.store_id = $1 AND cn.tenant_id = $2
            AND cn.created_at >= $3 AND cn.created_at < $4
        GROUP BY cn.operator_name
        "#,
    )
    .bind(store_id).bind(tenant_id).bind(from).bind(to)
    .fetch_all(pool).await?;

    // Build summary
    let mut item_flags = ItemFlags { removals: 0, comps: 0, uncomps: 0, price_modifications: 0 };
    let mut order_flags = OrderFlags { voids: 0, discounts: 0, surcharges: 0, rule_skips: 0 };
    let mut payment_flags = PaymentFlags { cancellations: 0, refund_count: 0, refund_amount: 0.0 };

    // Use operator_name as key for cloud (no operator_id in credit_notes)
    let mut op_map: std::collections::HashMap<String, OperatorRedFlags> =
        std::collections::HashMap::new();

    for row in &rows {
        item_flags.removals += row.removals;
        item_flags.comps += row.comps;
        item_flags.uncomps += row.uncomps;
        item_flags.price_modifications += row.price_modifications;
        order_flags.voids += row.voids;
        order_flags.discounts += row.discounts;
        order_flags.surcharges += row.surcharges;
        order_flags.rule_skips += row.rule_skips;
        payment_flags.cancellations += row.cancellations;

        let key = row.operator_name.clone().unwrap_or_default();
        let entry = op_map.entry(key).or_insert_with(|| OperatorRedFlags {
            operator_id: row.operator_id, operator_name: row.operator_name.clone(),
            removals: 0, comps: 0, uncomps: 0, price_modifications: 0,
            voids: 0, discounts: 0, surcharges: 0, rule_skips: 0,
            cancellations: 0, refund_count: 0, refund_amount: 0.0, total_flags: 0,
        });
        entry.removals += row.removals;
        entry.comps += row.comps;
        entry.uncomps += row.uncomps;
        entry.price_modifications += row.price_modifications;
        entry.voids += row.voids;
        entry.discounts += row.discounts;
        entry.surcharges += row.surcharges;
        entry.rule_skips += row.rule_skips;
        entry.cancellations += row.cancellations;
    }

    for refund in &refund_rows {
        use rust_decimal::prelude::ToPrimitive;
        let amount = refund.total.to_f64().unwrap_or(0.0);
        payment_flags.refund_count += refund.cnt;
        payment_flags.refund_amount += amount;

        let key = refund.operator_name.clone().unwrap_or_default();
        let entry = op_map.entry(key.clone()).or_insert_with(|| OperatorRedFlags {
            operator_id: None, operator_name: refund.operator_name.clone(),
            removals: 0, comps: 0, uncomps: 0, price_modifications: 0,
            voids: 0, discounts: 0, surcharges: 0, rule_skips: 0,
            cancellations: 0, refund_count: 0, refund_amount: 0.0, total_flags: 0,
        });
        entry.refund_count += refund.cnt;
        entry.refund_amount += amount;
    }

    let mut operator_breakdown: Vec<OperatorRedFlags> = op_map.into_values().map(|mut op| {
        op.total_flags = op.removals + op.comps + op.uncomps + op.price_modifications
            + op.voids + op.discounts + op.surcharges + op.rule_skips
            + op.cancellations + op.refund_count;
        op
    }).collect();
    operator_breakdown.sort_by(|a, b| b.total_flags.cmp(&a.total_flags));

    Ok(RedFlagsResponse { item_flags, order_flags, payment_flags, operator_breakdown })
}
```

**Step 2: Update analytics handler** (should need minimal change since it already calls `tenant_queries::get_red_flags`)

Verify `crab-cloud/src/api/tenant/analytics.rs:118-141` still works — the return type `ApiResult<tenant_queries::RedFlagsResponse>` should resolve correctly.

**Step 3: Verify compilation**

Run: `cargo check -p crab-cloud`
Expected: PASS

**Step 4: Commit**

```bash
git add crab-cloud/src/db/tenant_queries.rs crab-cloud/src/api/tenant/analytics.rs
git commit -m "refactor(cloud): rewrite red-flags API with grouped structure + refund tracking"
```

---

### Task 8: crab-cloud — Add Red Flags Event Log API

**Files:**
- Modify: `crab-cloud/src/db/tenant_queries.rs` (add log query function)
- Modify: `crab-cloud/src/api/tenant/analytics.rs` (add handler)
- Modify: `crab-cloud/src/api/tenant/mod.rs` (add route)

**Step 1: Add log query in `tenant_queries.rs`**

After `get_red_flags()`, add:

```rust
#[derive(Debug, serde::Serialize)]
pub struct RedFlagLogEntry {
    pub timestamp: i64,
    pub event_type: String,
    pub operator_name: String,
    pub receipt_number: String,
    pub order_id: i64,
    pub detail: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct RedFlagLogResponse {
    pub entries: Vec<RedFlagLogEntry>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
}

pub async fn get_red_flag_log(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    from: i64,
    to: i64,
    event_type: Option<&str>,
    operator_name_filter: Option<&str>,
    page: i32,
    per_page: i32,
) -> Result<RedFlagLogResponse, BoxError> {
    let per_page = per_page.clamp(1, 100);
    let offset = (page.max(1) - 1) * per_page;

    let mut entries: Vec<RedFlagLogEntry> = Vec::new();

    // Events
    if event_type.map_or(true, |t| t != "REFUND") {
        #[derive(sqlx::FromRow)]
        struct EventRow {
            timestamp: i64,
            event_type: String,
            operator_name: Option<String>,
            receipt_number: Option<String>,
            order_id: i64,
            data: Option<String>,
        }

        let et_filter = match event_type {
            Some(t) if t != "REFUND" => format!("AND e.event_type = '{t}'"),
            _ => String::new(),
        };
        let op_filter = match operator_name_filter {
            Some(n) => format!("AND e.operator_name = '{}'", n.replace('\'', "''")),
            None => String::new(),
        };

        let sql = format!(
            r#"SELECT e.timestamp, e.event_type,
                      e.operator_name, o.receipt_number, o.id as order_id, e.data
               FROM store_order_events e
               JOIN store_archived_orders o ON o.id = e.order_id
               WHERE o.store_id = $1 AND o.tenant_id = $2
                 AND o.end_time >= $3 AND o.end_time < $4
                 AND e.event_type IN ('ITEM_REMOVED','ITEM_COMPED','ITEM_UNCOMPED','ITEM_MODIFIED','ORDER_VOIDED','ORDER_DISCOUNT_APPLIED','ORDER_SURCHARGE_APPLIED','RULE_SKIP_TOGGLED','PAYMENT_CANCELLED')
                 {et_filter} {op_filter}"#
        );

        let rows: Vec<EventRow> = sqlx::query_as(&sql)
            .bind(store_id).bind(tenant_id).bind(from).bind(to)
            .fetch_all(pool).await?;

        for r in rows {
            entries.push(RedFlagLogEntry {
                timestamp: r.timestamp,
                event_type: r.event_type,
                operator_name: r.operator_name.unwrap_or_default(),
                receipt_number: r.receipt_number.unwrap_or_default(),
                order_id: r.order_id,
                detail: r.data,
            });
        }
    }

    // Refunds
    if event_type.map_or(true, |t| t == "REFUND") {
        #[derive(sqlx::FromRow)]
        struct RefRow {
            created_at: i64,
            operator_name: String,
            receipt_number: String,
            order_id: i64,
            total_credit: rust_decimal::Decimal,
            reason: String,
        }

        let op_filter = match operator_name_filter {
            Some(n) => format!("AND cn.operator_name = '{}'", n.replace('\'', "''")),
            None => String::new(),
        };

        let sql = format!(
            r#"SELECT cn.created_at, cn.operator_name, cn.original_receipt as receipt_number,
                      cn.original_order_id as order_id, cn.total_credit, cn.reason
               FROM store_credit_notes cn
               WHERE cn.store_id = $1 AND cn.tenant_id = $2
                 AND cn.created_at >= $3 AND cn.created_at < $4
                 {op_filter}"#
        );

        let rows: Vec<RefRow> = sqlx::query_as(&sql)
            .bind(store_id).bind(tenant_id).bind(from).bind(to)
            .fetch_all(pool).await?;

        for r in rows {
            use rust_decimal::prelude::ToPrimitive;
            entries.push(RedFlagLogEntry {
                timestamp: r.created_at,
                event_type: "REFUND".to_string(),
                operator_name: r.operator_name,
                receipt_number: r.receipt_number,
                order_id: r.order_id,
                detail: Some(format!("{:.2} - {}", r.total_credit.to_f64().unwrap_or(0.0), r.reason)),
            });
        }
    }

    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    let total = entries.len() as i64;
    let paginated: Vec<RedFlagLogEntry> = entries.into_iter()
        .skip(offset as usize).take(per_page as usize).collect();

    Ok(RedFlagLogResponse { entries: paginated, total, page: page.max(1), per_page })
}
```

**Step 2: Add handler in `analytics.rs`**

```rust
#[derive(Debug, Deserialize)]
pub struct RedFlagLogQuery {
    pub from: i64,
    pub to: i64,
    pub event_type: Option<String>,
    pub operator_name: Option<String>,
    #[serde(default = "default_page")]
    pub page: Option<i32>,
    #[serde(rename = "perPage")]
    pub per_page: Option<i32>,
}

fn default_page() -> Option<i32> { Some(1) }

/// GET /api/tenant/stores/:id/red-flags/log
pub async fn get_store_red_flag_log(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Query(query): Query<RedFlagLogQuery>,
) -> ApiResult<tenant_queries::RedFlagLogResponse> {
    validate_range(query.from, query.to)?;
    verify_store(&state, store_id, identity.tenant_id).await?;

    let log = tenant_queries::get_red_flag_log(
        &state.pool, store_id, identity.tenant_id,
        query.from, query.to,
        query.event_type.as_deref(),
        query.operator_name.as_deref(),
        query.page.unwrap_or(1),
        query.per_page.unwrap_or(50),
    ).await.map_err(|e| {
        tracing::error!("Red flag log query error: {e}");
        AppError::new(ErrorCode::InternalError)
    })?;

    Ok(Json(log))
}
```

**Step 3: Add route in `tenant/mod.rs`**

Find the red-flags route and add the log route next to it:

```rust
.route("/stores/{id}/red-flags", get(analytics::get_store_red_flags))
.route("/stores/{id}/red-flags/log", get(analytics::get_store_red_flag_log))
```

**Step 4: Verify compilation**

Run: `cargo check -p crab-cloud`
Expected: PASS

**Step 5: Commit**

```bash
git add crab-cloud/src/db/tenant_queries.rs \
        crab-cloud/src/api/tenant/analytics.rs \
        crab-cloud/src/api/tenant/mod.rs
git commit -m "feat(cloud): add red-flags event log API with pagination"
```

---

### Task 9: crab-console — Rewrite RedFlagsScreen

**Files:**
- Modify: `crab-console/src/core/types/stats.ts:152-174` (replace types)
- Modify: `crab-console/src/infrastructure/api/stats.ts:33-40` (add log API)
- Modify: `crab-console/src/screens/Store/RedFlags/RedFlagsScreen.tsx` (full rewrite)

**Step 1: Update TypeScript types in `stats.ts`**

Replace old `RedFlagsSummary`, `OperatorRedFlags`, `RedFlagsResponse` with:

```ts
export interface ItemFlags {
  removals: number;
  comps: number;
  uncomps: number;
  price_modifications: number;
}
export interface OrderFlags {
  voids: number;
  discounts: number;
  surcharges: number;
  rule_skips: number;
}
export interface PaymentFlags {
  cancellations: number;
  refund_count: number;
  refund_amount: number;
}
export interface OperatorRedFlags {
  operator_id: number | null;
  operator_name: string | null;
  removals: number;
  comps: number;
  uncomps: number;
  price_modifications: number;
  voids: number;
  discounts: number;
  surcharges: number;
  rule_skips: number;
  cancellations: number;
  refund_count: number;
  refund_amount: number;
  total_flags: number;
}
export interface RedFlagsResponse {
  item_flags: ItemFlags;
  order_flags: OrderFlags;
  payment_flags: PaymentFlags;
  operator_breakdown: OperatorRedFlags[];
}
export interface RedFlagLogEntry {
  timestamp: number;
  event_type: string;
  operator_name: string;
  receipt_number: string;
  order_id: number;
  detail: string | null;
}
export interface RedFlagLogResponse {
  entries: RedFlagLogEntry[];
  total: number;
  page: number;
  per_page: number;
}
```

**Step 2: Add log API function in `stats.ts`**

```ts
export function getStoreRedFlagLog(
  token: string,
  storeId: number,
  from: number,
  to: number,
  eventType?: string,
  operatorName?: string,
  page?: number,
): Promise<RedFlagLogResponse> {
  let path = `/api/tenant/stores/${storeId}/red-flags/log?from=${from}&to=${to}`;
  if (eventType) path += `&event_type=${eventType}`;
  if (operatorName) path += `&operator_name=${encodeURIComponent(operatorName)}`;
  if (page) path += `&page=${page}`;
  return request('GET', path, undefined, token);
}
```

**Step 3: Rewrite RedFlagsScreen.tsx**

Full rewrite with same grouped summary + event log layout as red_coral's `RedFlagsTab`. Use the same structure but adapted for crab-console (REST API calls instead of Tauri invoke, date picker instead of TimeRangeSelector). Mirror the component structure, event type labels, colors, and table from Task 5's `RedFlagsTab.tsx`.

Key differences from red_coral:
- Uses `getStoreRedFlags()` and `getStoreRedFlagLog()` instead of `invokeApi`
- Uses `useStoreId()` + `useAuthStore` for auth context
- Date picker input (already exists) instead of TimeRangeSelector
- Uses console's i18n keys at `red_flags.*` (not `statistics.red_flags.*`)

**Step 4: Verify compilation**

Run: `cd crab-console && npx tsc --noEmit`
Expected: PASS

**Step 5: Commit**

```bash
git add crab-console/src/core/types/stats.ts \
        crab-console/src/infrastructure/api/stats.ts \
        crab-console/src/screens/Store/RedFlags/RedFlagsScreen.tsx
git commit -m "feat(console): rewrite RedFlagsScreen with grouped summary + event log"
```

---

### Task 10: crab-console i18n — Update Translation Keys

**Files:**
- Modify: `crab-console/src/infrastructure/i18n/locales/zh.json` (red_flags section)
- Modify: `crab-console/src/infrastructure/i18n/locales/en.json` (red_flags section)
- Modify: `crab-console/src/infrastructure/i18n/locales/es.json` (red_flags section)

**Step 1: Update all three locales**

Replace the `red_flags` block in each locale. Example for zh:

```json
"red_flags": {
  "title": "异常监控",
  "group_items": "商品操作",
  "group_orders": "订单操作",
  "group_payments": "支付/退款",
  "operator_breakdown": "操作员明细",
  "operator": "员工",
  "total": "异常总数",
  "no_data": "该时段无异常记录",
  "unknown_operator": "系统",
  "event_log": "事件日志",
  "all_types": "全部类型",
  "all_operators": "全部操作员",
  "load_more": "加载更多",
  "compliance": "数据受 VeriFactu 法规保护，SHA-256 哈希链可验证。",
  "events": {
    "ITEM_REMOVED": "删菜",
    "ITEM_COMPED": "赠送",
    "ITEM_UNCOMPED": "取消赠送",
    "ITEM_MODIFIED": "改价",
    "ORDER_VOIDED": "作废",
    "ORDER_DISCOUNT_APPLIED": "整单折扣",
    "ORDER_SURCHARGE_APPLIED": "整单附加费",
    "RULE_SKIP_TOGGLED": "跳过规则",
    "PAYMENT_CANCELLED": "取消支付",
    "REFUND": "退款"
  }
},
```

Similarly for `en` and `es` (translate the new keys).

**Step 2: Commit**

```bash
git add crab-console/src/infrastructure/i18n/locales/
git commit -m "feat(console): update i18n keys for RedFlags redesign"
```

---

### Task 11: Final Verification + Compile Check

**Step 1: Full workspace Rust check**

Run: `cargo check --workspace`
Expected: PASS

**Step 2: Full TypeScript checks**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

Run: `cd crab-console && npx tsc --noEmit`
Expected: PASS

**Step 3: Clippy**

Run: `cargo clippy --workspace`
Expected: No warnings

**Step 4: Final commit (if any fixups needed)**

```bash
git add -u
git commit -m "chore: fixups from RedFlags redesign verification"
```
