CREATE TABLE IF NOT EXISTS cloud_audit_log (
    id BIGSERIAL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    action TEXT NOT NULL,
    detail JSONB,
    ip_address TEXT,
    created_at BIGINT NOT NULL
);

CREATE INDEX idx_cloud_audit_tenant ON cloud_audit_log (tenant_id, created_at);
