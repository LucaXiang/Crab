# 日报重构：精简为班次结算 + 自动生成

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 重构日报系统 — 删除与概览重复的聚合数据，精简为班次现金对账 + 摘要快照；新增自动生成、启动补漏、30天清理；修复班次现金退款扣减。

**Architecture:** 日报从"完整统计快照"瘦身为"日终班次结算记录"。概览 (StoreOverview) 承担所有统计展示职责。日报只保留：(1) 班次现金对账快照 (shift_breakdowns)，(2) 一行精简摘要数字 (net_revenue, orders, refund_amount 等，未来用于推送)。自动生成复用 ShiftAutoCloseScheduler 的 cutoff 定时模式。

**Tech Stack:** Rust (edge-server, shared, crab-cloud), TypeScript (red_coral, crab-console), SQLite, PostgreSQL

---

## 变更概览

### 删除（减法）
- `daily_report` 表: 删除 `total_orders`, `completed_orders`, `void_orders`, `total_sales`, `total_paid`, `total_unpaid`, `void_amount`, `total_tax`, `total_discount`, `total_surcharge` (10 列)
- `daily_report_tax_breakdown` 表: **整表删除**
- `daily_report_payment_breakdown` 表: **整表删除**
- `DailyReport` model: 删除对应字段和 `tax_breakdowns`, `payment_breakdowns` Vec
- `generate()`: 删除所有 archived_order 聚合查询和 tax/payment breakdown 写入
- 前端: 删除日报详情弹窗中的销售汇总、税率分解、支付方式分解区块

### 新增（加法）
- `daily_report` 表: 新增 `net_revenue`, `total_orders`, `refund_amount`, `refund_count` (4 列精简摘要)
- `daily_report` 表: 新增 `auto_generated` bool (区分自动/手动)
- `generate()`: 新增 3 个简单聚合查询 (archived_order + credit_note)
- `DailyReportScheduler`: 自动生成 + 启动补漏
- `daily_report_cleanup`: 30 天清理定时任务
- `shift::deduct_cash_refund()`: 现金退款扣减 expected_cash
- Cloud PG: 对应 schema 变更

### 保留（不变）
- `daily_report_shift_breakdown` 表: 完整保留（日报核心价值）
- `shift_breakdowns` Vec: 保留
- 概览 API (StoreOverview): 不变，已包含所有统计数据
- Cloud 同步机制: 保留 SyncResource::DailyReport 通道

---

## Task 1: Migration — Edge SQLite schema 变更

**Files:**
- Create: `edge-server/migrations/XXXX_daily_report_simplify.sql` (up + down)

**Step 1: 创建迁移文件**

```bash
cd /Users/xzy/workspace/crab && sqlx migrate add -r -s daily_report_simplify --source edge-server/migrations
```

**Step 2: 编写 up migration**

```sql
-- 1. 删除不再需要的子表
DROP TABLE IF EXISTS daily_report_tax_breakdown;
DROP TABLE IF EXISTS daily_report_payment_breakdown;

-- 2. 重建 daily_report 表（SQLite 不支持 DROP COLUMN）
CREATE TABLE daily_report_new (
    id              INTEGER PRIMARY KEY,
    business_date   TEXT NOT NULL UNIQUE,
    -- 精简摘要（用于未来推送，非详细统计）
    net_revenue     REAL NOT NULL DEFAULT 0.0,
    total_orders    INTEGER NOT NULL DEFAULT 0,
    refund_amount   REAL NOT NULL DEFAULT 0.0,
    refund_count    INTEGER NOT NULL DEFAULT 0,
    -- 元数据
    auto_generated  INTEGER NOT NULL DEFAULT 0,
    generated_at    INTEGER,
    generated_by_id INTEGER,
    generated_by_name TEXT,
    note            TEXT,
    created_at      INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at      INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
);

-- 3. 迁移现有数据（best effort：用旧的 total_sales 作为 net_revenue）
INSERT INTO daily_report_new (id, business_date, net_revenue, total_orders, generated_at, generated_by_id, generated_by_name, note, created_at, updated_at)
SELECT id, business_date, total_sales - COALESCE(total_unpaid, 0.0), total_orders, generated_at, generated_by_id, generated_by_name, note, COALESCE(generated_at, unixepoch() * 1000), COALESCE(generated_at, unixepoch() * 1000)
FROM daily_report;

-- 4. 替换表
DROP TABLE daily_report;
ALTER TABLE daily_report_new RENAME TO daily_report;
```

