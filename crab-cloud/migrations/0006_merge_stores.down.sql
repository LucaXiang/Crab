-- Reverse: stores → edge_servers + store_info

-- 1. Recreate store_info table
CREATE TABLE IF NOT EXISTS store_info (
    id                   BIGSERIAL PRIMARY KEY,
    edge_server_id       BIGINT NOT NULL UNIQUE,
    name                 TEXT NOT NULL DEFAULT '',
    address              TEXT NOT NULL DEFAULT '',
    nif                  TEXT NOT NULL DEFAULT '',
    logo_url             TEXT,
    phone                TEXT,
    email                TEXT,
    website              TEXT,
    business_day_cutoff  TEXT NOT NULL DEFAULT '00:00',
    created_at           BIGINT,
    updated_at           BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_store_info_edge ON store_info(edge_server_id);

-- 2. Rename FK columns back (store_id → edge_server_id)
ALTER TABLE store_pending_ops RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_label_templates RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_employees RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_shifts RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_commands RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_daily_report_payment_breakdown RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_daily_report_tax_breakdown RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_daily_reports RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_archived_orders RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_dining_tables RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_zones RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_versions RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_price_rules RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_category_tag RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_attribute_bindings RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_attribute_options RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_attributes RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_product_specs RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_products RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_categories RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_tags RENAME COLUMN store_id TO edge_server_id;
ALTER TABLE store_sync_cursors RENAME COLUMN store_id TO edge_server_id;

-- 3. Rename table back
ALTER INDEX idx_stores_tenant RENAME TO idx_edge_servers_tenant;
ALTER TABLE stores RENAME TO edge_servers;

-- 4. Copy data to store_info from edge_servers
INSERT INTO store_info (edge_server_id, name, address, nif, logo_url, phone, email, website, business_day_cutoff, created_at, updated_at)
SELECT id, COALESCE(name, ''), COALESCE(address, ''), COALESCE(nif, ''), logo_url, phone, email, website,
       COALESCE(business_day_cutoff, '00:00'), created_at, COALESCE(updated_at, 0)
FROM edge_servers
WHERE updated_at IS NOT NULL;

-- 5. Add FK constraint
ALTER TABLE store_info ADD CONSTRAINT store_info_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

-- 6. Drop extra columns from edge_servers
ALTER TABLE edge_servers DROP COLUMN IF EXISTS logo_url;
ALTER TABLE edge_servers DROP COLUMN IF EXISTS created_at;
ALTER TABLE edge_servers DROP COLUMN IF EXISTS updated_at;
