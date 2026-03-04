# Statistics Alignment Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 重写 RedCoral 统计页，对齐 Console 数据计算逻辑，统一日期范围预设，补齐 Red Flags / 发票明细 / 班次功能。

**Architecture:** Edge-server 新增 Red Flags + Invoice List API 端点；RedCoral 统计页重写为 4-tab 结构（概览 | 发票明细 | 日报&班次 | 审计日志）；两端统一日期范围预设集（today/yesterday/this_week/this_month/last_month/custom）。

**Tech Stack:** Rust (edge-server Axum), TypeScript/React (RedCoral Tauri), SQLite, PostgreSQL (crab-cloud)

---

## Task 1: Fix Tauri StoreOverview Missing Fields

**Files:**
- Modify: `red_coral/src-tauri/src/commands/statistics.rs:17-44`

**Step 1: Add missing fields to Tauri StoreOverview struct**

在 `StoreOverview` struct 中添加 `net_revenue`、`anulacion_count`、`anulacion_amount`，与 edge-server 的 `StoreOverview` 完全对齐。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreOverview {
    pub revenue: f64,
    pub net_revenue: f64,           // ← 新增
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
    pub anulacion_count: i32,       // ← 新增
    pub anulacion_amount: f64,      // ← 新增
    pub refund_count: i32,
    pub refund_amount: f64,
    pub revenue_trend: Vec<RevenueTrendPoint>,
    pub daily_trend: Vec<DailyTrendPoint>,
    pub payment_breakdown: Vec<PaymentBreakdownEntry>,
    pub tax_breakdown: Vec<TaxBreakdownEntry>,
    pub category_sales: Vec<CategorySaleEntry>,
    pub top_products: Vec<TopProductEntry>,
    pub tag_sales: Vec<TagSaleEntry>,
    pub refund_method_breakdown: Vec<RefundMethodEntry>,
    pub service_type_breakdown: Vec<ServiceTypeEntry>,
    pub zone_sales: Vec<ZoneSaleEntry>,
}
```

**Step 2: Verify compilation**

Run: `cargo check -p red_coral-app` (或 `cargo check --workspace`)
Expected: PASS — TypeScript 侧 `EMPTY_OVERVIEW` 已包含这些字段。

**Step 3: Commit**

```bash
git add red_coral/src-tauri/src/commands/statistics.rs
git commit -m "fix(tauri): add missing net_revenue/anulacion fields to StoreOverview struct"
```

---

## Task 2: Unify Date Range Presets — Edge-Server

**Files:**
- Modify: `edge-server/src/api/statistics/handler.rs:185-252`

**Step 1: Add `yesterday` and `last_month` to `calculate_time_range()`**

在 `match time_range` 中添加两个新分支：

```rust
match time_range {
    "today" => (
        cutoff_millis(today),
        cutoff_millis(today + Duration::days(1)),
    ),
    "yesterday" => {
        let yesterday = today - Duration::days(1);
        (
            cutoff_millis(yesterday),
            cutoff_millis(today),
        )
    },
    "this_week" | "week" => {
        let weekday = today.weekday().num_days_from_monday();
        let week_start = today - Duration::days(weekday as i64);
        (
            cutoff_millis(week_start),
            cutoff_millis(today + Duration::days(1)),
        )
    }
    "this_month" | "month" => {
        let month_start = today.with_day(1).unwrap_or(today);
        (
            cutoff_millis(month_start),
            cutoff_millis(today + Duration::days(1)),
        )
    }
    "last_month" => {
        let this_month_start = today.with_day(1).unwrap_or(today);
        let last_month_start = (this_month_start - Duration::days(1))
            .with_day(1)
            .unwrap_or(this_month_start - Duration::days(28));
        (
            cutoff_millis(last_month_start),
            cutoff_millis(this_month_start),
        )
    }
    "custom" => {
        // ... 保持不变
    }
    _ => (
        cutoff_millis(today),
        cutoff_millis(today + Duration::days(1)),
    ),
}
```

注意: 保留 `"week"` 和 `"month"` 旧值作为别名 (`"this_week" | "week"`)，确保向后兼容。

**Step 2: Verify compilation**

Run: `cargo check -p edge-server`
Expected: PASS

**Step 3: Commit**

```bash
git add edge-server/src/api/statistics/handler.rs
git commit -m "feat(edge): add yesterday/last_month presets to calculate_time_range"
```

---

## Task 3: Unify Date Range Presets — RedCoral Frontend

**Files:**
- Modify: `red_coral/src/core/domain/types/index.ts:95-96`
- Modify: `red_coral/src/screens/Statistics/index.tsx:29-54` (computeRange)
- Modify: `red_coral/src/screens/Statistics/index.tsx:162-171` (select options)
- Modify: `red_coral/src/infrastructure/i18n/locales/zh-CN.json`
- Modify: `red_coral/src/infrastructure/i18n/locales/es-ES.json`

**Step 1: Update TypeScript types**

```typescript
// red_coral/src/core/domain/types/index.ts:95-96
export type TimeRange = 'today' | 'yesterday' | 'this_week' | 'this_month' | 'last_month' | 'custom';
export type ActiveTab = 'overview' | 'invoices' | 'reports_shifts' | 'audit_log';
```

**Step 2: Rewrite `computeRange()` to use business-day-aware presets**

```typescript
function computeRange(range: TimeRange, cutoffMinutes: number, customStart?: string, customEnd?: string): { from: number; to: number } | null {
  if (range === 'custom') {
    if (!customStart || !customEnd) return null;
    return { from: new Date(customStart).getTime(), to: new Date(customEnd).getTime() };
  }

  const now = new Date();
  const todayStart = new Date(now);
  todayStart.setHours(0, 0, 0, 0);
  const cutoffMs = cutoffMinutes * 60_000;
  const DAY = 86_400_000;

  // Determine today's business day start
  const bizDayStart = todayStart.getTime() + cutoffMs;
  const todayBiz = now.getTime() < bizDayStart ? bizDayStart - DAY : bizDayStart;

  switch (range) {
    case 'today':
      return { from: todayBiz, to: todayBiz + DAY };
    case 'yesterday':
      return { from: todayBiz - DAY, to: todayBiz };
    case 'this_week': {
      const day = now.getDay(); // 0=Sun
      const daysSinceMonday = day === 0 ? 6 : day - 1;
      const weekStart = todayBiz - daysSinceMonday * DAY;
      return { from: weekStart, to: todayBiz + DAY };
    }
    case 'this_month': {
      const d = new Date(todayBiz);
      d.setDate(1);
      d.setHours(0, 0, 0, 0);
      const monthStart = d.getTime() + cutoffMs;
      return { from: monthStart, to: todayBiz + DAY };
    }
    case 'last_month': {
      const d = new Date(todayBiz);
      d.setDate(1);
      d.setHours(0, 0, 0, 0);
      const thisMonthStart = d.getTime() + cutoffMs;
      const d2 = new Date(thisMonthStart - DAY);
      d2.setDate(1);
      d2.setHours(0, 0, 0, 0);
      const lastMonthStart = d2.getTime() + cutoffMs;
      return { from: lastMonthStart, to: thisMonthStart };
    }
    default:
      return null;
  }
}
```

**Step 3: Update select options in `index.tsx`**

替换 `<select>` 的 `<option>` 列表:

```tsx
<option value="today">{t('statistics.time.today')}</option>
<option value="yesterday">{t('statistics.time.yesterday')}</option>
<option value="this_week">{t('statistics.time.this_week')}</option>
<option value="this_month">{t('statistics.time.this_month')}</option>
<option value="last_month">{t('statistics.time.last_month')}</option>
<option value="custom">{t('statistics.time.custom')}</option>
```

**Step 4: Add i18n keys**

zh-CN.json:
```json
"statistics.time.yesterday": "昨天",
"statistics.time.this_week": "本周",
"statistics.time.this_month": "本月",
"statistics.time.last_month": "上月"
```

es-ES.json:
```json
"statistics.time.yesterday": "Ayer",
"statistics.time.this_week": "Esta semana",
"statistics.time.this_month": "Este mes",
"statistics.time.last_month": "Mes anterior"
```

**Step 5: Fix all TypeScript usages of old `TimeRange` values**

搜索 `'week'`、`'month'`、`'year'`、`'sales'`、`'daily_report'` 在 Statistics 目录下的引用，替换为新值。特别注意:
- `Sidebar.tsx` 的 tab menu items
- `SalesReport.tsx` 接收的 `timeRange` prop

**Step 6: Verify compilation**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

**Step 7: Commit**

```bash
git add red_coral/src/core/domain/types/index.ts \
        red_coral/src/screens/Statistics/index.tsx \
        red_coral/src/screens/Statistics/components/Sidebar.tsx \
        red_coral/src/infrastructure/i18n/locales/zh-CN.json \
        red_coral/src/infrastructure/i18n/locales/es-ES.json
