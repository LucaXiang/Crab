# Business Day Cutoff Migration + RedCoral Stats Alignment Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Migrate `business_day_cutoff` from `String` ("HH:MM") to `i32` (minutes from midnight, 0-360), and align RedCoral POS statistics with Console's feature set.

**Architecture:** Three-phase approach: (1) Shared type + backend migration, (2) Console frontend adaptation, (3) RedCoral POS stats alignment. Frontend handles all cutoff-aware time range calculation and hourly trend ordering; backend APIs remain cutoff-unaware (just filter by from/to timestamps).

**Tech Stack:** Rust (shared, edge-server, crab-cloud), TypeScript/React (crab-console, red_coral), SQLite (edge), PostgreSQL (cloud), sqlx migrations.

---

## Task 1: Shared Type — `business_day_cutoff` String → i32

**Files:**
- Modify: `shared/src/models/store_info.rs`
- Modify: `shared/src/cloud/sync.rs` (line 842)

**Step 1: Update StoreInfo model**

```rust
// shared/src/models/store_info.rs

//! Store Info Model

use serde::{Deserialize, Serialize};

/// Store information entity (singleton per tenant)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct StoreInfo {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub address: String,
    /// Tax identification number (NIF)
    #[serde(default)]
    pub nif: String,
    pub logo_url: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    /// 营业日分界时间 — 从午夜 00:00 起的偏移分钟数 (0-360，即 00:00-06:00)
    #[serde(default)]
    pub business_day_cutoff: i32,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

/// Update store info payload
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoreInfoUpdate {
    pub name: Option<String>,
    pub address: Option<String>,
    pub nif: Option<String>,
    pub logo_url: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub business_day_cutoff: Option<i32>,
}
```

**Step 2: Update cloud sync type**

In `shared/src/cloud/sync.rs`, find `business_day_cutoff: Option<String>` and change to `Option<i32>`.

**Step 3: Fix all compilation errors across workspace**

Run: `cargo check --workspace 2>&1 | head -60`

Every file referencing `business_day_cutoff` as `String` will break. Fix each one (see Tasks 2-4 for specifics).

**Step 4: Commit**

```bash
git add shared/src/models/store_info.rs shared/src/cloud/sync.rs
git commit -m "refactor(shared): change business_day_cutoff from String to i32 minutes"
```

---

## Task 2: Edge Server — Migration + Adapt All References

**Files:**
- Create: `edge-server/migrations/NNNN_cutoff_to_int.up.sql`
- Create: `edge-server/migrations/NNNN_cutoff_to_int.down.sql`
- Modify: `edge-server/src/utils/time.rs` — `parse_cutoff()` and `date_cutoff_millis()`
- Modify: `edge-server/src/api/store_info/handler.rs` — validation
- Modify: `edge-server/src/api/statistics/handler.rs` — cutoff usage
- Modify: All other files referencing `business_day_cutoff`

**Step 1: Create SQLite migration**

Edge migration (figure out next number with `ls edge-server/migrations/`):

```sql
-- up.sql: Convert business_day_cutoff from TEXT "HH:MM" to INTEGER minutes
-- SQLite doesn't support ALTER COLUMN, so we add a new column and copy
ALTER TABLE store_info ADD COLUMN business_day_cutoff_new INTEGER NOT NULL DEFAULT 0;

UPDATE store_info SET business_day_cutoff_new =
    CAST(SUBSTR(business_day_cutoff, 1, INSTR(business_day_cutoff, ':') - 1) AS INTEGER) * 60 +
    CAST(SUBSTR(business_day_cutoff, INSTR(business_day_cutoff, ':') + 1) AS INTEGER);

-- SQLite can't drop columns in older versions, so we keep the old column
-- but the code will only use the new one
```

**Important**: SQLite doesn't support DROP COLUMN well. The cleanest approach is a table rebuild:

```sql
-- up.sql
CREATE TABLE store_info_new (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL DEFAULT '',
    address TEXT NOT NULL DEFAULT '',
    nif TEXT NOT NULL DEFAULT '',
    logo_url TEXT,
    phone TEXT,
    email TEXT,
    website TEXT,
    business_day_cutoff INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER,
    updated_at INTEGER
);

INSERT INTO store_info_new (id, name, address, nif, logo_url, phone, email, website, business_day_cutoff, created_at, updated_at)
SELECT id, name, address, nif, logo_url, phone, email, website,
    CAST(SUBSTR(business_day_cutoff, 1, INSTR(business_day_cutoff, ':') - 1) AS INTEGER) * 60 +
    CAST(SUBSTR(business_day_cutoff, INSTR(business_day_cutoff, ':') + 1) AS INTEGER),
    created_at, updated_at
FROM store_info;

DROP TABLE store_info;
ALTER TABLE store_info_new RENAME TO store_info;
```

