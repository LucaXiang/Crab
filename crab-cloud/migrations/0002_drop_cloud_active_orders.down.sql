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

CREATE INDEX IF NOT EXISTS idx_cloud_active_orders_tenant ON cloud_active_orders (tenant_id);