git commit -m "feat(redcoral): unify date range presets (yesterday/this_week/this_month/last_month)"
```

---

## Task 4: Red Flags Backend — Edge-Server

**Files:**
- Modify: `edge-server/src/api/statistics/handler.rs` (新增 handler)
- Modify: `edge-server/src/api/statistics/mod.rs` (新增路由)

**Step 1: Add Red Flags response types and handler**

在 `handler.rs` 添加:

```rust
// ── Red Flags ──

#[derive(Debug, Serialize)]
pub struct RedFlagsSummary {
    pub item_removals: i64,
    pub item_comps: i64,
    pub order_voids: i64,
    pub order_discounts: i64,
    pub price_modifications: i64,
}

#[derive(Debug, Serialize)]
pub struct OperatorRedFlags {
    pub operator_id: i64,
    pub operator_name: String,
    pub item_removals: i64,
    pub item_comps: i64,
    pub order_voids: i64,
    pub order_discounts: i64,
    pub price_modifications: i64,
}

#[derive(Debug, Serialize)]
pub struct RedFlagsResponse {
    pub summary: RedFlagsSummary,
    pub operator_breakdown: Vec<OperatorRedFlags>,
}

/// GET /api/statistics/red-flags
pub async fn get_red_flags(
    State(state): State<ServerState>,
    Query(params): Query<StatisticsQuery>,
) -> AppResult<Json<RedFlagsResponse>> {
    let store_info = store_info::get(&state.pool).await?;
    let tz = time::parse_timezone(&store_info.timezone);
    let cutoff = store_info.business_day_cutoff.unwrap_or(0);

    let (start, end) = calculate_time_range(
        &params.time_range,
        cutoff,
        params.custom_start.as_deref(),
        params.custom_end.as_deref(),
        tz,
    );

    // Summary: count events by type within time range
    let summary_rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT ae.event_type, COUNT(*)
         FROM archived_order_event ae
         JOIN archived_order o ON ae.order_pk = o.id
         WHERE o.end_time >= ?1 AND o.end_time < ?2
           AND ae.event_type IN ('ITEM_REMOVED', 'ITEM_COMPED', 'ORDER_VOIDED', 'ORDER_DISCOUNT_APPLIED', 'ITEM_MODIFIED')
         GROUP BY ae.event_type"
    )
    .bind(start)
    .bind(end)
    .fetch_all(&state.pool)
    .await?;

    let mut summary = RedFlagsSummary {
        item_removals: 0, item_comps: 0, order_voids: 0,
        order_discounts: 0, price_modifications: 0,
    };
    for (event_type, count) in &summary_rows {
        match event_type.as_str() {
            "ITEM_REMOVED" => summary.item_removals = *count,
            "ITEM_COMPED" => summary.item_comps = *count,
            "ORDER_VOIDED" => summary.order_voids = *count,
            "ORDER_DISCOUNT_APPLIED" => summary.order_discounts = *count,
            "ITEM_MODIFIED" => summary.price_modifications = *count,
            _ => {}
        }
    }

    // Operator breakdown
    let operator_rows = sqlx::query_as::<_, (i64, String, String, i64)>(
        "SELECT ae.operator_id, ae.operator_name, ae.event_type, COUNT(*)
         FROM archived_order_event ae
         JOIN archived_order o ON ae.order_pk = o.id
         WHERE o.end_time >= ?1 AND o.end_time < ?2
           AND ae.event_type IN ('ITEM_REMOVED', 'ITEM_COMPED', 'ORDER_VOIDED', 'ORDER_DISCOUNT_APPLIED', 'ITEM_MODIFIED')
         GROUP BY ae.operator_id, ae.operator_name, ae.event_type"
    )
    .bind(start)
    .bind(end)
    .fetch_all(&state.pool)
    .await?;

    let mut op_map: std::collections::HashMap<i64, OperatorRedFlags> = std::collections::HashMap::new();
    for (op_id, op_name, event_type, count) in operator_rows {
        let entry = op_map.entry(op_id).or_insert_with(|| OperatorRedFlags {
            operator_id: op_id,
            operator_name: op_name.clone(),
            item_removals: 0, item_comps: 0, order_voids: 0,
            order_discounts: 0, price_modifications: 0,
        });
        match event_type.as_str() {
            "ITEM_REMOVED" => entry.item_removals = count,
            "ITEM_COMPED" => entry.item_comps = count,
            "ORDER_VOIDED" => entry.order_voids = count,
            "ORDER_DISCOUNT_APPLIED" => entry.order_discounts = count,
            "ITEM_MODIFIED" => entry.price_modifications = count,
            _ => {}
        }
    }

    Ok(Json(RedFlagsResponse {
        summary,
        operator_breakdown: op_map.into_values().collect(),
    }))
}
```

**Step 2: Check `archived_order_event` schema for `operator_id` / `operator_name` columns**

这些字段可能不在 `archived_order_event` 表上，需要检查。如果缺失，改用 `JOIN archived_order o` 取 `o.operator_id` / `o.operator_name`，或从 event `data` JSON 提取。实际实现时需要核实 schema。

**Step 3: Register route**

在 `edge-server/src/api/statistics/mod.rs`:

```rust
fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::get_statistics))
        .route("/sales-report", get(handler::get_sales_report))
        .route("/red-flags", get(handler::get_red_flags))
        .layer(middleware::from_fn(require_permission("reports:view")))
}
```

**Step 4: Verify compilation**

Run: `cargo check -p edge-server`
Expected: PASS

**Step 5: Commit**

```bash
git add edge-server/src/api/statistics/handler.rs edge-server/src/api/statistics/mod.rs
git commit -m "feat(edge): add GET /api/statistics/red-flags endpoint"
```

---

## Task 5: Invoice List Backend — Edge-Server

**Files:**
- Modify: `edge-server/src/db/repository/invoice.rs` (新增查询)
- Modify: `edge-server/src/api/statistics/handler.rs` (新增 handler)
- Modify: `edge-server/src/api/statistics/mod.rs` (新增路由)

**Step 1: Add paginated invoice list to repository**

在 `edge-server/src/db/repository/invoice.rs` 添加:

```rust
#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct InvoiceListRow {
    pub id: i64,
    pub invoice_number: String,
    pub tipo_factura: String,
    pub receipt_number: Option<String>,
    pub credit_note_number: Option<String>,
    pub total: f64,
    pub tax_total: f64,
    pub aeat_status: String,
    pub created_at: i64,
}

