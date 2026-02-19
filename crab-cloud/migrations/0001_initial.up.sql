-- ════════════════════════════════════════════════════════════════
-- Crab Cloud — Unified Schema (pre-production)
-- ════════════════════════════════════════════════════════════════

-- ── Tenants & Auth ──

CREATE TABLE IF NOT EXISTS tenants (
    id                TEXT PRIMARY KEY,
    email             TEXT NOT NULL UNIQUE,
    hashed_password   TEXT NOT NULL,
    name              TEXT,
    status            TEXT NOT NULL DEFAULT 'pending',
    stripe_customer_id TEXT UNIQUE,
    created_at        BIGINT NOT NULL,
    verified_at       BIGINT
);

CREATE INDEX IF NOT EXISTS idx_tenants_email ON tenants (email);
CREATE INDEX IF NOT EXISTS idx_tenants_status ON tenants (status);

CREATE TABLE IF NOT EXISTS subscriptions (
    id                 TEXT PRIMARY KEY,
    tenant_id          TEXT NOT NULL REFERENCES tenants(id),
    status             TEXT NOT NULL DEFAULT 'active',
    plan               TEXT NOT NULL,
    max_edge_servers   INT NOT NULL DEFAULT 1,
    max_clients        INT NOT NULL DEFAULT 5,
    features           TEXT[] NOT NULL DEFAULT '{}',
    current_period_end BIGINT,
    created_at         BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_subscriptions_tenant ON subscriptions (tenant_id);

CREATE TABLE IF NOT EXISTS email_verifications (
    email      TEXT NOT NULL,
    purpose    TEXT NOT NULL DEFAULT 'registration',
    code       TEXT NOT NULL,
    attempts   INT NOT NULL DEFAULT 0,
    expires_at BIGINT NOT NULL,
    created_at BIGINT NOT NULL,
    metadata   TEXT,
    PRIMARY KEY (email, purpose)
);

CREATE TABLE IF NOT EXISTS refresh_tokens (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    device_id TEXT NOT NULL,
    expires_at BIGINT NOT NULL,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000)::BIGINT,
    revoked BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX idx_refresh_tokens_tenant ON refresh_tokens(tenant_id);
CREATE INDEX idx_refresh_tokens_expires ON refresh_tokens(expires_at) WHERE NOT revoked;

-- ── PKI / Activations ──

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

CREATE INDEX IF NOT EXISTS idx_client_connections_tenant_status ON client_connections(tenant_id, status);

-- P12 certificates metadata (binary + password in Secrets Manager)
CREATE TABLE IF NOT EXISTS p12_certificates (
    tenant_id         TEXT PRIMARY KEY,
    secret_name       TEXT NOT NULL,
    fingerprint       TEXT,
    common_name       TEXT,
    serial_number     TEXT,
    organization_id   TEXT,
    organization      TEXT,
    issuer            TEXT,
    country           TEXT,
    expires_at        BIGINT,
    not_before        BIGINT,
    uploaded_at       BIGINT NOT NULL,
    updated_at        BIGINT NOT NULL
);

-- ── Stripe ──

CREATE TABLE IF NOT EXISTS processed_webhook_events (
    event_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    processed_at BIGINT NOT NULL
);

-- ── Audit ──

CREATE TABLE IF NOT EXISTS cloud_audit_log (
    id BIGSERIAL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    action TEXT NOT NULL,
    detail JSONB,
    ip_address TEXT,
    created_at BIGINT NOT NULL
);

CREATE INDEX idx_cloud_audit_tenant ON cloud_audit_log (tenant_id, created_at);

-- ── Cloud Sync (edge-server data mirrors) ──

CREATE TABLE IF NOT EXISTS cloud_edge_servers (
    id BIGSERIAL PRIMARY KEY,
    entity_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    last_sync_at BIGINT,
    registered_at BIGINT NOT NULL,
    UNIQUE (entity_id, tenant_id)
);

CREATE INDEX IF NOT EXISTS idx_cloud_edge_servers_tenant ON cloud_edge_servers (tenant_id);

CREATE TABLE IF NOT EXISTS cloud_sync_cursors (
    id BIGSERIAL PRIMARY KEY,
    edge_server_id BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    resource TEXT NOT NULL,
    last_version BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL,
    UNIQUE (edge_server_id, resource)
);

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

CREATE INDEX IF NOT EXISTS idx_cloud_products_tenant ON cloud_products (tenant_id);

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

CREATE INDEX IF NOT EXISTS idx_cloud_categories_tenant ON cloud_categories (tenant_id);

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

CREATE INDEX IF NOT EXISTS idx_cloud_archived_orders_tenant ON cloud_archived_orders (tenant_id);
CREATE INDEX IF NOT EXISTS idx_cloud_archived_orders_receipt ON cloud_archived_orders (tenant_id, receipt_number);
CREATE INDEX IF NOT EXISTS idx_cloud_archived_orders_end_time ON cloud_archived_orders (tenant_id, end_time);
CREATE INDEX IF NOT EXISTS idx_cloud_archived_orders_status ON cloud_archived_orders (tenant_id, status);

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

CREATE INDEX IF NOT EXISTS idx_cloud_daily_reports_tenant ON cloud_daily_reports (tenant_id);

CREATE TABLE IF NOT EXISTS cloud_store_info (
    id BIGSERIAL PRIMARY KEY,
    edge_server_id BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    tenant_id TEXT NOT NULL,
    data JSONB NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    synced_at BIGINT NOT NULL,
    UNIQUE (edge_server_id, tenant_id)
);

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
