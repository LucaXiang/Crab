-- Desglose: change DOUBLE PRECISION → NUMERIC(12,2) to preserve Decimal precision
-- through the Decimal → PG → Decimal roundtrip (Verifactu tax data).

ALTER TABLE store_order_desglose ALTER COLUMN base_amount TYPE NUMERIC(12,2);
ALTER TABLE store_order_desglose ALTER COLUMN tax_amount TYPE NUMERIC(12,2);

ALTER TABLE store_invoice_desglose ALTER COLUMN base_amount TYPE NUMERIC(12,2);
ALTER TABLE store_invoice_desglose ALTER COLUMN tax_amount TYPE NUMERIC(12,2);

-- Overview query conditional index (covers 14 concurrent analytics queries)
CREATE INDEX idx_sao_overview
    ON store_archived_orders(tenant_id, store_id, end_time)
    WHERE status = 'COMPLETED';

-- Active payments index for payment breakdown queries
CREATE INDEX idx_sop_active
    ON store_order_payments(order_id, method, amount)
    WHERE cancelled IS NOT TRUE;