pub async fn list_paginated(
    pool: &SqlitePool,
    from: i64,
    to: i64,
    tipo: Option<&str>,
    aeat_status: Option<&str>,
    limit: i32,
    offset: i32,
) -> Result<(Vec<InvoiceListRow>, i64), sqlx::Error> {
    // Build dynamic WHERE clause
    let mut conditions = vec!["created_at >= ?1 AND created_at < ?2".to_string()];
    if tipo.is_some() { conditions.push("tipo_factura = ?3".to_string()); }
    if aeat_status.is_some() { conditions.push("aeat_status = ?4".to_string()); }
    let where_clause = conditions.join(" AND ");

    let count_sql = format!("SELECT COUNT(*) FROM invoice WHERE {where_clause}");
    let list_sql = format!(
        "SELECT id, invoice_number, tipo_factura, receipt_number, credit_note_number, \
         total, tax_total, aeat_status, created_at \
         FROM invoice WHERE {where_clause} ORDER BY id DESC LIMIT ?5 OFFSET ?6"
    );

    // 使用 sqlx::query_scalar 和 sqlx::query_as，绑定参数
    // 注意: 实现时需要用 sqlx::QueryBuilder 或条件绑定
    // 以下为伪代码，实际实现需根据 sqlx API 调整

    let total: i64 = /* count query */ ;
    let rows: Vec<InvoiceListRow> = /* list query */ ;

    Ok((rows, total))
}
```

注意: SQLite 的 sqlx 不支持 `query_builder` 动态绑定位置参数，实际实现时需要根据筛选条件动态构建 SQL 字符串（参考 `handler.rs` 中已有的动态 SQL 模式）。

**Step 2: Add handler**

在 `handler.rs` 添加:

```rust
#[derive(Debug, Deserialize)]
pub struct InvoiceListQuery {
    #[serde(default = "default_time_range")]
    pub time_range: String,
    pub custom_start: Option<String>,
    pub custom_end: Option<String>,
    pub tipo: Option<String>,
    pub aeat_status: Option<String>,
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(default = "default_page_size")]
    pub page_size: i32,
}

