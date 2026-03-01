-- Reverse full chain alignment migration

-- store_invoices
ALTER TABLE store_invoices DROP COLUMN IF EXISTS fecha_hora_registro;
ALTER TABLE store_invoices ALTER COLUMN subtotal TYPE DOUBLE PRECISION;
ALTER TABLE store_invoices ALTER COLUMN tax TYPE DOUBLE PRECISION;
ALTER TABLE store_invoices ALTER COLUMN total TYPE DOUBLE PRECISION;

-- store_credit_note_items
ALTER TABLE store_credit_note_items DROP COLUMN IF EXISTS original_instance_id;
ALTER TABLE store_credit_note_items ALTER COLUMN unit_price TYPE DOUBLE PRECISION;
ALTER TABLE store_credit_note_items ALTER COLUMN line_credit TYPE DOUBLE PRECISION;
ALTER TABLE store_credit_note_items ALTER COLUMN tax_credit TYPE DOUBLE PRECISION;

-- store_credit_notes
ALTER TABLE store_credit_notes ALTER COLUMN subtotal_credit TYPE DOUBLE PRECISION;
ALTER TABLE store_credit_notes ALTER COLUMN tax_credit TYPE DOUBLE PRECISION;
ALTER TABLE store_credit_notes ALTER COLUMN total_credit TYPE DOUBLE PRECISION;

-- store_order_payments
ALTER TABLE store_order_payments DROP COLUMN IF EXISTS cancel_reason;
ALTER TABLE store_order_payments DROP COLUMN IF EXISTS tendered;
ALTER TABLE store_order_payments DROP COLUMN IF EXISTS change_amount;
ALTER TABLE store_order_payments ALTER COLUMN amount TYPE DOUBLE PRECISION;

-- store_order_item_options
ALTER TABLE store_order_item_options ALTER COLUMN price TYPE DOUBLE PRECISION;

-- store_order_items
ALTER TABLE store_order_items ALTER COLUMN price TYPE DOUBLE PRECISION;
ALTER TABLE store_order_items ALTER COLUMN unit_price TYPE DOUBLE PRECISION;
ALTER TABLE store_order_items ALTER COLUMN line_total TYPE DOUBLE PRECISION;
ALTER TABLE store_order_items ALTER COLUMN discount_amount TYPE DOUBLE PRECISION;
ALTER TABLE store_order_items ALTER COLUMN surcharge_amount TYPE DOUBLE PRECISION;
ALTER TABLE store_order_items ALTER COLUMN tax TYPE DOUBLE PRECISION;

-- store_archived_orders
ALTER TABLE store_archived_orders DROP COLUMN IF EXISTS created_at;
ALTER TABLE store_archived_orders DROP COLUMN IF EXISTS queue_number;
ALTER TABLE store_archived_orders DROP COLUMN IF EXISTS shift_id;
ALTER TABLE store_archived_orders DROP COLUMN IF EXISTS operator_id;
ALTER TABLE store_archived_orders DROP COLUMN IF EXISTS member_id;
ALTER TABLE store_archived_orders ALTER COLUMN source_id TYPE TEXT USING source_id::TEXT;
ALTER TABLE store_archived_orders ALTER COLUMN total TYPE DOUBLE PRECISION;
ALTER TABLE store_archived_orders ALTER COLUMN tax TYPE DOUBLE PRECISION;
ALTER TABLE store_archived_orders ALTER COLUMN discount_amount TYPE DOUBLE PRECISION;
ALTER TABLE store_archived_orders ALTER COLUMN loss_amount TYPE DOUBLE PRECISION;
ALTER TABLE store_archived_orders ALTER COLUMN original_total TYPE DOUBLE PRECISION;
ALTER TABLE store_archived_orders ALTER COLUMN subtotal TYPE DOUBLE PRECISION;
ALTER TABLE store_archived_orders ALTER COLUMN paid_amount TYPE DOUBLE PRECISION;
ALTER TABLE store_archived_orders ALTER COLUMN surcharge_amount TYPE DOUBLE PRECISION;
ALTER TABLE store_archived_orders ALTER COLUMN comp_total_amount TYPE DOUBLE PRECISION;
ALTER TABLE store_archived_orders ALTER COLUMN order_manual_discount_amount TYPE DOUBLE PRECISION;
ALTER TABLE store_archived_orders ALTER COLUMN order_manual_surcharge_amount TYPE DOUBLE PRECISION;
ALTER TABLE store_archived_orders ALTER COLUMN order_rule_discount_amount TYPE DOUBLE PRECISION;
ALTER TABLE store_archived_orders ALTER COLUMN order_rule_surcharge_amount TYPE DOUBLE PRECISION;
ALTER TABLE store_archived_orders ALTER COLUMN service_type SET DEFAULT 'DineIn';
