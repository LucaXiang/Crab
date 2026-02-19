-- activations table (device activation records)
-- Tracks edge-server activation quota for max_edge_servers enforcement

CREATE TABLE IF NOT EXISTS activations (
    entity_id         TEXT PRIMARY KEY,
    tenant_id         TEXT NOT NULL,
    device_id         TEXT NOT NULL,
    fingerprint       TEXT NOT NULL,
    status            TEXT NOT NULL DEFAULT 'active',
    activated_at      BIGINT NOT NULL,
    deactivated_at    BIGINT,
    replaced_by       TEXT REFERENCES activations(entity_id),
    last_refreshed_at BIGINT,

    UNIQUE(tenant_id, device_id)
);

CREATE INDEX IF NOT EXISTS idx_activations_tenant_status ON activations(tenant_id, status);