**Step 3: 编写 down migration**

```sql
-- Reversible down 暂不实现，因为旧数据已丢失
-- 但提供 schema 回滚
DROP TABLE IF EXISTS daily_report;
CREATE TABLE daily_report (
    id INTEGER PRIMARY KEY,
    business_date TEXT NOT NULL UNIQUE,
    total_orders INTEGER NOT NULL DEFAULT 0,
    completed_orders INTEGER NOT NULL DEFAULT 0,
    void_orders INTEGER NOT NULL DEFAULT 0,
    total_sales REAL NOT NULL DEFAULT 0.0,
    total_paid REAL NOT NULL DEFAULT 0.0,
    total_unpaid REAL NOT NULL DEFAULT 0.0,
    void_amount REAL NOT NULL DEFAULT 0.0,
    total_tax REAL NOT NULL DEFAULT 0.0,
    total_discount REAL NOT NULL DEFAULT 0.0,
    total_surcharge REAL NOT NULL DEFAULT 0.0,
    generated_at INTEGER,
    generated_by_id INTEGER,
    generated_by_name TEXT,
    note TEXT
);

CREATE TABLE daily_report_tax_breakdown (
    id INTEGER PRIMARY KEY,
    report_id INTEGER NOT NULL REFERENCES daily_report(id),
    tax_rate INTEGER NOT NULL,
    net_amount REAL NOT NULL DEFAULT 0.0,
    tax_amount REAL NOT NULL DEFAULT 0.0,
    gross_amount REAL NOT NULL DEFAULT 0.0,
    order_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE daily_report_payment_breakdown (
    id INTEGER PRIMARY KEY,
    report_id INTEGER NOT NULL REFERENCES daily_report(id),
    method TEXT NOT NULL,
    amount REAL NOT NULL DEFAULT 0.0,
    count INTEGER NOT NULL DEFAULT 0
);
```

**Step 4: Commit**

```
feat(edge): migration to simplify daily_report schema
```

---

## Task 2: Shared model — 精简 DailyReport

**Files:**
- Modify: `shared/src/models/daily_report.rs`

**Step 1: 重写 DailyReport model**

```rust
//! Daily Report Model (日结报告)
//!
//! 精简版：班次现金对账 + 摘要快照。
//! 详细统计由 StoreOverview (概览) 承担。

use serde::{Deserialize, Serialize};

/// Shift breakdown within a daily report (日报核心：班次现金对账)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct ShiftBreakdown {
    pub id: i64,
    pub report_id: i64,
    pub shift_id: i64,
    pub operator_id: i64,
    pub operator_name: String,
    pub status: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub starting_cash: f64,
    pub expected_cash: f64,
    pub actual_cash: Option<f64>,
    pub cash_variance: Option<f64>,
    pub abnormal_close: bool,
    pub total_orders: i64,
    pub completed_orders: i64,
    pub void_orders: i64,
    pub total_sales: f64,
    pub total_paid: f64,
    pub void_amount: f64,
    pub total_tax: f64,
    pub total_discount: f64,
    pub total_surcharge: f64,
}

/// Daily Report — 日终班次结算记录
///
/// 核心数据: shift_breakdowns (班次现金对账)
/// 摘要数字: net_revenue / total_orders / refund (用于列表展示和未来推送)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct DailyReport {
    pub id: i64,
    /// Business date (YYYY-MM-DD format)
    pub business_date: String,
    /// 精简摘要
    pub net_revenue: f64,
    pub total_orders: i64,
    pub refund_amount: f64,
    pub refund_count: i64,
    /// 是否自动生成
    pub auto_generated: bool,
    /// When the report was generated (Unix millis)
    pub generated_at: Option<i64>,
    pub generated_by_id: Option<i64>,
    pub generated_by_name: Option<String>,
    pub note: Option<String>,

    /// 班次现金对账 (日报的核心价值)
    #[cfg_attr(feature = "db", sqlx(skip))]
    #[serde(default)]
    pub shift_breakdowns: Vec<ShiftBreakdown>,
}

/// Generate daily report payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReportGenerate {
    /// Business date to generate report for (YYYY-MM-DD)
    pub business_date: String,
    pub note: Option<String>,
}
```