fn default_page() -> i32 { 1 }
fn default_page_size() -> i32 { 20 }

#[derive(Debug, Serialize)]
pub struct InvoiceListResponse {
    pub invoices: Vec<invoice::InvoiceListRow>,
    pub total: i64,
    pub page: i32,
    pub page_size: i32,
}

/// GET /api/statistics/invoices
pub async fn list_invoices(
    State(state): State<ServerState>,
    Query(params): Query<InvoiceListQuery>,
) -> AppResult<Json<InvoiceListResponse>> {
    let store_info = store_info::get(&state.pool).await?;
    let tz = time::parse_timezone(&store_info.timezone);
    let cutoff = store_info.business_day_cutoff.unwrap_or(0);

    let (start, end) = calculate_time_range(
        &params.time_range, cutoff,
        params.custom_start.as_deref(), params.custom_end.as_deref(), tz,
    );

    let offset = (params.page - 1) * params.page_size;
    let (invoices, total) = invoice::list_paginated(
        &state.pool, start, end,
        params.tipo.as_deref(), params.aeat_status.as_deref(),
        params.page_size, offset,
    ).await?;

    Ok(Json(InvoiceListResponse {
        invoices, total,
        page: params.page,
        page_size: params.page_size,
    }))
}
```

**Step 3: Register route**

```rust
.route("/invoices", get(handler::list_invoices))
```

**Step 4: Add `use crate::db::repository::invoice;` to handler imports if not present**

**Step 5: Verify compilation**

Run: `cargo check -p edge-server`
Expected: PASS

**Step 6: Commit**

```bash
git add edge-server/src/db/repository/invoice.rs \
        edge-server/src/api/statistics/handler.rs \
        edge-server/src/api/statistics/mod.rs