```sql
-- down.sql
CREATE TABLE store_info_old (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL DEFAULT '',
    address TEXT NOT NULL DEFAULT '',
    nif TEXT NOT NULL DEFAULT '',
    logo_url TEXT,
    phone TEXT,
    email TEXT,
    website TEXT,
    business_day_cutoff TEXT NOT NULL DEFAULT '00:00',
    created_at INTEGER,
    updated_at INTEGER
);

INSERT INTO store_info_old SELECT id, name, address, nif, logo_url, phone, email, website,
    PRINTF('%02d:%02d', business_day_cutoff / 60, business_day_cutoff % 60),
    created_at, updated_at
FROM store_info;

DROP TABLE store_info;
ALTER TABLE store_info_old RENAME TO store_info;
```

**Step 2: Update `edge-server/src/utils/time.rs`**

Replace `parse_cutoff(&str) -> NaiveTime`:

```rust
/// 从分钟偏移量构造 NaiveTime (e.g. 210 → 03:30)
pub fn cutoff_to_time(minutes: i32) -> NaiveTime {
    let clamped = minutes.clamp(0, 360);
    let h = (clamped / 60) as u32;
    let m = (clamped % 60) as u32;
    NaiveTime::from_hms_opt(h, m, 0).unwrap_or(NaiveTime::MIN)
}
```

Update all callers:
- `calculate_time_range()` — change `cutoff: &str` param to `cutoff: i32`, call `cutoff_to_time(cutoff)` instead of `parse_cutoff(cutoff)`

**Step 3: Update store_info handler validation**

In `edge-server/src/api/store_info/handler.rs`, replace the text validation with:

```rust
fn validate_update(payload: &StoreInfoUpdate) -> AppResult<()> {
    // ... existing validations ...
    if let Some(cutoff) = payload.business_day_cutoff {
        if !(0..=360).contains(&cutoff) {
            return Err(AppError::validation(
                "business_day_cutoff must be between 0 and 360 minutes (00:00-06:00)".to_string(),
            ));
        }
    }
    Ok(())
}
```

**Step 4: Update statistics handler**

In `edge-server/src/api/statistics/handler.rs` (line ~214):

```rust
// Before:
let cutoff = store_info::get(&state.pool)
    .await.ok().flatten()
    .map(|s| s.business_day_cutoff)
    .unwrap_or_else(|| "02:00".to_string());

// After:
let cutoff = store_info::get(&state.pool)
    .await.ok().flatten()
    .map(|s| s.business_day_cutoff)
    .unwrap_or(0);
```

**Step 5: Fix ALL other edge-server references**

Search: `grep -rn "business_day_cutoff" edge-server/src/`

Known files:
- `edge-server/src/archiving/service.rs`
- `edge-server/src/orders/manager/mod.rs`
- `edge-server/src/cloud/ops/resource.rs`
- `edge-server/src/shifts.rs`
- `edge-server/src/core/state.rs`
- `edge-server/src/api/shifts/handler.rs`
- `edge-server/src/archiving/verify.rs`
- `edge-server/src/db/repository/store_info.rs`

Each of these currently treats cutoff as `String`. Change to `i32` and use `cutoff_to_time()` where NaiveTime is needed.

**Step 6: Verify compilation**

Run: `cargo check --workspace`
Expected: Zero errors

**Step 7: Commit**

```bash
git add edge-server/
git commit -m "refactor(edge): migrate business_day_cutoff to i32 minutes"
```

---

## Task 3: Cloud (PostgreSQL) — Migration + Adapt

**Files:**
- Create: `crab-cloud/migrations/NNNN_cutoff_to_int.up.sql`
- Create: `crab-cloud/migrations/NNNN_cutoff_to_int.down.sql`
- Modify: `crab-cloud/src/db/store/store_info.rs`
- Modify: All other cloud files referencing `business_day_cutoff`

**Step 1: Create PG migration**