**注意:** 删除 `TaxBreakdown`, `PaymentMethodBreakdown` struct（不再使用）。

**Step 2: Commit**

```
refactor(shared): simplify DailyReport model — remove duplicated aggregates
```

---

## Task 3: Edge repository — 重写 generate() + 删除 tax/payment breakdown

**Files:**
- Modify: `edge-server/src/db/repository/daily_report.rs`

**Step 1: 重写 generate() 函数**

核心变化:
- 删除所有 `archived_order` 逐字段聚合 (total_sales, void_orders 等 ~8 个查询)
- 删除 tax_breakdown、payment_breakdown 的写入逻辑
- 新增 3 个简单查询: revenue (已有类似), refund (credit_note), orders count
- 保留 shift_breakdown 写入 (不变)

```rust
pub async fn generate(
    pool: &SqlitePool,
    data: DailyReportGenerate,
    start_millis: i64,
    end_millis: i64,
    operator_id: Option<i64>,
    operator_name: Option<String>,
    auto_generated: bool,
) -> RepoResult<DailyReport> {
    let now = shared::util::now_millis();

    // ── 精简摘要 (3 个查询) ──

    // 1. 完成订单数 + 销售额
    let (completed_orders, total_sales): (i64, f64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(total_amount), 0.0) \
         FROM archived_order \
         WHERE end_time >= ? AND end_time < ? AND status = 'COMPLETED' AND is_voided = 0",
    )
    .bind(start_millis).bind(end_millis)
    .fetch_one(pool).await?;

    // 2. 退款汇总
    let (refund_count, refund_amount): (i64, f64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(total_credit), 0.0) \
         FROM credit_note WHERE created_at >= ? AND created_at < ?",
    )
    .bind(start_millis).bind(end_millis)
    .fetch_one(pool).await?;

    let net_revenue = total_sales - refund_amount;

    // ── 创建报告 + shift breakdown 事务 ──
    let mut tx = pool.begin().await?;
    let report_id = shared::util::snowflake_id();

    sqlx::query(
        "INSERT INTO daily_report (id, business_date, net_revenue, total_orders, refund_amount, refund_count, auto_generated, generated_at, generated_by_id, generated_by_name, note) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
    )
    .bind(report_id)
    .bind(&data.business_date)
    .bind(net_revenue)
    .bind(completed_orders)
    .bind(refund_amount)
    .bind(refund_count)
    .bind(auto_generated)
    .bind(now)
    .bind(operator_id)
    .bind(&operator_name)
    .bind(&data.note)
    .execute(&mut *tx)
    .await?;

    // ── Shift breakdown (保留不变) ──
    // ... 现有 shift_rows 查询 + 写入逻辑完全保留 ...

    tx.commit().await?;

    find_by_id(pool, report_id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to generate daily report".into()))
}
```

**Step 2: 更新查询函数**

- `find_by_id` / `find_by_date` / `find_all` / `find_by_date_range`: 更新 SELECT 列名
- 删除 `find_tax_breakdowns()`, `find_payment_breakdowns()` 函数
- `batch_load_breakdowns()`: 只加载 shift_breakdowns

**Step 3: 新增 cleanup 函数**

```rust
/// 删除超过 retention_days 天的日报 (含 shift_breakdown)
pub async fn cleanup_old_reports(pool: &SqlitePool, retention_days: i32) -> RepoResult<u64> {
    let cutoff_date = chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::days(retention_days as i64))
        .unwrap_or(chrono::Utc::now())
        .format("%Y-%m-%d")
        .to_string();

    // 先删子表
    let report_ids: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM daily_report WHERE business_date < ?",
    )
    .bind(&cutoff_date)
    .fetch_all(pool)
    .await?;

    if report_ids.is_empty() {
        return Ok(0);
    }

    let ids: Vec<i64> = report_ids.into_iter().map(|(id,)| id).collect();
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");

    let shift_sql = format!(
        "DELETE FROM daily_report_shift_breakdown WHERE report_id IN ({placeholders})"
    );
    let mut q = sqlx::query(&shift_sql);
    for id in &ids {
        q = q.bind(id);
    }
    q.execute(pool).await?;

    // 再删主表
    let result = sqlx::query("DELETE FROM daily_report WHERE business_date < ?")
        .bind(&cutoff_date)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}
```

