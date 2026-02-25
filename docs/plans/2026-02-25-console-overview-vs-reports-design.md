# Console 经营概览 vs 日报 设计

**日期**: 2026-02-25
**状态**: Approved

## 背景

Console 现有 Overview（今日概览）和 Stats（日报列表/详情），但存在概念混乱：
- Stats 命名不准确，实际是日报
- 日报只有店铺级汇总，缺少班次维度
- archived_order 没有 shift_id，无法按班次聚合
- 日报详情页复用了 Overview API（实时聚合），没有自己的数据源

## 目标

1. **概览 (Overview)** = 店铺级宏观视图（今日实时汇总 + 趋势 + 排行），数据源：`store_archived_orders` 实时 JSONB 聚合
2. **日报 (Reports)** = 班次级明细视图（每日班次卡片 + 全面统计），数据源：`store_daily_reports` + breakdown 子表
3. 两者概念清晰，互不重叠

## 设计

### 1. 路由重命名

`stats` → `reports`，所有相关文件、路由、i18n key 统一改名。

```
/stores/:id/overview          → 经营概览（保持不变）
/stores/:id/reports           → 日报列表（原 stats）
/stores/:id/reports/:date     → 日报详情（原 stats/:date，重构为班次卡片）
```

### 2. archived_order 添加 shift_id

新增 Edge migration：`ALTER TABLE archived_order ADD COLUMN shift_id INTEGER REFERENCES shift(id)`

归档订单时（ArchiveWorker），在调用 `archive_service.archive_order()` 前，通过 `shift::find_any_open()` 查询当前 OPEN 班次的 ID 并传入。系统保证同时只有一个 OPEN 班次。

如果没有开放班次（归档重试场景），shift_id 为 NULL。

Cloud 端 `store_archived_orders` 暂不加 shift_id（overview 页面不需要按班次切分，日报走 breakdown 子表）。

### 3. 新增 daily_report_shift_breakdown 子表

跟现有 tax_breakdown / payment_breakdown 模式一致。

#### Edge SQLite

```sql
CREATE TABLE daily_report_shift_breakdown (
    id              INTEGER PRIMARY KEY,
    report_id       INTEGER NOT NULL REFERENCES daily_report(id) ON DELETE CASCADE,
    shift_id        INTEGER NOT NULL REFERENCES shift(id),
    operator_id     INTEGER NOT NULL,
    operator_name   TEXT    NOT NULL,
    status          TEXT    NOT NULL,         -- OPEN / CLOSED
    start_time      INTEGER NOT NULL,
    end_time        INTEGER,
    starting_cash   REAL    NOT NULL DEFAULT 0.0,
    expected_cash   REAL    NOT NULL DEFAULT 0.0,
    actual_cash     REAL,
    cash_variance   REAL,
    abnormal_close  INTEGER NOT NULL DEFAULT 0,
    -- 聚合统计
    total_orders      INTEGER NOT NULL DEFAULT 0,
    completed_orders  INTEGER NOT NULL DEFAULT 0,
    void_orders       INTEGER NOT NULL DEFAULT 0,
    total_sales       REAL NOT NULL DEFAULT 0.0,
    total_paid        REAL NOT NULL DEFAULT 0.0,
    void_amount       REAL NOT NULL DEFAULT 0.0,
    total_tax         REAL NOT NULL DEFAULT 0.0,
    total_discount    REAL NOT NULL DEFAULT 0.0,
    total_surcharge   REAL NOT NULL DEFAULT 0.0
);
CREATE INDEX idx_shift_breakdown_report ON daily_report_shift_breakdown(report_id);
```

#### Cloud PostgreSQL

```sql
CREATE TABLE store_daily_report_shift_breakdown (
    id               BIGSERIAL PRIMARY KEY,
    report_id        BIGINT NOT NULL REFERENCES store_daily_reports(id) ON DELETE CASCADE,
    shift_source_id  BIGINT NOT NULL,
    operator_id      BIGINT NOT NULL,
    operator_name    TEXT NOT NULL,
    status           TEXT NOT NULL,
    start_time       BIGINT NOT NULL,
    end_time         BIGINT,
    starting_cash    DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    expected_cash    DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    actual_cash      DOUBLE PRECISION,
    cash_variance    DOUBLE PRECISION,
    abnormal_close   BOOLEAN NOT NULL DEFAULT FALSE,
    total_orders     INTEGER NOT NULL DEFAULT 0,
    completed_orders INTEGER NOT NULL DEFAULT 0,
    void_orders      INTEGER NOT NULL DEFAULT 0,
    total_sales      DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    total_paid       DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    void_amount      DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    total_tax        DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    total_discount   DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    total_surcharge  DOUBLE PRECISION NOT NULL DEFAULT 0.0
);
```

### 4. 日报生成逻辑扩展

在 `daily_report::generate()` 的事务中，新增第5步：

```
Step 5: 按 shift_id 从 archived_order 聚合班次统计
  - SELECT shift_id, COUNT(*), SUM(total_amount), ... FROM archived_order
    WHERE end_time >= ? AND end_time < ? GROUP BY shift_id
  - 对每个 shift_id，JOIN shift 表获取元信息（operator, cash_variance 等）
  - 写入 daily_report_shift_breakdown
```