```sql
-- up.sql
ALTER TABLE stores
    ALTER COLUMN business_day_cutoff TYPE INTEGER
    USING (
        CAST(SPLIT_PART(business_day_cutoff, ':', 1) AS INTEGER) * 60 +
        CAST(SPLIT_PART(business_day_cutoff, ':', 2) AS INTEGER)
    );

ALTER TABLE stores ALTER COLUMN business_day_cutoff SET DEFAULT 0;
```

```sql
-- down.sql
ALTER TABLE stores
    ALTER COLUMN business_day_cutoff TYPE TEXT
    USING LPAD((business_day_cutoff / 60)::TEXT, 2, '0') || ':' || LPAD((business_day_cutoff % 60)::TEXT, 2, '0');

ALTER TABLE stores ALTER COLUMN business_day_cutoff SET DEFAULT '00:00';
```

**Step 2: Update `crab-cloud/src/db/store/store_info.rs`**

The `StoreInfo` struct now has `business_day_cutoff: i32`, so sqlx will read it as integer from PG automatically. Check all `.bind(&info.business_day_cutoff)` calls — they should work since `i32` binds to `INTEGER`.

**Step 3: Add validation in cloud API**

In `crab-cloud/src/api/store/store_info.rs`, add validation for Console-side updates:

```rust
if let Some(cutoff) = &data.business_day_cutoff {
    if !(*cutoff >= 0 && *cutoff <= 360) {
        return Err(AppError::validation("business_day_cutoff must be 0-360"));
    }
}
```

**Step 4: Verify compilation**

Run: `cargo check --workspace`
Expected: Zero errors

**Step 5: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: Zero warnings

**Step 6: Commit**

```bash
git add crab-cloud/ shared/
git commit -m "refactor(cloud): migrate business_day_cutoff to i32 minutes"
```

---

## Task 4: Console Frontend — Adapt to i32 cutoff

**Files:**
- Modify: `crab-console/src/shared/components/TimeRangeSelector/TimeRangeSelector.tsx`
- Modify: `crab-console/src/screens/Store/Overview/StoreOverviewScreen.tsx`
- Modify: `crab-console/src/screens/Store/Overview/StoreOverviewDisplay.tsx`
- Modify: `crab-console/src/screens/Dashboard/DashboardScreen.tsx`
- Modify: `crab-console/src/core/types/store.ts` (if `business_day_cutoff` type needs updating)

**Step 1: Update TimeRangeSelector — rename `cutoffHour` → `cutoffMinutes`**

```typescript
interface Props {
  value: TimeRange;
  onChange: (range: TimeRange) => void;
  /** Business day cutoff in minutes from midnight (0-360). Defaults to 0. */
  cutoffMinutes?: number;
}

function startOfBusinessDay(date: Date, cutoffMinutes: number): Date {
  const h = Math.floor(cutoffMinutes / 60);
  const m = cutoffMinutes % 60;
  const d = new Date(date.getFullYear(), date.getMonth(), date.getDate(), h, m, 0, 0);
  if (date < d) d.setDate(d.getDate() - 1);
  return d;
}
```

Update `getPresetRange` signature:
```typescript
export function getPresetRange(
  preset: TimeRangePreset,
  t: (key: string) => string,
  customFrom?: number,
  customTo?: number,
  cutoffMinutes = 0,
): TimeRange {
  const now = new Date();
  const sod = cutoffMinutes > 0
    ? (d: Date) => startOfBusinessDay(d, cutoffMinutes)
    : (d: Date) => startOfDay(d);

  const cutoffH = Math.floor(cutoffMinutes / 60);
  const cutoffM = cutoffMinutes % 60;

  switch (preset) {
    case 'today':
      return { from: sod(now).getTime(), to: endOfNow(), preset, label: t('stats.today') };
    case 'yesterday': {
      const todayStart = sod(now);
      const yesterdayStart = new Date(todayStart);
      yesterdayStart.setDate(yesterdayStart.getDate() - 1);
      return { from: yesterdayStart.getTime(), to: todayStart.getTime(), preset, label: t('stats.yesterday') };
    }
    case 'this_week': {
      const weekStart = startOfWeek(now);
      weekStart.setHours(cutoffH, cutoffM, 0, 0);
      return { from: weekStart.getTime(), to: endOfNow(), preset, label: t('stats.this_week') };
    }
    case 'this_month': {
      const monthStart = startOfMonth(now);
      monthStart.setHours(cutoffH, cutoffM, 0, 0);
      return { from: monthStart.getTime(), to: endOfNow(), preset, label: t('stats.this_month') };
    }
    case 'custom':
      return {
        from: customFrom ?? sod(now).getTime(),
        to: customTo ?? endOfNow(),
        preset,
        label: t('stats.custom_range'),
      };
  }
}
```

