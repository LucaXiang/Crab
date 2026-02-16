-- Cloud sync tables (crab-cloud owns these)
-- All tables are partitioned by tenant_id for isolation

-- Registered edge-servers
CREATE TABLE IF NOT EXISTS cloud_edge_servers (
    id BIGSERIAL PRIMARY KEY,
    entity_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    last_sync_at BIGINT,
    registered_at BIGINT NOT NULL,
    UNIQUE (entity_id, tenant_id)
);

CREATE INDEX IF NOT EXISTS idx_cloud_edge_servers_tenant
    ON cloud_edge_servers (tenant_id);

-- Sync cursor: tracks last synced version per edge+resource
CREATE TABLE IF NOT EXISTS cloud_sync_cursors (
    id BIGSERIAL PRIMARY KEY,
    edge_server_id BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    resource TEXT NOT NULL,
    last_version BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL,
    UNIQUE (edge_server_id, resource)
);

-- Product mirror
CREATE TABLE IF NOT EXISTS cloud_products (
    id BIGSERIAL PRIMARY KEY,
    edge_server_id BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    tenant_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    data JSONB NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    synced_at BIGINT NOT NULL,
    UNIQUE (edge_server_id, source_id)
);

CREATE INDEX IF NOT EXISTS idx_cloud_products_tenant
    ON cloud_products (tenant_id);

-- Category mirror
CREATE TABLE IF NOT EXISTS cloud_categories (
    id BIGSERIAL PRIMARY KEY,
    edge_server_id BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    tenant_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    data JSONB NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    synced_at BIGINT NOT NULL,
    UNIQUE (edge_server_id, source_id)
);

CREATE INDEX IF NOT EXISTS idx_cloud_categories_tenant
    ON cloud_categories (tenant_id);

-- Archived order mirror (core fields extracted + full JSONB snapshot)
CREATE TABLE IF NOT EXISTS cloud_archived_orders (
    id BIGSERIAL PRIMARY KEY,
    edge_server_id BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    tenant_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    receipt_number TEXT,
    status TEXT NOT NULL,
    end_time BIGINT,
    total DOUBLE PRECISION,
    data JSONB NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    synced_at BIGINT NOT NULL,
    UNIQUE (edge_server_id, source_id)
);

CREATE INDEX IF NOT EXISTS idx_cloud_archived_orders_tenant
    ON cloud_archived_orders (tenant_id);
CREATE INDEX IF NOT EXISTS idx_cloud_archived_orders_receipt
    ON cloud_archived_orders (tenant_id, receipt_number);
CREATE INDEX IF NOT EXISTS idx_cloud_archived_orders_end_time
    ON cloud_archived_orders (tenant_id, end_time);
CREATE INDEX IF NOT EXISTS idx_cloud_archived_orders_status
    ON cloud_archived_orders (tenant_id, status);

-- Active order snapshot (real-time mirror)
CREATE TABLE IF NOT EXISTS cloud_active_orders (
    id BIGSERIAL PRIMARY KEY,
    edge_server_id BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    tenant_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    data JSONB NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    synced_at BIGINT NOT NULL,
    UNIQUE (edge_server_id, source_id)
);

CREATE INDEX IF NOT EXISTS idx_cloud_active_orders_tenant
    ON cloud_active_orders (tenant_id);

-- Daily report mirror
CREATE TABLE IF NOT EXISTS cloud_daily_reports (
    id BIGSERIAL PRIMARY KEY,
    edge_server_id BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    tenant_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    data JSONB NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    synced_at BIGINT NOT NULL,
    UNIQUE (edge_server_id, source_id)
);

CREATE INDEX IF NOT EXISTS idx_cloud_daily_reports_tenant
    ON cloud_daily_reports (tenant_id);

-- Store info mirror
CREATE TABLE IF NOT EXISTS cloud_store_info (
    id BIGSERIAL PRIMARY KEY,
    edge_server_id BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    tenant_id TEXT NOT NULL,
    data JSONB NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    synced_at BIGINT NOT NULL,
    UNIQUE (edge_server_id, tenant_id)
);

-- Remote command queue (future use)
CREATE TABLE IF NOT EXISTS cloud_commands (
    id BIGSERIAL PRIMARY KEY,
    edge_server_id BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    tenant_id TEXT NOT NULL,
    command_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at BIGINT NOT NULL,
    executed_at BIGINT,
    result JSONB
);

CREATE INDEX IF NOT EXISTS idx_cloud_commands_pending
    ON cloud_commands (edge_server_id, status) WHERE status = 'pending';