shift_id 为 NULL 的订单归入"未关联班次"组（operator_name = "未关联班次"，shift_id = 0）。

### 5. Shared 类型扩展

```rust
// shared/src/models/daily_report.rs

/// Shift breakdown within a daily report
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

// DailyReport 新增字段
pub struct DailyReport {
    // ... 现有字段 ...
    #[cfg_attr(feature = "db", sqlx(skip))]
    #[serde(default)]
    pub shift_breakdowns: Vec<ShiftBreakdown>,
}
```

### 6. Cloud API

#### 新增日报详情端点

`GET /api/tenant/stores/:id/reports/:date` — 返回完整 DailyReport（含 shift/tax/payment breakdowns）。

现有 `StatsDetailScreen` 调用的是 `getStoreOverview()`（实时 JSONB 聚合），新设计需要用日报数据，所以必须新增此端点。

实现：`tenant_queries::get_daily_report_by_date(pool, edge_server_id, tenant_id, date)` — 查询 `store_daily_reports` 主表 + 三张 breakdown 子表。

#### 现有端点保持不变

- `GET /api/tenant/stores/:id/stats?from=&to=` — 日报列表（返回 `DailyReportEntry` 摘要）
- `GET /api/tenant/stores/:id/overview?from=&to=` — 经营概览（实时 JSONB 聚合）

#### Cloud 同步扩展

`upsert_daily_report_from_sync()` 事务内新增第4步：DELETE + INSERT `store_daily_report_shift_breakdown`，跟 tax/payment breakdown 同样模式。

### 7. Console 前端

#### 7.1 重命名

- 目录 `screens/Store/Stats/` → `screens/Store/Reports/`
- `StatsScreen` → `ReportsScreen`
- `StatsDetailScreen` → `ReportDetailScreen`
- 路由 `stats` → `reports`
- i18n namespace `stats.*` 保持不变（避免大面积改动，Overview 页面也用 `stats.*`）
- API 文件 `stats.ts` 中新增 `getReportDetail(token, storeId, date)` 函数
- 类型文件新增 `DailyReportDetail` 类型（含 shift_breakdowns）

注意：`stats.ts` 类型文件中还有 RedFlags 相关类型，暂不拆分。

#### 7.2 日报详情页重构

`/stores/:id/reports/:date` 页面结构：

```
┌─────────────────────────────────────────────┐
│  📅 2026-02-24 (周二)     生成人: 张经理      │
│                                             │
│  ┌─ 班次 1 ──────────────────────────────┐  │
│  │ 操作人: 李明  08:00 - 16:30  ✅ 已收班  │  │
│  │                                       │  │
│  │ 营业额  ¥3,200   订单数  45           │  │
│  │ 作废    ¥120(2单) 折扣    ¥85         │  │
│  │ 附加费  ¥30       税额    ¥416        │  │
│  │                                       │  │
│  │ 现金: 期初 ¥500 → 应有 ¥1,820        │  │
│  │       实际 ¥1,800  差异 -¥20 ⚠️       │  │
│  └───────────────────────────────────────┘  │
│                                             │
│  ┌─ 班次 2 ──────────────────────────────┐  │
│  │ 操作人: 王芳  16:30 - 23:00  ✅ 已收班  │  │
│  │ ...                                   │  │
│  └───────────────────────────────────────┘  │
│                                             │
│  ┌─ 当日总计 ────────────────────────────┐  │
│  │ 营业额 ¥6,800  订单 92  作废 ¥180     │  │
│  │ 现金差异合计 -¥20                     │  │
│  │ 支付方式汇总 / 税务汇总               │  │
│  └───────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

数据源：`getReportDetail(token, storeId, date)` → `GET /api/tenant/stores/:id/reports/:date`

班次卡片内容：
- **头部**：操作人、时间段、状态（开班中/已收班/异常收班⚠️）
- **核心指标**：营业额、订单数、作废（金额+单数）、折扣、附加费、税额
- **现金对账**：期初→应有→实际→差异（差异非零高亮）

当日总计区域：
- 使用 DailyReport 主表的汇总数据
- 支付方式明细（payment_breakdowns）
- 税务明细（tax_breakdowns）

### 8. 实现顺序

1. **Shared 类型**: 新增 ShiftBreakdown，扩展 DailyReport
2. **Edge migration**: archived_order 加 shift_id + 新建 shift_breakdown 表
3. **Edge 归档逻辑**: ArchiveWorker 传入 shift_id
4. **Edge 日报生成**: generate() 增加班次聚合步骤
5. **Edge 日报查询**: find_by_id / batch_load 加载 shift_breakdowns
6. **Cloud migration**: store_daily_report_shift_breakdown 表
7. **Cloud 同步**: upsert_daily_report_from_sync 扩展
8. **Cloud API**: 新增 GET /api/tenant/stores/:id/reports/:date 端点
9. **Console 重命名**: stats → reports（路由、组件、目录）
10. **Console 日报详情页**: 用新 API 重构为班次卡片布局