Update component:
```typescript
export const TimeRangeSelector: React.FC<Props> = ({ value, onChange, cutoffMinutes = 0 }) => {
  // ...
  const cutoffH = Math.floor(cutoffMinutes / 60);
  const cutoffM = cutoffMinutes % 60;

  const handlePreset = (preset: TimeRangePreset) => {
    if (preset === 'custom') { setShowCustom(!showCustom); return; }
    setShowCustom(false);
    onChange(getPresetRange(preset, t, undefined, undefined, cutoffMinutes));
  };

  const handleCustomApply = () => {
    const from = new Date(customFrom);
    from.setHours(cutoffH, cutoffM, 0, 0);
    const toDate = new Date(customTo);
    toDate.setDate(toDate.getDate() + 1);
    toDate.setHours(cutoffH, cutoffM, 0, 0);
    onChange(getPresetRange('custom', t, from.getTime(), toDate.getTime(), cutoffMinutes));
    setShowCustom(false);
  };
  // ...
};
```

**Step 2: Update StoreOverviewScreen**

Replace cutoff parsing logic:
```typescript
// Before:
const raw = info.business_day_cutoff;
const parsed = raw ? parseInt(raw.split(':')[0], 10) || 0 : 0;
const hour = Math.min(Math.max(parsed, 0), 6);

// After (API now returns i32 directly):
const cutoff = Math.min(Math.max(info.business_day_cutoff ?? 0, 0), 360);
```

State: `const [cutoffMinutes, setCutoffMinutes] = useState(0);`

Pass to components: `<TimeRangeSelector cutoffMinutes={cutoffMinutes} />` and `<StoreOverviewDisplay cutoffMinutes={cutoffMinutes} />`

**Step 3: Update DashboardScreen**

Same pattern — replace string parsing with direct i32 usage:
```typescript
const cutoffs = storeList
  .map(s => Math.min(Math.max(s.business_day_cutoff ?? 0, 0), 360))
  .filter(h => h > 0);
effectiveCutoff = cutoffs.length > 0 ? Math.max(...cutoffs) : 0;
```

**Step 4: Update StoreOverviewDisplay**

Rename prop `cutoffHour` → `cutoffMinutes`. Update hourly trend sorting:

```typescript
// Business-day hour ordering
const cutoffHour = Math.floor(cutoffMinutes / 60);
const toBizOrder = (h: number) => (h - cutoffHour + 24) % 24;
```

**Step 5: Update store type definitions if needed**

In `crab-console/src/core/types/store.ts`, ensure `business_day_cutoff` is typed as `number` (not `string`).

**Step 6: TypeScript check**

Run: `cd crab-console && npx tsc --noEmit`
Expected: Zero errors

**Step 7: Commit**

```bash
git add crab-console/
git commit -m "refactor(console): adapt to i32 cutoffMinutes"
```

---

## Task 5: RedCoral POS — Statistics Alignment with Console

**Goal:** Make RedCoral's statistics overview match Console's `StoreOverviewDisplay`.

**Files:**
- Modify: `edge-server/src/api/statistics/handler.rs` — expand response to match cloud's `StoreOverview`
- Modify: `red_coral/src/core/domain/types/index.ts` — update `StatisticsResponse` type
- Modify: `red_coral/src/screens/Statistics/components/Overview.tsx` — new layout
- Modify: `red_coral/src/screens/Statistics/components/StatsCards.tsx` — align KPI cards
- Modify: `red_coral/src/screens/Statistics/components/RevenueTrendChart.tsx` — add comparison lines
- Modify: `red_coral/src/screens/Statistics/index.tsx` — add cutoff-aware TimeRangeSelector

### Step 1: Expand Edge Statistics API Response

The current edge API returns a limited `StatisticsResponse`. Expand it to match the cloud's `StoreOverview` format.

Add these queries to `edge-server/src/api/statistics/handler.rs`:

