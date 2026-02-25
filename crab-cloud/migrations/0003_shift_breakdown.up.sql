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

CREATE INDEX idx_store_shift_breakdown_report ON store_daily_report_shift_breakdown(report_id);
