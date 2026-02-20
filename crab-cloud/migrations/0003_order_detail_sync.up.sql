-- 1. cloud_archived_orders: add order_key, tax, prev_hash, curr_hash; drop data column
ALTER TABLE cloud_archived_orders ADD COLUMN order_key TEXT;
ALTER TABLE cloud_archived_orders ADD COLUMN tax NUMERIC(12,2);
ALTER TABLE cloud_archived_orders ADD COLUMN prev_hash TEXT;
ALTER TABLE cloud_archived_orders ADD COLUMN curr_hash TEXT;

-- Backfill order_key from source_id for existing data
UPDATE cloud_archived_orders SET order_key = source_id WHERE order_key IS NULL;
ALTER TABLE cloud_archived_orders ALTER COLUMN order_key SET NOT NULL;

-- Drop legacy data JSONB column (summary fields now in dedicated columns)
ALTER TABLE cloud_archived_orders DROP COLUMN data;

-- Replace old unique constraint with three-layer key
ALTER TABLE cloud_archived_orders DROP CONSTRAINT cloud_archived_orders_edge_server_id_source_id_key;
CREATE UNIQUE INDEX uq_cloud_archived_orders_key
    ON cloud_archived_orders (tenant_id, edge_server_id, order_key);

-- 2. Tax breakdown table (VeriFactu desglose, permanent)
CREATE TABLE cloud_order_desglose (
    id BIGSERIAL PRIMARY KEY,
    archived_order_id BIGINT NOT NULL REFERENCES cloud_archived_orders(id) ON DELETE CASCADE,
    tax_rate INTEGER NOT NULL,
    base_amount NUMERIC(12,2) NOT NULL,
    tax_amount NUMERIC(12,2) NOT NULL,
    UNIQUE (archived_order_id, tax_rate)
);

-- 3. Order detail table (30-day rolling, JSONB)
CREATE TABLE cloud_order_details (
    id BIGSERIAL PRIMARY KEY,
    archived_order_id BIGINT NOT NULL REFERENCES cloud_archived_orders(id) ON DELETE CASCADE,
    detail JSONB NOT NULL,
    synced_at BIGINT NOT NULL,
    UNIQUE (archived_order_id)
);

CREATE INDEX idx_cloud_order_details_synced_at ON cloud_order_details (synced_at);

-- Composite index for tenant order listing performance
CREATE INDEX idx_cloud_archived_orders_list
    ON cloud_archived_orders (edge_server_id, tenant_id, status, end_time DESC);
