-- Fix INTEGER columns that should be BIGINT to match Rust i64 models

-- store_employees.role_id: shared Employee.role_id is i64
ALTER TABLE store_employees ALTER COLUMN role_id TYPE BIGINT;

-- store_daily_reports: count columns used as i64 in shared DailyReport
ALTER TABLE store_daily_reports ALTER COLUMN total_orders TYPE BIGINT;
ALTER TABLE store_daily_reports ALTER COLUMN completed_orders TYPE BIGINT;
ALTER TABLE store_daily_reports ALTER COLUMN void_orders TYPE BIGINT;

-- store_daily_report_tax_breakdown.order_count: shared TaxBreakdown.order_count is i64
ALTER TABLE store_daily_report_tax_breakdown ALTER COLUMN order_count TYPE BIGINT;

-- store_daily_report_payment_breakdown.count: shared PaymentMethodBreakdown.count is i64
ALTER TABLE store_daily_report_payment_breakdown ALTER COLUMN count TYPE BIGINT;

-- store_daily_report_shift_breakdown: count columns used as i64 in shared ShiftBreakdown
ALTER TABLE store_daily_report_shift_breakdown ALTER COLUMN total_orders TYPE BIGINT;
ALTER TABLE store_daily_report_shift_breakdown ALTER COLUMN completed_orders TYPE BIGINT;
ALTER TABLE store_daily_report_shift_breakdown ALTER COLUMN void_orders TYPE BIGINT;

-- Remove duplicate index on subscriptions (idx_subscriptions_tenant and idx_subscriptions_tenant_id are identical)
DROP INDEX IF EXISTS idx_subscriptions_tenant;

-- Add missing index on store_price_rules
CREATE INDEX IF NOT EXISTS idx_store_price_rules_edge ON store_price_rules(edge_server_id);

-- Fix store_employees.created_at DEFAULT 0 (nonsensical default)
ALTER TABLE store_employees ALTER COLUMN created_at DROP DEFAULT;

-- Unify money columns: NUMERIC(12,2) → DOUBLE PRECISION (consistent with total, total_sales, etc.)
ALTER TABLE store_archived_orders ALTER COLUMN tax TYPE DOUBLE PRECISION;
ALTER TABLE store_archived_orders ALTER COLUMN discount_amount TYPE DOUBLE PRECISION;
ALTER TABLE store_archived_orders ALTER COLUMN loss_amount TYPE DOUBLE PRECISION;
ALTER TABLE store_order_items ALTER COLUMN line_total TYPE DOUBLE PRECISION;
ALTER TABLE store_order_payments ALTER COLUMN amount TYPE DOUBLE PRECISION;
