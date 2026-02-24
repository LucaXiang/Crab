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
    cancel_at_period_end BOOLEAN NOT NULL DEFAULT false,
    billing_interval   TEXT,
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
    name TEXT,
    address TEXT,
    phone TEXT,
    nif TEXT,
    email TEXT,
    website TEXT,
    business_day_cutoff TEXT DEFAULT '06:00',
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

-- ── Orders (archived) ──

CREATE TABLE IF NOT EXISTS cloud_archived_orders (
    id BIGSERIAL PRIMARY KEY,
    edge_server_id BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    tenant_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    order_key TEXT NOT NULL,
    receipt_number TEXT,
    status TEXT NOT NULL,
    end_time BIGINT,
    total DOUBLE PRECISION,
    tax NUMERIC(12,2),
    desglose JSONB NOT NULL DEFAULT '[]'::JSONB,
    guest_count INTEGER,
    discount_amount NUMERIC(12,2) NOT NULL DEFAULT 0,
    void_type TEXT,
    loss_amount NUMERIC(12,2),
    start_time BIGINT,
    prev_hash TEXT,
    curr_hash TEXT,
    version BIGINT NOT NULL DEFAULT 0,
    synced_at BIGINT NOT NULL
);

CREATE UNIQUE INDEX uq_cloud_archived_orders_key
    ON cloud_archived_orders (tenant_id, edge_server_id, order_key);
CREATE INDEX IF NOT EXISTS idx_cloud_archived_orders_tenant ON cloud_archived_orders (tenant_id);
CREATE INDEX IF NOT EXISTS idx_cloud_archived_orders_receipt ON cloud_archived_orders (tenant_id, receipt_number);
CREATE INDEX IF NOT EXISTS idx_cloud_archived_orders_end_time ON cloud_archived_orders (tenant_id, end_time);
CREATE INDEX IF NOT EXISTS idx_cloud_archived_orders_status ON cloud_archived_orders (tenant_id, status);
CREATE INDEX idx_cloud_archived_orders_list
    ON cloud_archived_orders (edge_server_id, tenant_id, status, end_time DESC);

