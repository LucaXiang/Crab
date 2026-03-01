-- Reverse denormalization: restore JSONB columns, drop relational tables.
-- Data loss: relational data cannot be automatically restored to JSONB.

-- Restore JSONB columns
ALTER TABLE store_archived_orders ADD COLUMN detail JSONB;
ALTER TABLE store_archived_orders ADD COLUMN desglose JSONB NOT NULL DEFAULT '[]'::JSONB;
ALTER TABLE store_credit_notes ADD COLUMN detail JSONB;
ALTER TABLE store_invoices ADD COLUMN detail JSONB;
ALTER TABLE store_anulaciones ADD COLUMN detail JSONB NOT NULL DEFAULT '{}'::JSONB;

-- Drop promoted scalar columns
ALTER TABLE store_archived_orders DROP COLUMN zone_name;
ALTER TABLE store_archived_orders DROP COLUMN table_name;
ALTER TABLE store_archived_orders DROP COLUMN is_retail;
ALTER TABLE store_archived_orders DROP COLUMN original_total;
ALTER TABLE store_archived_orders DROP COLUMN subtotal;
ALTER TABLE store_archived_orders DROP COLUMN paid_amount;
ALTER TABLE store_archived_orders DROP COLUMN surcharge_amount;
ALTER TABLE store_archived_orders DROP COLUMN comp_total_amount;
ALTER TABLE store_archived_orders DROP COLUMN order_manual_discount_amount;
ALTER TABLE store_archived_orders DROP COLUMN order_manual_surcharge_amount;
ALTER TABLE store_archived_orders DROP COLUMN order_rule_discount_amount;
ALTER TABLE store_archived_orders DROP COLUMN order_rule_surcharge_amount;
ALTER TABLE store_archived_orders DROP COLUMN operator_name;
ALTER TABLE store_archived_orders DROP COLUMN loss_reason;
ALTER TABLE store_archived_orders DROP COLUMN void_note;
ALTER TABLE store_archived_orders DROP COLUMN member_name;
ALTER TABLE store_archived_orders DROP COLUMN service_type;

-- Drop child tables
DROP TABLE IF EXISTS store_order_item_options;
DROP TABLE IF EXISTS store_order_items;
DROP TABLE IF EXISTS store_order_payments;
DROP TABLE IF EXISTS store_order_events;
DROP TABLE IF EXISTS store_order_desglose;
DROP TABLE IF EXISTS store_credit_note_items;
DROP TABLE IF EXISTS store_invoice_desglose;