**Step 4: Commit**

```
refactor(edge): rewrite daily_report generate() — shift-only + summary snapshot
```

---

## Task 4: DailyReportScheduler — 自动生成 + 启动补漏 + 30天清理

**Files:**
- Create: `edge-server/src/daily_reports.rs`
- Modify: `edge-server/src/core/state.rs` (注册任务)
- Modify: `edge-server/src/lib.rs` (mod 声明)

**Step 1: 创建 DailyReportScheduler**

模式完全复用 `ShiftAutoCloseScheduler`:
- 启动时补漏: 检查昨天是否有日报，没有则自动生成
- cutoff 定时: 营业日切换时自动生成前一天的日报
- 30天清理: 每次触发时顺便清理过期日报
- config_notify: 支持 cutoff 配置变更后重算

```rust
//! 日报自动生成调度器
//!
//! 在 `business_day_cutoff` 时间点自动为前一营业日生成日报，
//! 启动时补漏检查最近 7 天的缺失日报。
//! 同时执行 30 天清理。

use std::sync::Arc;
use chrono::NaiveTime;
use chrono_tz::Tz;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::core::ServerState;
use crate::db::repository::{daily_report, store_info};
use crate::utils::time;

const RETENTION_DAYS: i32 = 30;
const CATCHUP_DAYS: i64 = 7;

pub struct DailyReportScheduler {
    state: ServerState,
    shutdown: CancellationToken,
    config_notify: Arc<Notify>,
}

impl DailyReportScheduler {
    pub fn new(state: ServerState, shutdown: CancellationToken) -> Self {
        let config_notify = state.config_notify.clone();
        Self { state, shutdown, config_notify }
    }

    pub async fn run(self) {
        tracing::info!("DailyReport scheduler started");

        // 启动时补漏 + 清理
        self.catchup_missing_reports().await;
        self.cleanup_old_reports().await;

        loop {
            let cutoff_time = self.get_cutoff_time().await;
            let tz = self.state.config.timezone;
            let sleep_duration = crate::shifts::ShiftAutoCloseScheduler::duration_until_next_cutoff(cutoff_time, tz);

            tracing::info!(
                "Next daily report generation in {} minutes",
                sleep_duration.as_secs() / 60,
            );

            tokio::select! {
                _ = tokio::time::sleep(sleep_duration) => {
                    self.generate_for_previous_day().await;
                    self.cleanup_old_reports().await;
                }
                _ = self.config_notify.notified() => {
                    tracing::info!("Config changed, recalculating next daily report trigger");
                }
                _ = self.shutdown.cancelled() => {
                    tracing::info!("DailyReport scheduler shutting down");
                    return;
                }
            }
        }
    }

    /// 为前一营业日自动生成日报
    async fn generate_for_previous_day(&self) {
        let cutoff_time = self.get_cutoff_time().await;
        let tz = self.state.config.timezone;
        let today = time::current_business_date(cutoff_time, tz);
        let yesterday = today - chrono::Duration::days(1);

        self.generate_if_missing(yesterday, cutoff_time, tz).await;
    }

    /// 启动时补漏: 检查最近 CATCHUP_DAYS 天
    async fn catchup_missing_reports(&self) {
        let cutoff_time = self.get_cutoff_time().await;
        let tz = self.state.config.timezone;
        let today = time::current_business_date(cutoff_time, tz);

        for i in 1..=CATCHUP_DAYS {
            let date = today - chrono::Duration::days(i);
            self.generate_if_missing(date, cutoff_time, tz).await;
        }
    }

    /// 如果该日期没有日报则自动生成
    async fn generate_if_missing(&self, date: chrono::NaiveDate, cutoff_time: NaiveTime, tz: Tz) {
        let date_str = date.format("%Y-%m-%d").to_string();

        match daily_report::find_by_date(&self.state.pool, &date_str).await {
            Ok(Some(_)) => return, // 已存在
            Ok(None) => {}
            Err(e) => {
                tracing::error!("Failed to check daily report for {}: {}", date_str, e);
                return;
            }
        }

        let start = time::date_cutoff_millis(date, cutoff_time, tz);
        let end = time::date_cutoff_millis(date + chrono::Duration::days(1), cutoff_time, tz);

        let data = shared::models::DailyReportGenerate {
            business_date: date_str.clone(),
            note: None,
        };

        match daily_report::generate(&self.state.pool, data, start, end, None, None, true).await {
            Ok(report) => {
                tracing::info!("Auto-generated daily report for {} (id={})", date_str, report.id);
                // 触发 cloud 同步
                self.state.broadcast_sync(
                    shared::cloud::SyncResource::DailyReport,
                    shared::message::SyncChangeType::Created,
                    report.id,
                    Some(&report),
                    false,
                ).await;
            }
            Err(e) => {
                tracing::error!("Failed to auto-generate daily report for {}: {}", date_str, e);
            }
        }
    }

    /// 清理超过 30 天的日报
    async fn cleanup_old_reports(&self) {
        match daily_report::cleanup_old_reports(&self.state.pool, RETENTION_DAYS).await {
            Ok(0) => {}
            Ok(n) => tracing::info!("Cleaned up {} old daily reports (>{} days)", n, RETENTION_DAYS),
            Err(e) => tracing::error!("Failed to cleanup old daily reports: {}", e),
        }
    }

    async fn get_cutoff_time(&self) -> NaiveTime {
        let cutoff = store_info::get(&self.state.pool)
            .await.ok().flatten()
            .map(|s| s.business_day_cutoff)
            .unwrap_or(0);
        time::cutoff_to_time(cutoff)
    }
}
```