git commit -m "feat(edge): add GET /api/statistics/invoices paginated endpoint"
```

---

## Task 6: Add Tauri Commands for New Endpoints

**Files:**
- Modify: `red_coral/src-tauri/src/commands/statistics.rs`

**Step 1: Add Red Flags types and command**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedFlagsSummary {
    pub item_removals: i64,
    pub item_comps: i64,
    pub order_voids: i64,
    pub order_discounts: i64,
    pub price_modifications: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorRedFlags {
    pub operator_id: i64,
    pub operator_name: String,
    pub item_removals: i64,
    pub item_comps: i64,
    pub order_voids: i64,
    pub order_discounts: i64,
    pub price_modifications: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedFlagsResponse {
    pub summary: RedFlagsSummary,
    pub operator_breakdown: Vec<OperatorRedFlags>,
}

#[tauri::command]
pub async fn get_red_flags(
    bridge: State<'_, Arc<ClientBridge>>,
    from: i64,
    to: i64,
) -> Result<ApiResponse<RedFlagsResponse>, String> {
    bridge
        .get(&format!("/api/statistics/red-flags?time_range=custom&custom_start={from}&custom_end={to}"))
        .await
        .map_err(|e| e.to_string())
}
```

**Step 2: Add Invoice List types and command**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceListRow {
    pub id: i64,
    pub invoice_number: String,
    pub tipo_factura: String,
    pub receipt_number: Option<String>,
    pub credit_note_number: Option<String>,
    pub total: f64,
    pub tax_total: f64,
    pub aeat_status: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceListResponse {
    pub invoices: Vec<InvoiceListRow>,
    pub total: i64,
    pub page: i32,
    pub page_size: i32,
}

#[tauri::command]
pub async fn list_invoices(
    bridge: State<'_, Arc<ClientBridge>>,
    from: i64,
    to: i64,
    tipo: Option<String>,
    aeat_status: Option<String>,
    page: Option<i32>,
) -> Result<ApiResponse<InvoiceListResponse>, String> {
    let mut url = format!(
        "/api/statistics/invoices?time_range=custom&custom_start={from}&custom_end={to}"
    );
    if let Some(t) = tipo { url.push_str(&format!("&tipo={t}")); }
    if let Some(s) = aeat_status { url.push_str(&format!("&aeat_status={s}")); }
    if let Some(p) = page { url.push_str(&format!("&page={p}")); }
    bridge.get(&url).await.map_err(|e| e.to_string())
}
```

**Step 3: Register commands in `main.rs` / `lib.rs`**

在 Tauri `invoke_handler` 注册 `get_red_flags` 和 `list_invoices`。

**Step 4: Verify compilation**

Run: `cargo check -p red_coral-app`
Expected: PASS

**Step 5: Commit**

```bash
git add red_coral/src-tauri/src/commands/statistics.rs red_coral/src-tauri/src/lib.rs
git commit -m "feat(tauri): add get_red_flags and list_invoices commands"
```

---

## Task 7: Rewrite RedCoral Statistics — Tab Structure + Shared Time Range

**Files:**
- Modify: `red_coral/src/screens/Statistics/index.tsx` (全面重写)
- Modify: `red_coral/src/screens/Statistics/components/Sidebar.tsx`
- Modify: `red_coral/src/infrastructure/i18n/locales/zh-CN.json`
- Modify: `red_coral/src/infrastructure/i18n/locales/es-ES.json`

**Step 1: Rewrite `index.tsx` with 4-tab structure**

保持 `StatisticsScreen` 作为根组件，但:
- Tab 从 `overview | sales | daily_report | audit_log` → `overview | invoices | reports_shifts | audit_log`
- 所有 tab 共享同一个时间选择器 (顶部 bar)
- Overview tab 加载 Red Flags 数据

```tsx
// index.tsx 核心结构
const [activeTab, setActiveTab] = useState<ActiveTab>('overview');
const [timeRange, setTimeRange] = useState<TimeRange>('today');
// ...

// 根据 activeTab 渲染:
{activeTab === 'overview' && <Overview overview={data} redFlags={redFlags} ... />}
{activeTab === 'invoices' && <InvoiceList from={range.from} to={range.to} />}
{activeTab === 'reports_shifts' && <ReportsAndShifts from={range.from} to={range.to} />}
{activeTab === 'audit_log' && <AuditLog />}
```

**Step 2: Update Sidebar tab items**

```tsx
const menuItems = [
  { id: 'overview', icon: TrendingUp, label: t('statistics.sidebar.overview') },
  { id: 'invoices', icon: FileText, label: t('statistics.sidebar.invoices') },
  { id: 'reports_shifts', icon: ClipboardList, label: t('statistics.sidebar.reports_shifts') },
  { id: 'audit_log', icon: ShieldCheck, label: t('statistics.sidebar.audit_log') },
];
```

**Step 3: Add Red Flags fetch to overview loading**

在 `useEffect` 中，当 `activeTab === 'overview'` 时额外调用 `get_red_flags`：

```tsx
const redFlagsData = activeTab === 'overview'
  ? await invokeApi<RedFlagsResponse>('get_red_flags', { from: range.from, to: range.to }).catch(() => null)
  : null;
```

**Step 4: Add i18n keys**

```json
"statistics.sidebar.invoices": "发票明细",
"statistics.sidebar.reports_shifts": "日报 & 班次"
```

```json
"statistics.sidebar.invoices": "Facturas",
"statistics.sidebar.reports_shifts": "Informes y turnos"
```

**Step 5: Verify compilation**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

**Step 6: Commit**

```bash
git add red_coral/src/screens/Statistics/ \
        red_coral/src/core/domain/types/index.ts \
        red_coral/src/infrastructure/i18n/locales/
git commit -m "feat(redcoral): rewrite Statistics tab structure (overview/invoices/reports+shifts/audit)"
```

---

## Task 8: Red Flags Component — RedCoral

**Files:**
- Create: `red_coral/src/screens/Statistics/components/RedFlagsBar.tsx`
- Modify: `red_coral/src/screens/Statistics/components/Overview.tsx`

**Step 1: Create `RedFlagsBar.tsx`**

条件显示的红色警示条。当所有 flag 为 0 时隐藏。点击展开操作员明细。

```tsx
import React, { useState } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { AlertTriangle, ChevronDown, ChevronUp } from 'lucide-react';

interface RedFlagsBarProps {
  summary: {
    item_removals: number;
    item_comps: number;
    order_voids: number;
    order_discounts: number;
    price_modifications: number;
  };
  operatorBreakdown: Array<{
    operator_id: number;
    operator_name: string;
    item_removals: number;
    item_comps: number;
    order_voids: number;
    order_discounts: number;
    price_modifications: number;
  }>;
}

export const RedFlagsBar: React.FC<RedFlagsBarProps> = ({ summary, operatorBreakdown }) => {
  const { t } = useI18n();
  const [expanded, setExpanded] = useState(false);

  const total = summary.item_removals + summary.item_comps + summary.order_voids
    + summary.order_discounts + summary.price_modifications;

  if (total === 0) return null;

  const flags = [
    { key: 'item_removals', count: summary.item_removals, label: t('statistics.red_flags.item_removals') },
    { key: 'item_comps', count: summary.item_comps, label: t('statistics.red_flags.item_comps') },
    { key: 'order_voids', count: summary.order_voids, label: t('statistics.red_flags.order_voids') },
    { key: 'order_discounts', count: summary.order_discounts, label: t('statistics.red_flags.order_discounts') },
    { key: 'price_modifications', count: summary.price_modifications, label: t('statistics.red_flags.price_modifications') },
  ].filter(f => f.count > 0);

  return (
    <div className="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center justify-between"
      >
        <div className="flex items-center gap-3">
          <AlertTriangle className="w-5 h-5 text-red-500" />
          <div className="flex gap-4">
            {flags.map(f => (
              <span key={f.key} className="text-sm font-medium text-red-700">
                {f.label}: {f.count}
              </span>
            ))}
          </div>
        </div>
        {expanded ? <ChevronUp className="w-4 h-4 text-red-400" /> : <ChevronDown className="w-4 h-4 text-red-400" />}
      </button>

      {expanded && operatorBreakdown.length > 0 && (
        <div className="mt-3 pt-3 border-t border-red-200">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-red-600">
                <th className="text-left py-1">{t('statistics.red_flags.operator')}</th>
                {flags.map(f => <th key={f.key} className="text-center py-1">{f.label}</th>)}
              </tr>
            </thead>
            <tbody>
              {operatorBreakdown.map(op => (
                <tr key={op.operator_id} className="text-red-700">
                  <td className="py-1">{op.operator_name}</td>
                  {flags.map(f => (
                    <td key={f.key} className="text-center py-1">
                      {(op as any)[f.key] || 0}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};
```

**Step 2: Integrate into Overview**

在 `Overview.tsx` 接收 `redFlags` prop 并在 KPI 卡片上方渲染 `<RedFlagsBar />`。

**Step 3: Add i18n keys**

```json
"statistics.red_flags.item_removals": "删除菜品",
"statistics.red_flags.item_comps": "赠送",
"statistics.red_flags.order_voids": "作废订单",
"statistics.red_flags.order_discounts": "整单折扣",
"statistics.red_flags.price_modifications": "改价",
"statistics.red_flags.operator": "操作员"
```

**Step 4: Verify compilation**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

**Step 5: Commit**

```bash
git add red_coral/src/screens/Statistics/components/RedFlagsBar.tsx \
        red_coral/src/screens/Statistics/components/Overview.tsx \
        red_coral/src/infrastructure/i18n/locales/
git commit -m "feat(redcoral): add Red Flags warning bar to Statistics overview"
```

---

## Task 9: Invoice List Component — RedCoral

**Files:**
- Create: `red_coral/src/screens/Statistics/components/InvoiceList.tsx`

**Step 1: Create `InvoiceList.tsx`**

分页发票表，支持类型和 AEAT 状态筛选。REJECTED 状态红色高亮。

```tsx
import React, { useState, useEffect, useCallback } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { formatCurrency } from '@/utils/currency';

interface InvoiceListProps {
  from: number;
  to: number;
}

interface InvoiceRow {
  id: number;
  invoice_number: string;
  tipo_factura: string;
  receipt_number: string | null;
  credit_note_number: string | null;
  total: number;
  tax_total: number;
  aeat_status: string;
  created_at: number;
}

interface InvoiceListResponse {
  invoices: InvoiceRow[];
  total: number;
  page: number;
  page_size: number;
}

export const InvoiceList: React.FC<InvoiceListProps> = ({ from, to }) => {
  const { t } = useI18n();
  const [data, setData] = useState<InvoiceListResponse | null>(null);
  const [page, setPage] = useState(1);
  const [tipoFilter, setTipoFilter] = useState<string>('');
  const [statusFilter, setStatusFilter] = useState<string>('');

  const fetchInvoices = useCallback(async () => {
    const result = await invokeApi<InvoiceListResponse>('list_invoices', {
      from, to,
      tipo: tipoFilter || undefined,
      aeatStatus: statusFilter || undefined,
      page,
    });
    setData(result);
  }, [from, to, page, tipoFilter, statusFilter]);

  useEffect(() => { fetchInvoices(); }, [fetchInvoices]);
  useEffect(() => { setPage(1); }, [from, to, tipoFilter, statusFilter]);

  // 渲染表格: invoice_number, tipo_factura, 关联单号, total, tax_total, aeat_status, created_at
  // REJECTED 行 className="bg-red-50 text-red-700"
  // 分页: 上一页/下一页按钮
  // 筛选: tipo (全部/F2/R5) + aeat_status (全部/PENDING/SUBMITTED/ACCEPTED/REJECTED)

  return (/* 表格 JSX */);
};
```

实际渲染包含: 筛选行 (两个 select)、DataTable、分页控件。参考 `SalesReport.tsx` 的表格样式。

**Step 2: Add i18n keys**

```json
"statistics.invoices.number": "发票号",
"statistics.invoices.type": "类型",
"statistics.invoices.related": "关联单号",
"statistics.invoices.amount": "金额",
"statistics.invoices.tax": "税额",
"statistics.invoices.aeat_status": "AEAT 状态",
"statistics.invoices.date": "日期",
"statistics.invoices.all_types": "全部类型",
"statistics.invoices.all_statuses": "全部状态"
```

**Step 3: Verify compilation**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

**Step 4: Commit**

```bash
git add red_coral/src/screens/Statistics/components/InvoiceList.tsx \
        red_coral/src/infrastructure/i18n/locales/
git commit -m "feat(redcoral): add Invoice List tab to Statistics"
```

---

## Task 10: Reports & Shifts Tab — RedCoral

**Files:**
- Create: `red_coral/src/screens/Statistics/components/ReportsAndShifts.tsx`

**Step 1: Create `ReportsAndShifts.tsx`**

上下两个区域:
1. **日报区域**: 复用 `<DailyReportManagement />`
2. **班次区域**: 新增班次表格，调用现有 `GET /api/shifts` 端点

班次表格列: 操作员 | 状态 | 开始 | 结束 | 时长 | 初始现金 | 预期现金 | 实际现金 | 差异

差异为负值时红色显示。OPEN 状态绿色标记。

班次数据通过 `invokeApi('list_shifts', { startDate, endDate })` 获取（已有 Tauri 命令）。

**Step 2: Add i18n keys**

```json
"statistics.shifts.title": "班次记录",
"statistics.shifts.operator": "操作员",
"statistics.shifts.status": "状态",
"statistics.shifts.start": "开始",
"statistics.shifts.end": "结束",
"statistics.shifts.duration": "时长",
"statistics.shifts.starting_cash": "初始现金",
"statistics.shifts.expected_cash": "预期现金",
"statistics.shifts.actual_cash": "实际现金",
"statistics.shifts.variance": "差异"
```

**Step 3: Verify compilation**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

**Step 4: Commit**

```bash
git add red_coral/src/screens/Statistics/components/ReportsAndShifts.tsx \
        red_coral/src/infrastructure/i18n/locales/
git commit -m "feat(redcoral): add Reports & Shifts tab to Statistics"
```

---

## Task 11: Console TimeRangeSelector — Add `last_month`

**Files:**
- Modify: `crab-console/src/shared/components/TimeRangeSelector/TimeRangeSelector.tsx:139`
- Modify: `crab-console/src/infrastructure/i18n/locales/zh.json`
- Modify: `crab-console/src/infrastructure/i18n/locales/es.json`

**Step 1: Add `last_month` to PRESETS and type**

```typescript
// TimeRangePreset type (line 5):
export type TimeRangePreset = 'today' | 'yesterday' | 'this_week' | 'this_month' | 'last_month' | 'custom';

// PRESETS constant (line 139):
const PRESETS: TimeRangePreset[] = ['today', 'yesterday', 'this_week', 'this_month', 'last_month'];
```

**Step 2: Add `last_month` handling in `getPresetRange()`**

在 `getPresetRange` switch/if 中添加:

```typescript
case 'last_month': {
  const now = new Date();
  const thisMonthStart = new Date(now.getFullYear(), now.getMonth(), 1);
  const lastMonthStart = new Date(now.getFullYear(), now.getMonth() - 1, 1);
  const cutoffMs = (cutoffMinutes ?? 0) * 60_000;
  return {
    from: lastMonthStart.getTime() + cutoffMs,
    to: thisMonthStart.getTime() + cutoffMs,
    preset: 'last_month',
    label: t('timeRange.last_month'),
  };
}
```

**Step 3: Add `getPreviousRange` case for `last_month`**

```typescript
// last_month 的 previous 是 再上一个月
case 'last_month': {
  const d = new Date(range.from);
  d.setMonth(d.getMonth() - 1);
  const prevStart = d.getTime();
  return { from: prevStart, to: range.from };
}
```

**Step 4: Add i18n**

zh.json: `"timeRange.last_month": "上月"`
es.json: `"timeRange.last_month": "Mes anterior"`

**Step 5: Verify Console builds**

Run: `cd crab-console && npx tsc --noEmit`
Expected: PASS

**Step 6: Commit**

```bash
git add crab-console/src/shared/components/TimeRangeSelector/TimeRangeSelector.tsx \
        crab-console/src/infrastructure/i18n/locales/
git commit -m "feat(console): add last_month preset to TimeRangeSelector"
```

---

## Task 12: Final Verification

**Step 1: Full workspace Rust check**

Run: `cargo check --workspace`
Expected: PASS

**Step 2: Full TypeScript check**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

**Step 3: Console TypeScript check**

Run: `cd crab-console && npx tsc --noEmit`
Expected: PASS

**Step 4: Run Rust tests**

Run: `cargo test --workspace --lib`
Expected: PASS

**Step 5: Clippy check**

Run: `cargo clippy --workspace`
Expected: Zero warnings

**Step 6: Final commit (if any cleanup needed)**

```bash
git commit -m "chore: statistics alignment final verification pass"
```
