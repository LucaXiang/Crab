-- Zones (normalized, mirrors edge-server SQLite schema + cloud isolation)

CREATE TABLE cloud_zones (
    id              BIGSERIAL PRIMARY KEY,
    edge_server_id  BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    source_id       BIGINT NOT NULL,
    name            TEXT NOT NULL,
    description     TEXT,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at      BIGINT NOT NULL,
    UNIQUE (edge_server_id, source_id)
);
CREATE INDEX idx_cloud_zones_edge ON cloud_zones(edge_server_id);
CREATE INDEX idx_cloud_zones_name ON cloud_zones(edge_server_id, name);

-- Dining tables (normalized, mirrors edge-server SQLite schema + cloud isolation)

CREATE TABLE cloud_dining_tables (
    id              BIGSERIAL PRIMARY KEY,
    edge_server_id  BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    source_id       BIGINT NOT NULL,
    name            TEXT NOT NULL,
    zone_source_id  BIGINT NOT NULL,
    capacity        INTEGER NOT NULL DEFAULT 4,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at      BIGINT NOT NULL,
    UNIQUE (edge_server_id, source_id),
    UNIQUE (edge_server_id, zone_source_id, name)
);
CREATE INDEX idx_cloud_dining_tables_edge ON cloud_dining_tables(edge_server_id);
CREATE INDEX idx_cloud_dining_tables_zone ON cloud_dining_tables(edge_server_id, zone_source_id);
