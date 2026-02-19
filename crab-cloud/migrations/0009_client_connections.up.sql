-- Client connection records (similar to activations but for clients)
-- Tracks client activation quota for max_clients enforcement

CREATE TABLE IF NOT EXISTS client_connections (
    entity_id         TEXT PRIMARY KEY,
    tenant_id         TEXT NOT NULL,
    device_id         TEXT NOT NULL,
    fingerprint       TEXT NOT NULL,
    status            TEXT NOT NULL DEFAULT 'active',
    activated_at      BIGINT NOT NULL,
    deactivated_at    BIGINT,
    replaced_by       TEXT REFERENCES client_connections(entity_id),
    last_refreshed_at BIGINT,

    UNIQUE(tenant_id, device_id)
);

CREATE INDEX IF NOT EXISTS idx_client_connections_tenant_status
ON client_connections(tenant_id, status);