**Step 2: 注册到 BackgroundTasks**

在 `state.rs` 的 `start_background_tasks()` 中，在 `register_shift_auto_close` 后面添加:

```rust
// DailyReportScheduler: 自动生成日报 + 启动补漏 + 30天清理
self.register_daily_report_scheduler(&mut tasks);
```

新增方法:

```rust
fn register_daily_report_scheduler(&self, tasks: &mut BackgroundTasks) {
    let scheduler = crate::daily_reports::DailyReportScheduler::new(
        self.clone(),
        tasks.shutdown_token(),
    );
    tasks.spawn("daily_report_scheduler", TaskKind::Periodic, async move {
        scheduler.run().await;
    });
}
```

**Step 3: 公开 ShiftAutoCloseScheduler::duration_until_next_cutoff**

把 `shifts.rs` 中的 `duration_until_next_cutoff` 改为 `pub` (目前无 pub)，供 DailyReportScheduler 复用。

**Step 4: Commit**

```
feat(edge): add DailyReportScheduler — auto-generate + catchup + 30-day cleanup
```

---

## Task 5: 班次现金退款扣减

**Files:**
- Modify: `edge-server/src/db/repository/shift.rs`
- Modify: `edge-server/src/archiving/credit_note.rs` (调用 deduct)

**Step 1: 新增 deduct_cash_refund**

在 `shift.rs` 的 `add_cash_payment` 后面添加:

```rust
/// 现金退款扣减班次预期现金
pub async fn deduct_cash_refund(pool: &SqlitePool, amount: f64) -> RepoResult<()> {
    let now = shared::util::now_millis();
    sqlx::query!(
        "UPDATE shift SET expected_cash = expected_cash - ?1, last_active_at = ?2, updated_at = ?2 WHERE status = 'OPEN'",
        amount,
        now,
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

**Step 2: 在退款流程中调用**

找到 `credit_note.rs` 中创建退款凭证的位置，在退款方式为 CASH 时调用:

```rust
// 在 credit_note 创建成功后
if refund_method == "CASH" {
    if let Err(e) = shift::deduct_cash_refund(&pool, total_credit).await {
        tracing::warn!("Failed to deduct cash refund from shift: {}", e);
    }
}
```

**Step 3: Commit**

```
fix(edge): deduct cash refunds from shift expected_cash
```

---

## Task 6: 手动生成 API 适配

**Files:**
- Modify: `edge-server/src/api/daily_reports/handler.rs`

**Step 1: 更新 generate handler**

给 `generate()` 调用加上 `auto_generated: false` 参数。

**Step 2: Commit**

```
refactor(edge): pass auto_generated flag to daily report generate
```

---

## Task 7: Cloud PG migration + sync 适配

**Files:**
- Cloud PG migration (手动 SQL 或新增迁移文件)
- Modify: `crab-cloud/src/db/store/daily_report.rs`
- Modify: `crab-cloud/src/db/tenant_queries.rs`

**Step 1: PG schema 变更**

```sql
-- 删除不再需要的表
DROP TABLE IF EXISTS store_daily_report_tax_breakdown;
DROP TABLE IF EXISTS store_daily_report_payment_breakdown;

