-- Zones and dining tables JSONB mirror tables for edge sync

CREATE TABLE cloud_zones (
    id              BIGSERIAL PRIMARY KEY,
    edge_server_id  BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    tenant_id       TEXT NOT NULL,
    source_id       TEXT NOT NULL,
    data            JSONB NOT NULL,
    version         BIGINT NOT NULL DEFAULT 0,
    synced_at       BIGINT NOT NULL,
    UNIQUE (edge_server_id, source_id)
);
CREATE INDEX idx_cloud_zones_tenant ON cloud_zones(tenant_id);
CREATE INDEX idx_cloud_zones_edge ON cloud_zones(edge_server_id);

CREATE TABLE cloud_dining_tables (
    id              BIGSERIAL PRIMARY KEY,
    edge_server_id  BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    tenant_id       TEXT NOT NULL,
    source_id       TEXT NOT NULL,
    data            JSONB NOT NULL,
    version         BIGINT NOT NULL DEFAULT 0,
    synced_at       BIGINT NOT NULL,
    UNIQUE (edge_server_id, source_id)
);
CREATE INDEX idx_cloud_dining_tables_tenant ON cloud_dining_tables(tenant_id);
CREATE INDEX idx_cloud_dining_tables_edge ON cloud_dining_tables(edge_server_id);