**New fields needed:**
- `guests` (already have as `customers`)
- `per_guest_spend` (already computed as `avg_guest_spend`)
- `total_tax` — add to overview query
- `total_surcharge` — add query
- `avg_items_per_order` — add query
- `payment_breakdown` — already have cash/card, expand to array format
- `tax_breakdown` — add from `archived_order_desglose`
- `refund_method_breakdown` — add from `credit_note`
- `daily_trend` — already have for non-today ranges, include always
- `service_type_breakdown` — add query
- `zone_sales` — add query
- `tag_sales` — add from `archived_order_item`

**New response struct** (in `edge-server/src/api/statistics/handler.rs`):

```rust
#[derive(Debug, Serialize)]
pub struct StoreOverviewResponse {
    pub revenue: f64,
    pub orders: i32,
    pub guests: i32,
    pub average_order_value: f64,
    pub per_guest_spend: f64,
    pub average_dining_minutes: f64,
    pub total_tax: f64,
    pub total_discount: f64,
    pub total_surcharge: f64,
    pub avg_items_per_order: f64,
    pub voided_orders: i32,
    pub voided_amount: f64,
    pub loss_orders: i32,
    pub loss_amount: f64,
    pub refund_count: i32,
    pub refund_amount: f64,
    pub revenue_trend: Vec<RevenueTrendPointNew>,
    pub daily_trend: Vec<DailyTrendPoint>,
    pub payment_breakdown: Vec<PaymentBreakdownEntry>,
    pub tax_breakdown: Vec<TaxBreakdownEntry>,
    pub category_sales: Vec<CategorySale>,
    pub top_products: Vec<TopProductNew>,
    pub tag_sales: Vec<TagSaleEntry>,
    pub refund_method_breakdown: Vec<RefundMethodEntry>,
    pub service_type_breakdown: Vec<ServiceTypeEntry>,
    pub zone_sales: Vec<ZoneSaleEntry>,
}

#[derive(Debug, Serialize)]
pub struct RevenueTrendPointNew {
    pub hour: i32,
    pub revenue: f64,
    pub orders: i64,
}

#[derive(Debug, Serialize)]
pub struct DailyTrendPoint {
    pub date: String,
    pub revenue: f64,
    pub orders: i64,
}

// ... (PaymentBreakdownEntry, TaxBreakdownEntry, etc. — match cloud structs)
```

Add the missing SQL queries (modeled after cloud's `tenant_queries.rs`):

- **total_tax + total_surcharge**: Add to main overview query
- **payment_breakdown**: `SELECT method, SUM(amount), COUNT(*) FROM archived_order_payment JOIN archived_order ...`
- **tax_breakdown**: `SELECT tax_rate, SUM(base_amount), SUM(tax_amount) FROM archived_order_desglose JOIN archived_order ...`
- **refund_method_breakdown**: `SELECT refund_method, SUM(total_credit), COUNT(*) FROM credit_note ...`
- **service_type_breakdown**: `SELECT service_type, SUM(total_amount), COUNT(*) FROM archived_order WHERE status='COMPLETED' ...`
- **zone_sales**: `SELECT zone_name, SUM(total_amount), COUNT(*), SUM(guest_count) FROM archived_order WHERE status='COMPLETED' ...`
- **tag_sales**: needs join through `archived_order_item` → product tags (may need schema check)
- **avg_items_per_order**: `SELECT AVG(item_count) FROM (SELECT COUNT(*) as item_count FROM archived_order_item GROUP BY order_pk)`
- **revenue_trend**: Change format from `{ time: "04:00", value: 10.5 }` to `{ hour: 4, revenue: 10.5, orders: 3 }` to match cloud

**Keep backward compatibility**: Create a new endpoint `GET /api/statistics/overview` with the expanded response, keep old `GET /api/statistics` for backward compat (or just change it if only RedCoral uses it).

### Step 2: Update RedCoral TypeScript Types

Update `red_coral/src/core/domain/types/index.ts` to match the new response:

```typescript
export interface StoreOverview {
  revenue: number;
  orders: number;
  guests: number;
  average_order_value: number;
  per_guest_spend: number;
  average_dining_minutes: number;
  total_tax: number;
  total_discount: number;
  total_surcharge: number;
  avg_items_per_order: number;
  voided_orders: number;
  voided_amount: number;
  loss_orders: number;
  loss_amount: number;
  refund_count: number;
  refund_amount: number;
  revenue_trend: { hour: number; revenue: number; orders: number }[];
  daily_trend: { date: string; revenue: number; orders: number }[];
  payment_breakdown: { method: string; amount: number; count: number }[];
  tax_breakdown: { tax_rate: number; base_amount: number; tax_amount: number }[];
  category_sales: { name: string; revenue: number }[];
  top_products: { name: string; quantity: number; revenue: number }[];
  tag_sales: { name: string; color: string | null; revenue: number; quantity: number }[];
  refund_method_breakdown: { method: string; amount: number; count: number }[];
  service_type_breakdown: { service_type: string; revenue: number; orders: number }[];
  zone_sales: { zone_name: string; revenue: number; orders: number; guests: number }[];
}
```

### Step 3: Add Comparison Data Fetching to RedCoral

Add previous period + last week comparison (like Console does):

In RedCoral's statistics screen, add:
- Fetch 3 time ranges in parallel: current, previous period, last week same day
- Pass all three to the overview display component
- Show delta indicators on KPI cards

### Step 4: Port StoreOverviewDisplay to RedCoral

The simplest approach: **copy Console's `StoreOverviewDisplay.tsx` component** (with minor adaptations for Tauri environment). This includes:
- All 16 KPI cards with delta indicators
- Hourly revenue trend with 3 comparison lines
- Daily trend chart
- Category sales pie chart
- Top products bar chart
- Tax breakdown
- Payment breakdown
- Service type distribution
- Zone sales
- Refund method breakdown

### Step 5: Add cutoff-aware TimeRangeSelector to RedCoral

Port Console's `TimeRangeSelector` component to RedCoral, or create a similar one that:
- Reads `business_day_cutoff` from store info (via Zustand `useStoreInfoStore`)
- Applies cutoff to all time range calculations
- Passes `cutoffMinutes` to the overview display

### Step 6: TypeScript check

Run: `cd red_coral && npx tsc --noEmit`
Expected: Zero errors

### Step 7: Commit

```bash
git add edge-server/src/api/statistics/ red_coral/src/
git commit -m "feat(redcoral): align statistics with Console, add comparison data"
```

---

## Task 6: RedCoral POS — Cutoff Settings UI Adaptation

**Files:**
- Modify: `red_coral/src/screens/Settings/StoreSettings.tsx`

**Step 1: Change WheelTimePicker to minute-based input**

Replace the `WheelTimePicker` with a simpler input that shows HH:MM but stores minutes:

```typescript
// Parse minutes to display "HH:MM"
const formatCutoff = (minutes: number) => {
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}`;
};

