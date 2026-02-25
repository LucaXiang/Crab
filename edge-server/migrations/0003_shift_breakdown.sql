-- Add shift_id to archived_order
ALTER TABLE archived_order ADD COLUMN shift_id INTEGER REFERENCES shift(id);
CREATE INDEX idx_archived_order_shift ON archived_order(shift_id);

-- Shift breakdown for daily reports
CREATE TABLE daily_report_shift_breakdown (
    id              INTEGER PRIMARY KEY,
    report_id       INTEGER NOT NULL REFERENCES daily_report(id) ON DELETE CASCADE,
    shift_id        INTEGER NOT NULL REFERENCES shift(id),
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
CREATE INDEX idx_shift_breakdown_report ON daily_report_shift_breakdown(report_id);
