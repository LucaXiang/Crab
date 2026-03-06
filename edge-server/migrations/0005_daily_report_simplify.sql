-- Simplify daily_report: remove duplicated aggregates, keep shift breakdowns
-- Daily report becomes a "shift settlement record" with summary snapshot.
-- Detailed statistics are served by the overview (StoreOverview) API.

-- 1. Drop redundant breakdown tables (data duplicated by overview queries)
DROP TABLE IF EXISTS daily_report_tax_breakdown;
DROP TABLE IF EXISTS daily_report_payment_breakdown;

-- 2. Rebuild daily_report with simplified schema
--    SQLite doesn't support DROP COLUMN, so recreate the table.
CREATE TABLE daily_report_new (
    id                INTEGER PRIMARY KEY,
    business_date     TEXT    NOT NULL,
    -- Summary snapshot (for list display and future push notifications)
    net_revenue       REAL    NOT NULL DEFAULT 0.0,
    total_orders      INTEGER NOT NULL DEFAULT 0,
    refund_amount     REAL    NOT NULL DEFAULT 0.0,
    refund_count      INTEGER NOT NULL DEFAULT 0,
    -- Metadata
    auto_generated    INTEGER NOT NULL DEFAULT 0,
    generated_at      INTEGER,
    generated_by_id   INTEGER,
    generated_by_name TEXT,
    note              TEXT,
    created_at        INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at        INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
);

-- Migrate existing data (best-effort: use total_sales as net_revenue approximation)
INSERT INTO daily_report_new (
    id, business_date, net_revenue, total_orders,
    generated_at, generated_by_id, generated_by_name, note,
    created_at, updated_at
)
SELECT
    id, business_date,
    COALESCE(total_sales, 0.0),
    COALESCE(total_orders, 0),
    generated_at, generated_by_id, generated_by_name, note,
    COALESCE(generated_at, unixepoch() * 1000),
    COALESCE(generated_at, unixepoch() * 1000)
FROM daily_report;

DROP TABLE daily_report;
ALTER TABLE daily_report_new RENAME TO daily_report;
CREATE UNIQUE INDEX idx_daily_report_date ON daily_report(business_date);

-- 3. Fix shift_breakdown FK: shift_id=0 is used for "unlinked" orders,
--    so the FK to shift(id) is wrong. Recreate without the FK constraint.
CREATE TABLE daily_report_shift_breakdown_new (
    id              INTEGER PRIMARY KEY,
    report_id       INTEGER NOT NULL REFERENCES daily_report(id) ON DELETE CASCADE,
    shift_id        INTEGER NOT NULL DEFAULT 0,
    operator_id     INTEGER NOT NULL,
    operator_name   TEXT    NOT NULL,
    status          TEXT    NOT NULL,
    start_time      INTEGER NOT NULL,
    end_time        INTEGER,
    starting_cash   REAL    NOT NULL DEFAULT 0.0,
    expected_cash   REAL    NOT NULL DEFAULT 0.0,
    actual_cash     REAL,
    cash_variance   REAL,
    abnormal_close  INTEGER NOT NULL DEFAULT 0,
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

INSERT INTO daily_report_shift_breakdown_new
SELECT * FROM daily_report_shift_breakdown;

DROP TABLE daily_report_shift_breakdown;
ALTER TABLE daily_report_shift_breakdown_new RENAME TO daily_report_shift_breakdown;
CREATE INDEX idx_shift_breakdown_report ON daily_report_shift_breakdown(report_id);
