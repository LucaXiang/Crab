DROP TABLE IF EXISTS catalog_versions CASCADE;
DROP TABLE IF EXISTS catalog_price_rules CASCADE;
DROP TABLE IF EXISTS catalog_attribute_bindings CASCADE;
DROP TABLE IF EXISTS catalog_attribute_options CASCADE;
DROP TABLE IF EXISTS catalog_attributes CASCADE;
DROP TABLE IF EXISTS catalog_product_tag CASCADE;
DROP TABLE IF EXISTS catalog_product_specs CASCADE;
DROP TABLE IF EXISTS catalog_products CASCADE;
DROP TABLE IF EXISTS catalog_category_tag CASCADE;
DROP TABLE IF EXISTS catalog_category_print_dest CASCADE;
DROP TABLE IF EXISTS catalog_categories CASCADE;
DROP TABLE IF EXISTS catalog_tags CASCADE;

-- Restore JSONB blob tables
CREATE TABLE IF NOT EXISTS cloud_products (
    id             BIGSERIAL PRIMARY KEY,
    edge_server_id BIGINT NOT NULL REFERENCES cloud_edge_servers(id) ON DELETE CASCADE,
    tenant_id      TEXT   NOT NULL,
    source_id      BIGINT NOT NULL,
    data           JSONB  NOT NULL,
    version        BIGINT NOT NULL DEFAULT 0,
    synced_at      BIGINT NOT NULL,
    UNIQUE (edge_server_id, source_id)
);
CREATE INDEX IF NOT EXISTS idx_cloud_products_tenant ON cloud_products (tenant_id);

CREATE TABLE IF NOT EXISTS cloud_categories (
    id             BIGSERIAL PRIMARY KEY,
    edge_server_id BIGINT NOT NULL REFERENCES cloud_edge_servers(id) ON DELETE CASCADE,
    tenant_id      TEXT   NOT NULL,
    source_id      BIGINT NOT NULL,
    data           JSONB  NOT NULL,
    version        BIGINT NOT NULL DEFAULT 0,
    synced_at      BIGINT NOT NULL,
    UNIQUE (edge_server_id, source_id)
);
CREATE INDEX IF NOT EXISTS idx_cloud_categories_tenant ON cloud_categories (tenant_id);
