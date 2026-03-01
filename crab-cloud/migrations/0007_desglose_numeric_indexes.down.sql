DROP INDEX IF EXISTS idx_sop_active;
DROP INDEX IF EXISTS idx_sao_overview;

ALTER TABLE store_invoice_desglose ALTER COLUMN tax_amount TYPE DOUBLE PRECISION;
ALTER TABLE store_invoice_desglose ALTER COLUMN base_amount TYPE DOUBLE PRECISION;

ALTER TABLE store_order_desglose ALTER COLUMN tax_amount TYPE DOUBLE PRECISION;
ALTER TABLE store_order_desglose ALTER COLUMN base_amount TYPE DOUBLE PRECISION;
