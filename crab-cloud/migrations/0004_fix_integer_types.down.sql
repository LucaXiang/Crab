-- Revert BIGINT columns back to INTEGER

ALTER TABLE store_employees ALTER COLUMN role_id TYPE INTEGER;

ALTER TABLE store_daily_reports ALTER COLUMN total_orders TYPE INTEGER;
ALTER TABLE store_daily_reports ALTER COLUMN completed_orders TYPE INTEGER;
ALTER TABLE store_daily_reports ALTER COLUMN void_orders TYPE INTEGER;

ALTER TABLE store_daily_report_tax_breakdown ALTER COLUMN order_count TYPE INTEGER;

ALTER TABLE store_daily_report_payment_breakdown ALTER COLUMN count TYPE INTEGER;

ALTER TABLE store_daily_report_shift_breakdown ALTER COLUMN total_orders TYPE INTEGER;
ALTER TABLE store_daily_report_shift_breakdown ALTER COLUMN completed_orders TYPE INTEGER;
ALTER TABLE store_daily_report_shift_breakdown ALTER COLUMN void_orders TYPE INTEGER;

CREATE INDEX IF NOT EXISTS idx_subscriptions_tenant ON subscriptions(tenant_id);

DROP INDEX IF EXISTS idx_store_price_rules_edge;

ALTER TABLE store_employees ALTER COLUMN created_at SET DEFAULT 0;

-- Revert money columns back to NUMERIC(12,2)
ALTER TABLE store_archived_orders ALTER COLUMN tax TYPE NUMERIC(12,2);
ALTER TABLE store_archived_orders ALTER COLUMN discount_amount TYPE NUMERIC(12,2);
ALTER TABLE store_archived_orders ALTER COLUMN loss_amount TYPE NUMERIC(12,2);
ALTER TABLE store_order_items ALTER COLUMN line_total TYPE NUMERIC(12,2);
ALTER TABLE store_order_payments ALTER COLUMN amount TYPE NUMERIC(12,2);