-- 重建 store_daily_reports
-- (保守方案: ALTER TABLE ADD/DROP COLUMN)
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS completed_orders;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS void_orders;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS total_sales;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS total_paid;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS total_unpaid;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS void_amount;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS total_tax;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS total_discount;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS total_surcharge;

ALTER TABLE store_daily_reports ADD COLUMN IF NOT EXISTS net_revenue DOUBLE PRECISION NOT NULL DEFAULT 0.0;
ALTER TABLE store_daily_reports ADD COLUMN IF NOT EXISTS refund_amount DOUBLE PRECISION NOT NULL DEFAULT 0.0;
ALTER TABLE store_daily_reports ADD COLUMN IF NOT EXISTS refund_count BIGINT NOT NULL DEFAULT 0;
ALTER TABLE store_daily_reports ADD COLUMN IF NOT EXISTS auto_generated BOOLEAN NOT NULL DEFAULT FALSE;
```

**Step 2: 更新 upsert_daily_report_from_sync()**

适配新字段，删除 tax/payment breakdown 的 DELETE + INSERT 逻辑。

**Step 3: 更新 tenant_queries**

- `list_daily_reports`: SELECT 新字段
- `get_daily_report_detail`: 只查 shift_breakdowns，删除 tax/payment 并发查询

**Step 4: Commit**

```
refactor(cloud): adapt daily report schema + sync to simplified model
```

---

## Task 8: RedCoral 前端 — 日报详情精简

**Files:**
- Modify: `red_coral/src/features/daily-report/DailyReportDetailModal.tsx`
- Modify: `red_coral/src/core/domain/types/api/models.ts` (DailyReport type)
- Modify: i18n 翻译文件

**Step 1: 更新 TypeScript DailyReport 类型**

```typescript
export interface DailyReport {
  id: number;
  business_date: string;
  net_revenue: number;
  total_orders: number;
  refund_amount: number;
  refund_count: number;
  auto_generated: boolean;
  generated_at: number | null;
  generated_by_id: number | null;
  generated_by_name: string | null;
  note: string | null;
  shift_breakdowns: ShiftBreakdown[];
}
// 删除 TaxBreakdown, PaymentMethodBreakdown
```

**Step 2: 重写 DailyReportDetailModal**

删除: 销售汇总卡片、订单统计、税率分解、支付方式分解
保留: Header + 精简摘要 (净收入/订单/退款一行) + 班次对账列表
新增: 班次对账卡片 (每个班次的现金对账信息)

**Step 3: 更新列表展示**

DailyReportManagement 列表行：日期 + 净收入 + 订单数 + 退款 + 自动/手动标记

**Step 4: Commit**

```
refactor(redcoral): simplify daily report UI — shift settlement focus
```

---

## Task 9: Console 前端适配

**Files:**
- Modify: `crab-console/src/core/types/stats.ts`
- Modify: `crab-console/src/screens/Store/Reports/ReportsScreen.tsx`
- Modify: `crab-console/src/screens/Store/Reports/ReportDetailScreen.tsx`

**Step 1: 更新类型**

对齐新的 DailyReport 字段。

**Step 2: 更新列表页**

展示: 日期 + 净收入 + 订单数 + 退款 + 自动/手动标记

**Step 3: 更新详情页**

删除税率/支付方式分解，重点展示班次对账卡片。

**Step 4: Commit**

```
refactor(console): adapt reports screens to simplified daily report model
```

---

## Task 10: 验证 + sqlx prepare

**Step 1: 运行 Edge 编译检查**

```bash
cargo check --workspace
cargo clippy --workspace
```

**Step 2: TypeScript 类型检查**

```bash
cd red_coral && npx tsc --noEmit
```

**Step 3: 更新 sqlx 离线元数据**

```bash
cargo sqlx prepare --workspace
```

**Step 4: Final commit**

```
chore: sqlx prepare after daily report schema changes
```
