-- Reverse: re-add old columns, drop new ones
ALTER TABLE store_daily_reports ADD COLUMN completed_orders BIGINT NOT NULL DEFAULT 0;
ALTER TABLE store_daily_reports ADD COLUMN void_orders BIGINT NOT NULL DEFAULT 0;
ALTER TABLE store_daily_reports ADD COLUMN total_sales DOUBLE PRECISION NOT NULL DEFAULT 0.0;
ALTER TABLE store_daily_reports ADD COLUMN total_paid DOUBLE PRECISION NOT NULL DEFAULT 0.0;
ALTER TABLE store_daily_reports ADD COLUMN total_unpaid DOUBLE PRECISION NOT NULL DEFAULT 0.0;
ALTER TABLE store_daily_reports ADD COLUMN void_amount DOUBLE PRECISION NOT NULL DEFAULT 0.0;
ALTER TABLE store_daily_reports ADD COLUMN total_tax DOUBLE PRECISION NOT NULL DEFAULT 0.0;
ALTER TABLE store_daily_reports ADD COLUMN total_discount DOUBLE PRECISION NOT NULL DEFAULT 0.0;
ALTER TABLE store_daily_reports ADD COLUMN total_surcharge DOUBLE PRECISION NOT NULL DEFAULT 0.0;

ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS net_revenue;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS refund_amount;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS refund_count;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS auto_generated;

CREATE TABLE IF NOT EXISTS store_daily_report_tax_breakdown (
    id           BIGSERIAL PRIMARY KEY,
    report_id    BIGINT NOT NULL REFERENCES store_daily_reports(id) ON DELETE CASCADE,
    tax_rate     INTEGER NOT NULL,
    net_amount   DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    tax_amount   DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    gross_amount DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    order_count  BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX idx_store_dr_tax_report ON store_daily_report_tax_breakdown(report_id);

CREATE TABLE IF NOT EXISTS store_daily_report_payment_breakdown (
    id        BIGSERIAL PRIMARY KEY,
    report_id BIGINT NOT NULL REFERENCES store_daily_reports(id) ON DELETE CASCADE,
    method    TEXT NOT NULL,
    amount    DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    count     BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX idx_store_dr_payment_report ON store_daily_report_payment_breakdown(report_id);
