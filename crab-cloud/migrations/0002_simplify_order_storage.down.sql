-- Rollback: recreate sub-tables and move detail back

-- 1. Recreate store_order_details
CREATE TABLE IF NOT EXISTS store_order_details (
    id BIGSERIAL PRIMARY KEY,
    archived_order_id BIGINT NOT NULL REFERENCES store_archived_orders(id) ON DELETE CASCADE,
    detail JSONB NOT NULL,
    synced_at BIGINT NOT NULL,
    UNIQUE (archived_order_id)
);
CREATE INDEX IF NOT EXISTS idx_store_order_details_synced_at ON store_order_details (synced_at);

-- Migrate detail back
INSERT INTO store_order_details (archived_order_id, detail, synced_at)
SELECT id, detail, synced_at
FROM store_archived_orders
WHERE detail IS NOT NULL;

-- Remove detail column
ALTER TABLE store_archived_orders DROP COLUMN IF EXISTS detail;

-- 2. Recreate other sub-tables (empty — data was already lost on up migration)
CREATE TABLE IF NOT EXISTS store_order_items (
    id BIGSERIAL PRIMARY KEY,
    archived_order_id BIGINT NOT NULL REFERENCES store_archived_orders(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    category_name TEXT,
    quantity INTEGER NOT NULL,
    line_total NUMERIC(12,2) NOT NULL,
    tax_rate INTEGER NOT NULL DEFAULT 0,
    product_source_id BIGINT
);
CREATE INDEX IF NOT EXISTS idx_store_order_items_order ON store_order_items (archived_order_id);
CREATE INDEX IF NOT EXISTS idx_store_order_items_product ON store_order_items (product_source_id);

CREATE TABLE IF NOT EXISTS store_order_payments (
    id BIGSERIAL PRIMARY KEY,
    archived_order_id BIGINT NOT NULL REFERENCES store_archived_orders(id) ON DELETE CASCADE,
    method TEXT NOT NULL,
    amount NUMERIC(12,2) NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_store_order_payments_order ON store_order_payments (archived_order_id);

CREATE TABLE IF NOT EXISTS store_order_events (
    id                BIGSERIAL PRIMARY KEY,
    archived_order_id BIGINT NOT NULL REFERENCES store_archived_orders(id) ON DELETE CASCADE,
    seq               INTEGER NOT NULL,
    event_type        TEXT NOT NULL,
    timestamp         BIGINT NOT NULL,
    operator_id       BIGINT,
    operator_name     TEXT
);
CREATE INDEX IF NOT EXISTS idx_store_order_events_order ON store_order_events(archived_order_id);
CREATE INDEX IF NOT EXISTS idx_store_order_events_red_flags ON store_order_events(event_type, timestamp, operator_id);
