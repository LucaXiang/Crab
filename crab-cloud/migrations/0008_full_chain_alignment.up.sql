-- Full chain alignment: add missing columns, fix types, complete NUMERIC migration.
-- Run with clean database (DROP pgdata + Edge re-sync).

-- ═══════════════════════════════════════════════════════════════
-- 1. store_archived_orders: add missing columns + NUMERIC
-- ═══════════════════════════════════════════════════════════════

-- Missing columns
ALTER TABLE store_archived_orders ADD COLUMN created_at BIGINT;
ALTER TABLE store_archived_orders ADD COLUMN queue_number TEXT;
ALTER TABLE store_archived_orders ADD COLUMN shift_id BIGINT;
ALTER TABLE store_archived_orders ADD COLUMN operator_id BIGINT;
ALTER TABLE store_archived_orders ADD COLUMN member_id BIGINT;

-- Fix source_id TEXT → BIGINT (clean DB, no data to migrate)
ALTER TABLE store_archived_orders ALTER COLUMN source_id TYPE BIGINT USING source_id::BIGINT;

-- Monetary DOUBLE PRECISION → NUMERIC(12,2)
ALTER TABLE store_archived_orders ALTER COLUMN total TYPE NUMERIC(12,2);
ALTER TABLE store_archived_orders ALTER COLUMN tax TYPE NUMERIC(12,2);
ALTER TABLE store_archived_orders ALTER COLUMN discount_amount TYPE NUMERIC(12,2);
ALTER TABLE store_archived_orders ALTER COLUMN loss_amount TYPE NUMERIC(12,2);
ALTER TABLE store_archived_orders ALTER COLUMN original_total TYPE NUMERIC(12,2);
ALTER TABLE store_archived_orders ALTER COLUMN subtotal TYPE NUMERIC(12,2);
ALTER TABLE store_archived_orders ALTER COLUMN paid_amount TYPE NUMERIC(12,2);
ALTER TABLE store_archived_orders ALTER COLUMN surcharge_amount TYPE NUMERIC(12,2);
ALTER TABLE store_archived_orders ALTER COLUMN comp_total_amount TYPE NUMERIC(12,2);
ALTER TABLE store_archived_orders ALTER COLUMN order_manual_discount_amount TYPE NUMERIC(12,2);
ALTER TABLE store_archived_orders ALTER COLUMN order_manual_surcharge_amount TYPE NUMERIC(12,2);
ALTER TABLE store_archived_orders ALTER COLUMN order_rule_discount_amount TYPE NUMERIC(12,2);
ALTER TABLE store_archived_orders ALTER COLUMN order_rule_surcharge_amount TYPE NUMERIC(12,2);

-- Fix service_type default: NULL instead of 'DineIn'
ALTER TABLE store_archived_orders ALTER COLUMN service_type DROP DEFAULT;

-- ═══════════════════════════════════════════════════════════════
-- 2. store_order_items: NUMERIC
-- ═══════════════════════════════════════════════════════════════

ALTER TABLE store_order_items ALTER COLUMN price TYPE NUMERIC(12,2);
ALTER TABLE store_order_items ALTER COLUMN unit_price TYPE NUMERIC(12,2);
ALTER TABLE store_order_items ALTER COLUMN line_total TYPE NUMERIC(12,2);
ALTER TABLE store_order_items ALTER COLUMN discount_amount TYPE NUMERIC(12,2);
ALTER TABLE store_order_items ALTER COLUMN surcharge_amount TYPE NUMERIC(12,2);
ALTER TABLE store_order_items ALTER COLUMN tax TYPE NUMERIC(12,2);

-- ═══════════════════════════════════════════════════════════════
-- 3. store_order_item_options: NUMERIC
-- ═══════════════════════════════════════════════════════════════

ALTER TABLE store_order_item_options ALTER COLUMN price TYPE NUMERIC(12,2);

-- ═══════════════════════════════════════════════════════════════
-- 4. store_order_payments: add columns + NUMERIC
-- ═══════════════════════════════════════════════════════════════

ALTER TABLE store_order_payments ADD COLUMN cancel_reason TEXT;
ALTER TABLE store_order_payments ADD COLUMN tendered NUMERIC(12,2);
ALTER TABLE store_order_payments ADD COLUMN change_amount NUMERIC(12,2);
ALTER TABLE store_order_payments ALTER COLUMN amount TYPE NUMERIC(12,2);

-- ═══════════════════════════════════════════════════════════════
-- 5. store_credit_notes: NUMERIC
-- ═══════════════════════════════════════════════════════════════

ALTER TABLE store_credit_notes ALTER COLUMN subtotal_credit TYPE NUMERIC(12,2);
ALTER TABLE store_credit_notes ALTER COLUMN tax_credit TYPE NUMERIC(12,2);
ALTER TABLE store_credit_notes ALTER COLUMN total_credit TYPE NUMERIC(12,2);

-- ═══════════════════════════════════════════════════════════════
-- 6. store_credit_note_items: add column + NUMERIC
-- ═══════════════════════════════════════════════════════════════

ALTER TABLE store_credit_note_items ADD COLUMN original_instance_id TEXT;
ALTER TABLE store_credit_note_items ALTER COLUMN unit_price TYPE NUMERIC(12,2);
ALTER TABLE store_credit_note_items ALTER COLUMN line_credit TYPE NUMERIC(12,2);
ALTER TABLE store_credit_note_items ALTER COLUMN tax_credit TYPE NUMERIC(12,2);

-- ═══════════════════════════════════════════════════════════════
-- 7. store_invoices: add column + NUMERIC
-- ═══════════════════════════════════════════════════════════════

ALTER TABLE store_invoices ADD COLUMN fecha_hora_registro TEXT;
ALTER TABLE store_invoices ALTER COLUMN subtotal TYPE NUMERIC(12,2);
ALTER TABLE store_invoices ALTER COLUMN tax TYPE NUMERIC(12,2);
ALTER TABLE store_invoices ALTER COLUMN total TYPE NUMERIC(12,2);