// Quick presets
const CUTOFF_PRESETS = [
  { label: '00:00', value: 0 },
  { label: '02:00', value: 120 },
  { label: '03:00', value: 180 },
  { label: '04:00', value: 240 },
  { label: '05:00', value: 300 },
  { label: '06:00', value: 360 },
];
```

**Step 2: Validate range 0-360**

Clamp value on change: `Math.min(Math.max(value, 0), 360)`

**Step 3: TypeScript check + Commit**

---

## Task 7: Full Stack Verification + Deploy

**Step 1: Rust compilation + clippy**

```bash
cargo clippy --workspace -- -D warnings
```

**Step 2: TypeScript checks**

```bash
cd red_coral && npx tsc --noEmit
cd crab-console && npx tsc --noEmit
```

**Step 3: Run edge-server tests**

```bash
cargo test --workspace --lib
```

**Step 4: Build + Deploy dev**

```bash
# Cloud
./deploy/build-cloud.sh push
# SSH to EC2, pull + restart dev-cloud

# Console
cd crab-console && npx vite build --mode development
scp -i deploy/ec2/crab-ec2.pem -r build/* ec2-user@51.92.72.162:/opt/crab/dev-console/
```

**Step 5: Verify**

- Check dev-cloud health: `curl https://dev-cloud.redcoral.app/health`
- Check dev-console loads
- Verify store overview shows correct cutoff behavior
- Test creating/updating cutoff from Console settings

**Step 6: Commit all remaining changes**

```bash
git add -A
git commit -m "feat: migrate business_day_cutoff to i32, align RedCoral stats with Console"
```

---

## Dependency Graph

```
Task 1 (shared types)
  ├─→ Task 2 (edge-server)
  ├─→ Task 3 (cloud)
  └─→ Task 4 (console frontend)
       └─→ Task 5 (RedCoral stats)  ← can start after Task 2 (edge API changes)
            └─→ Task 6 (RedCoral settings)
                 └─→ Task 7 (verify + deploy)
```

Tasks 2, 3, 4 can run in parallel after Task 1.
Task 5 depends on Task 2 (expanded edge API).