-- Order items (permanent, for statistics)
CREATE TABLE IF NOT EXISTS cloud_order_items (
    id BIGSERIAL PRIMARY KEY,
    archived_order_id BIGINT NOT NULL REFERENCES cloud_archived_orders(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    category_name TEXT,
    quantity INTEGER NOT NULL,
    line_total NUMERIC(12,2) NOT NULL,
    tax_rate INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_cloud_order_items_order ON cloud_order_items (archived_order_id);

-- Order payments (permanent, for statistics)
CREATE TABLE IF NOT EXISTS cloud_order_payments (
    id BIGSERIAL PRIMARY KEY,
    archived_order_id BIGINT NOT NULL REFERENCES cloud_archived_orders(id) ON DELETE CASCADE,
    method TEXT NOT NULL,
    amount NUMERIC(12,2) NOT NULL
);

CREATE INDEX idx_cloud_order_payments_order ON cloud_order_payments (archived_order_id);

-- Order details (30-day rolling cache, JSONB)
CREATE TABLE IF NOT EXISTS cloud_order_details (
    id BIGSERIAL PRIMARY KEY,
    archived_order_id BIGINT NOT NULL REFERENCES cloud_archived_orders(id) ON DELETE CASCADE,
    detail JSONB NOT NULL,
    synced_at BIGINT NOT NULL,
    UNIQUE (archived_order_id)
);

CREATE INDEX idx_cloud_order_details_synced_at ON cloud_order_details (synced_at);

-- ── Daily Reports (normalized, mirrors edge-server SQLite schema) ──

CREATE TABLE IF NOT EXISTS cloud_daily_reports (
    id               BIGSERIAL PRIMARY KEY,
    edge_server_id   BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    source_id        BIGINT NOT NULL,
    business_date    TEXT NOT NULL,
    total_orders     INTEGER NOT NULL DEFAULT 0,
    completed_orders INTEGER NOT NULL DEFAULT 0,
    void_orders      INTEGER NOT NULL DEFAULT 0,
    total_sales      DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    total_paid       DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    total_unpaid     DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    void_amount      DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    total_tax        DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    total_discount   DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    total_surcharge  DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    generated_at     BIGINT,
    generated_by_id  BIGINT,
    generated_by_name TEXT,
    note             TEXT,
    updated_at       BIGINT NOT NULL,
    UNIQUE (edge_server_id, source_id),
    UNIQUE (edge_server_id, business_date)
);

CREATE INDEX IF NOT EXISTS idx_cloud_daily_reports_edge ON cloud_daily_reports(edge_server_id);

CREATE TABLE IF NOT EXISTS cloud_daily_report_tax_breakdown (
    id           BIGSERIAL PRIMARY KEY,
    report_id    BIGINT NOT NULL REFERENCES cloud_daily_reports(id) ON DELETE CASCADE,
    tax_rate     INTEGER NOT NULL,
    net_amount   DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    tax_amount   DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    gross_amount DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    order_count  INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_cloud_dr_tax_report ON cloud_daily_report_tax_breakdown(report_id);

CREATE TABLE IF NOT EXISTS cloud_daily_report_payment_breakdown (
    id        BIGSERIAL PRIMARY KEY,
    report_id BIGINT NOT NULL REFERENCES cloud_daily_reports(id) ON DELETE CASCADE,
    method    TEXT NOT NULL,
    amount    DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    count     INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_cloud_dr_payment_report ON cloud_daily_report_payment_breakdown(report_id);

-- ── Store Info (normalized, mirrors edge-server SQLite schema) ──

CREATE TABLE IF NOT EXISTS cloud_store_info (
    id                   BIGSERIAL PRIMARY KEY,
    edge_server_id       BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    name                 TEXT NOT NULL DEFAULT '',
    address              TEXT NOT NULL DEFAULT '',
    nif                  TEXT NOT NULL DEFAULT '',
    logo_url             TEXT,
    phone                TEXT,
    email                TEXT,
    website              TEXT,
    business_day_cutoff  TEXT NOT NULL DEFAULT '00:00',
    created_at           BIGINT,
    updated_at           BIGINT NOT NULL,
    UNIQUE (edge_server_id)
);

-- ── Commands ──

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

-- ── Red Flags 监控 ──

-- 订单事件（永久存储，用于红旗监控）
CREATE TABLE IF NOT EXISTS cloud_order_events (
    id                BIGSERIAL PRIMARY KEY,
    archived_order_id BIGINT NOT NULL REFERENCES cloud_archived_orders(id) ON DELETE CASCADE,
    seq               INTEGER NOT NULL,
    event_type        TEXT NOT NULL,
    timestamp         BIGINT NOT NULL,
    operator_id       BIGINT,
    operator_name     TEXT
);
CREATE INDEX IF NOT EXISTS idx_coe_order ON cloud_order_events(archived_order_id);
CREATE INDEX IF NOT EXISTS idx_coe_red_flags ON cloud_order_events(event_type, timestamp, operator_id);

-- Shifts (normalized, mirrors edge-server SQLite schema + cloud isolation)
CREATE TABLE IF NOT EXISTS cloud_shifts (
    id              BIGSERIAL PRIMARY KEY,
    edge_server_id  BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    source_id       BIGINT NOT NULL,
    operator_id     BIGINT NOT NULL,
    operator_name   TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'OPEN',
    start_time      BIGINT NOT NULL,
    end_time        BIGINT,
    starting_cash   DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    expected_cash   DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    actual_cash     DOUBLE PRECISION,
    cash_variance   DOUBLE PRECISION,
    abnormal_close  BOOLEAN NOT NULL DEFAULT FALSE,
    last_active_at  BIGINT,
    note            TEXT,
    created_at      BIGINT,
    updated_at      BIGINT NOT NULL,
    UNIQUE (edge_server_id, source_id)
);
CREATE INDEX IF NOT EXISTS idx_cloud_shifts_edge ON cloud_shifts(edge_server_id);
CREATE INDEX IF NOT EXISTS idx_cloud_shifts_status ON cloud_shifts(edge_server_id, status);

-- Employees (normalized, mirrors edge-server SQLite schema + cloud isolation)
CREATE TABLE IF NOT EXISTS cloud_employees (
    id              BIGSERIAL PRIMARY KEY,
    edge_server_id  BIGINT NOT NULL REFERENCES cloud_edge_servers(id),
    source_id       BIGINT NOT NULL,
    username        TEXT NOT NULL,
    hash_pass       TEXT NOT NULL,
    display_name    TEXT NOT NULL DEFAULT '',
    role_id         INTEGER NOT NULL,
    is_system       BOOLEAN NOT NULL DEFAULT FALSE,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      BIGINT NOT NULL DEFAULT 0,
    updated_at      BIGINT NOT NULL,
    UNIQUE (edge_server_id, source_id),
    UNIQUE (edge_server_id, username)
);
CREATE INDEX IF NOT EXISTS idx_cloud_employees_edge ON cloud_employees(edge_server_id);
