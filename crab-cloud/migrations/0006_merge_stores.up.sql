-- Merge edge_servers + store_info → stores (1:1 relationship, shared fields)

-- 1. Add columns from store_info that edge_servers doesn't have
ALTER TABLE edge_servers ADD COLUMN IF NOT EXISTS logo_url TEXT;
ALTER TABLE edge_servers ADD COLUMN IF NOT EXISTS created_at BIGINT;
ALTER TABLE edge_servers ADD COLUMN IF NOT EXISTS updated_at BIGINT;

-- 2. Backfill from store_info (store_info values take priority)
UPDATE edge_servers e SET
    name = COALESCE(NULLIF(si.name, ''), e.name),
    address = COALESCE(NULLIF(si.address, ''), e.address),
    nif = COALESCE(NULLIF(si.nif, ''), e.nif),
    phone = COALESCE(si.phone, e.phone),
    email = COALESCE(si.email, e.email),
    website = COALESCE(si.website, e.website),
    business_day_cutoff = COALESCE(si.business_day_cutoff, e.business_day_cutoff),
    logo_url = si.logo_url,
    created_at = si.created_at,
    updated_at = si.updated_at
FROM store_info si WHERE si.edge_server_id = e.id;

-- 3. Rename table
ALTER TABLE edge_servers RENAME TO stores;
ALTER INDEX idx_edge_servers_tenant RENAME TO idx_stores_tenant;

-- 4. Rename FK columns in all child tables (edge_server_id → store_id)
ALTER TABLE store_sync_cursors RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_tags RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_categories RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_products RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_product_specs RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_attributes RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_attribute_options RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_attribute_bindings RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_category_tag RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_price_rules RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_versions RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_zones RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_dining_tables RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_archived_orders RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_daily_reports RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_daily_report_tax_breakdown RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_daily_report_payment_breakdown RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_commands RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_shifts RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_employees RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_label_templates RENAME COLUMN edge_server_id TO store_id;
ALTER TABLE store_pending_ops RENAME COLUMN edge_server_id TO store_id;

-- 5. Drop store_info table
DROP TABLE store_info;
