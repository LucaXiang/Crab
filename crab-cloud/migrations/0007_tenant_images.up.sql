-- Tenant image index: track S3 image references for orphan cleanup
CREATE TABLE tenant_images (
    tenant_id   TEXT    NOT NULL REFERENCES tenants(id),
    hash        TEXT    NOT NULL,
    ref_count   INTEGER NOT NULL DEFAULT 0,
    created_at  BIGINT  NOT NULL,
    orphaned_at BIGINT,  -- NULL = active, timestamp = marked for deletion
    PRIMARY KEY (tenant_id, hash)
);

-- For periodic orphan cleanup scan
CREATE INDEX idx_tenant_images_orphaned ON tenant_images (orphaned_at) WHERE orphaned_at IS NOT NULL;
