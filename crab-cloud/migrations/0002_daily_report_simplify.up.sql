-- Simplify store_daily_reports to match edge schema changes

-- Drop removed breakdown tables
DROP TABLE IF EXISTS store_daily_report_tax_breakdown;
DROP TABLE IF EXISTS store_daily_report_payment_breakdown;

-- Add new columns
ALTER TABLE store_daily_reports ADD COLUMN net_revenue DOUBLE PRECISION NOT NULL DEFAULT 0.0;
ALTER TABLE store_daily_reports ADD COLUMN refund_amount DOUBLE PRECISION NOT NULL DEFAULT 0.0;
ALTER TABLE store_daily_reports ADD COLUMN refund_count BIGINT NOT NULL DEFAULT 0;
ALTER TABLE store_daily_reports ADD COLUMN auto_generated BOOLEAN NOT NULL DEFAULT FALSE;

-- Drop old columns (PG supports DROP COLUMN)
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS completed_orders;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS void_orders;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS total_sales;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS total_paid;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS total_unpaid;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS void_amount;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS total_tax;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS total_discount;
ALTER TABLE store_daily_reports DROP COLUMN IF EXISTS total_surcharge;
