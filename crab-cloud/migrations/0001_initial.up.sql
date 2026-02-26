-- ════════════════════════════════════════════════════════════════
-- Crab Cloud — Unified Schema (consolidated from 0001–0007)
-- ════════════════════════════════════════════════════════════════
-- Naming: store_* = store-scoped (has store_id)
--         no prefix = global infrastructure

-- ── Tenants & Auth ──

CREATE TABLE IF NOT EXISTS tenants (
    id                TEXT PRIMARY KEY,
    email             TEXT NOT NULL UNIQUE,
    hashed_password   TEXT NOT NULL,
    name              TEXT,
    status            TEXT NOT NULL DEFAULT 'pending',
    stripe_customer_id TEXT UNIQUE,
    ca_cert_pem       TEXT,
    ca_key_encrypted  TEXT,
    created_at        BIGINT NOT NULL,
    verified_at       BIGINT
);

CREATE INDEX IF NOT EXISTS idx_tenants_email ON tenants (email);
CREATE INDEX IF NOT EXISTS idx_tenants_status ON tenants (status);

CREATE TABLE IF NOT EXISTS subscriptions (
    id                 TEXT PRIMARY KEY,
    tenant_id          TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
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

CREATE INDEX IF NOT EXISTS idx_subscriptions_tenant_id ON subscriptions(tenant_id);

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
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
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
CREATE INDEX IF NOT EXISTS idx_activations_replaced_by ON activations(replaced_by) WHERE replaced_by IS NOT NULL;

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
CREATE INDEX IF NOT EXISTS idx_client_connections_replaced_by ON client_connections(replaced_by) WHERE replaced_by IS NOT NULL;

CREATE TABLE IF NOT EXISTS p12_certificates (
    tenant_id         TEXT PRIMARY KEY,
    p12_encrypted     TEXT,
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

CREATE TABLE IF NOT EXISTS audit_logs (
    id BIGSERIAL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    action TEXT NOT NULL,
    detail JSONB,
    ip_address TEXT,
    created_at BIGINT NOT NULL
);

CREATE INDEX idx_audit_logs_tenant ON audit_logs (tenant_id, created_at);

-- ── Stores (was edge_servers, merged with store_info) ──

CREATE TABLE IF NOT EXISTS stores (
    id BIGSERIAL PRIMARY KEY,
    entity_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    store_number INT NOT NULL,
    name TEXT,
    address TEXT,
    phone TEXT,
    nif TEXT,
    email TEXT,
    website TEXT,
    logo_url TEXT,
    business_day_cutoff TEXT DEFAULT '06:00',
    last_sync_at BIGINT,
    registered_at BIGINT NOT NULL,
    created_at BIGINT,
    updated_at BIGINT,
    UNIQUE (entity_id, tenant_id)
);

CREATE INDEX IF NOT EXISTS idx_stores_tenant ON stores (tenant_id);

-- ── Sync Cursors ──

CREATE TABLE IF NOT EXISTS store_sync_cursors (
    id BIGSERIAL PRIMARY KEY,
    store_id BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    resource TEXT NOT NULL,
    last_version BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL,
    UNIQUE (store_id, resource)
);

-- ── Tags ──

CREATE TABLE store_tags (
    id             BIGSERIAL PRIMARY KEY,
    store_id       BIGINT  NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    source_id      BIGINT  NOT NULL,
    name           TEXT    NOT NULL,
    color          TEXT    NOT NULL DEFAULT '#3B82F6',
    display_order  INTEGER NOT NULL DEFAULT 0,
    is_active      BOOLEAN NOT NULL DEFAULT TRUE,
    is_system      BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at     BIGINT  NOT NULL,
    UNIQUE (store_id, source_id)
);
CREATE INDEX idx_store_tags_store ON store_tags (store_id);

-- ── Categories ──

CREATE TABLE store_categories (
    id                       BIGSERIAL PRIMARY KEY,
    store_id                 BIGINT  NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    source_id                BIGINT  NOT NULL,
    name                     TEXT    NOT NULL,
    sort_order               INTEGER NOT NULL DEFAULT 0,
    is_kitchen_print_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    is_label_print_enabled   BOOLEAN NOT NULL DEFAULT FALSE,
    is_active                BOOLEAN NOT NULL DEFAULT TRUE,
    is_virtual               BOOLEAN NOT NULL DEFAULT FALSE,
    match_mode               TEXT    NOT NULL DEFAULT 'any',
    is_display               BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at               BIGINT  NOT NULL,
    UNIQUE (store_id, source_id)
);
CREATE INDEX idx_store_categories_store ON store_categories (store_id);

CREATE TABLE store_category_print_dest (
    id             BIGSERIAL PRIMARY KEY,
    category_id    BIGINT       NOT NULL REFERENCES store_categories(id) ON DELETE CASCADE,
    dest_source_id BIGINT       NOT NULL,
    purpose        VARCHAR(10)  NOT NULL
);
CREATE INDEX idx_store_cat_print_dest_category ON store_category_print_dest (category_id);

CREATE TABLE store_category_tag (
    category_id    BIGINT NOT NULL REFERENCES store_categories(id) ON DELETE CASCADE,
    tag_source_id  BIGINT NOT NULL,
    PRIMARY KEY (category_id, tag_source_id)
);

-- ── Products ──

CREATE TABLE store_products (
    id                       BIGSERIAL PRIMARY KEY,
    store_id                 BIGINT  NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    source_id                BIGINT  NOT NULL,
    name                     TEXT    NOT NULL,
    image                    TEXT    NOT NULL DEFAULT '',
    category_source_id       BIGINT  NOT NULL,
    sort_order               INTEGER NOT NULL DEFAULT 0,
    tax_rate                 INTEGER NOT NULL DEFAULT 0,
    receipt_name             TEXT,
    kitchen_print_name       TEXT,
    is_kitchen_print_enabled INTEGER NOT NULL DEFAULT -1,
    is_label_print_enabled   INTEGER NOT NULL DEFAULT -1,
    is_active                BOOLEAN NOT NULL DEFAULT TRUE,
    external_id              BIGINT,
    updated_at               BIGINT  NOT NULL,
    UNIQUE (store_id, source_id)
);
CREATE INDEX idx_store_products_store ON store_products (store_id);

CREATE TABLE store_product_specs (
    id            BIGSERIAL PRIMARY KEY,
    product_id    BIGINT  NOT NULL REFERENCES store_products(id) ON DELETE CASCADE,
    source_id     BIGINT  NOT NULL,
    name          TEXT    NOT NULL,
    price         DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    display_order INTEGER NOT NULL DEFAULT 0,
    is_default    BOOLEAN NOT NULL DEFAULT FALSE,
    is_active     BOOLEAN NOT NULL DEFAULT TRUE,
    receipt_name  TEXT,
    is_root       BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX idx_store_specs_product ON store_product_specs (product_id);

CREATE TABLE store_product_tag (
    product_id    BIGINT NOT NULL REFERENCES store_products(id) ON DELETE CASCADE,
    tag_source_id BIGINT NOT NULL,
    PRIMARY KEY (product_id, tag_source_id)
);

-- ── Attributes ──

CREATE TABLE store_attributes (
    id                    BIGSERIAL PRIMARY KEY,
    store_id              BIGINT  NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    source_id             BIGINT  NOT NULL,
    name                  TEXT    NOT NULL,
    is_multi_select       BOOLEAN NOT NULL DEFAULT FALSE,
    max_selections        INTEGER,
    default_option_ids    JSONB,
    display_order         INTEGER NOT NULL DEFAULT 0,
    is_active             BOOLEAN NOT NULL DEFAULT TRUE,
    show_on_receipt       BOOLEAN NOT NULL DEFAULT FALSE,
    receipt_name          TEXT,
    show_on_kitchen_print BOOLEAN NOT NULL DEFAULT FALSE,
    kitchen_print_name    TEXT,
    updated_at            BIGINT  NOT NULL,
    UNIQUE (store_id, source_id)
);
CREATE INDEX idx_store_attributes_store ON store_attributes (store_id);

CREATE TABLE store_attribute_options (
    id                 BIGSERIAL PRIMARY KEY,
    attribute_id       BIGINT  NOT NULL REFERENCES store_attributes(id) ON DELETE CASCADE,
    source_id          BIGINT  NOT NULL,
    name               TEXT    NOT NULL,
    price_modifier     DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    display_order      INTEGER NOT NULL DEFAULT 0,
    is_active          BOOLEAN NOT NULL DEFAULT TRUE,
    receipt_name       TEXT,
    kitchen_print_name TEXT,
    enable_quantity    BOOLEAN NOT NULL DEFAULT FALSE,
    max_quantity       INTEGER
);
CREATE INDEX idx_store_options_attribute ON store_attribute_options (attribute_id);

CREATE TABLE store_attribute_bindings (
    id                  BIGSERIAL PRIMARY KEY,
    store_id            BIGINT  NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    source_id           BIGINT  NOT NULL,
    owner_type          TEXT    NOT NULL,
    owner_source_id     BIGINT  NOT NULL,
    attribute_source_id BIGINT  NOT NULL,
    is_required         BOOLEAN NOT NULL DEFAULT FALSE,
    display_order       INTEGER NOT NULL DEFAULT 0,
    default_option_ids  JSONB,
    UNIQUE (store_id, source_id)
);
CREATE INDEX idx_store_bindings_store ON store_attribute_bindings (store_id);
CREATE INDEX idx_store_bindings_owner ON store_attribute_bindings (owner_type, owner_source_id);

-- ── Price Rules ──

CREATE TABLE store_price_rules (
    id               BIGSERIAL PRIMARY KEY,
    store_id         BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    source_id        BIGINT NOT NULL,
    name             TEXT NOT NULL,
    receipt_name     TEXT,
    description      TEXT,
    rule_type        TEXT NOT NULL,
    product_scope    TEXT NOT NULL,
    target_id        BIGINT,
    zone_scope       TEXT NOT NULL DEFAULT 'all',
    adjustment_type  TEXT NOT NULL,
    adjustment_value DOUBLE PRECISION NOT NULL,
    is_stackable     BOOLEAN NOT NULL DEFAULT TRUE,
    is_exclusive     BOOLEAN NOT NULL DEFAULT FALSE,
    valid_from       BIGINT,
    valid_until      BIGINT,
    active_days      INTEGER,
    active_start_time TEXT,
    active_end_time  TEXT,
    is_active        BOOLEAN NOT NULL DEFAULT TRUE,
    created_by       BIGINT,
    created_at       BIGINT NOT NULL,
    updated_at       BIGINT NOT NULL,
    UNIQUE (store_id, source_id)
);
CREATE INDEX IF NOT EXISTS idx_store_price_rules_store ON store_price_rules(store_id);

-- ── Store Version Tracking ──

CREATE TABLE store_versions (
    id             BIGSERIAL PRIMARY KEY,
    store_id       BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE UNIQUE,
    version        BIGINT NOT NULL DEFAULT 0,
    updated_at     BIGINT NOT NULL
);

-- ── Zones ──

CREATE TABLE store_zones (
    id              BIGSERIAL PRIMARY KEY,
    store_id        BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    source_id       BIGINT NOT NULL,
    name            TEXT NOT NULL,
    description     TEXT,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at      BIGINT NOT NULL,
    UNIQUE (store_id, source_id)
);
CREATE INDEX idx_store_zones_store ON store_zones(store_id);
CREATE INDEX idx_store_zones_name ON store_zones(store_id, name);

-- ── Dining Tables ──

CREATE TABLE store_dining_tables (
    id              BIGSERIAL PRIMARY KEY,
    store_id        BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    source_id       BIGINT NOT NULL,
    name            TEXT NOT NULL,
    zone_source_id  BIGINT NOT NULL,
    capacity        INTEGER NOT NULL DEFAULT 4,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at      BIGINT NOT NULL,
    UNIQUE (store_id, source_id),
    UNIQUE (store_id, zone_source_id, name)
);
CREATE INDEX idx_store_dining_tables_store ON store_dining_tables(store_id);
CREATE INDEX idx_store_dining_tables_zone ON store_dining_tables(store_id, zone_source_id);

-- ── Orders (archived, simplified — detail as JSONB) ──

CREATE TABLE IF NOT EXISTS store_archived_orders (
    id BIGSERIAL PRIMARY KEY,
    store_id BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    tenant_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    order_key TEXT NOT NULL,
    receipt_number TEXT,
    status TEXT NOT NULL,
    end_time BIGINT,
    total DOUBLE PRECISION,
    tax DOUBLE PRECISION,
    desglose JSONB NOT NULL DEFAULT '[]'::JSONB,
    guest_count INTEGER,
    discount_amount DOUBLE PRECISION NOT NULL DEFAULT 0,
    void_type TEXT,
    loss_amount DOUBLE PRECISION,
    start_time BIGINT,
    prev_hash TEXT,
    curr_hash TEXT,
    version BIGINT NOT NULL DEFAULT 0,
    detail JSONB,
    synced_at BIGINT NOT NULL
);

CREATE UNIQUE INDEX uq_store_archived_orders_key
    ON store_archived_orders (tenant_id, store_id, order_key);
CREATE INDEX IF NOT EXISTS idx_store_archived_orders_tenant ON store_archived_orders (tenant_id);
CREATE INDEX IF NOT EXISTS idx_store_archived_orders_receipt ON store_archived_orders (tenant_id, receipt_number);
CREATE INDEX IF NOT EXISTS idx_store_archived_orders_end_time ON store_archived_orders (tenant_id, end_time);
CREATE INDEX IF NOT EXISTS idx_store_archived_orders_status ON store_archived_orders (tenant_id, status);
CREATE INDEX idx_store_archived_orders_list
    ON store_archived_orders (store_id, tenant_id, status, end_time DESC);

-- ── Daily Reports ──

CREATE TABLE IF NOT EXISTS store_daily_reports (
    id               BIGSERIAL PRIMARY KEY,
    store_id         BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    source_id        BIGINT NOT NULL,
    business_date    TEXT NOT NULL,
    total_orders     BIGINT NOT NULL DEFAULT 0,
    completed_orders BIGINT NOT NULL DEFAULT 0,
    void_orders      BIGINT NOT NULL DEFAULT 0,
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
    UNIQUE (store_id, source_id),
    UNIQUE (store_id, business_date)
);

CREATE INDEX IF NOT EXISTS idx_store_daily_reports_store ON store_daily_reports(store_id);

CREATE TABLE IF NOT EXISTS store_daily_report_tax_breakdown (
    id           BIGSERIAL PRIMARY KEY,
    report_id    BIGINT NOT NULL REFERENCES store_daily_reports(id) ON DELETE CASCADE,
    tax_rate     INTEGER NOT NULL,
    net_amount   DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    tax_amount   DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    gross_amount DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    order_count  BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_store_dr_tax_report ON store_daily_report_tax_breakdown(report_id);

CREATE TABLE IF NOT EXISTS store_daily_report_payment_breakdown (
    id        BIGSERIAL PRIMARY KEY,
    report_id BIGINT NOT NULL REFERENCES store_daily_reports(id) ON DELETE CASCADE,
    method    TEXT NOT NULL,
    amount    DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    count     BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_store_dr_payment_report ON store_daily_report_payment_breakdown(report_id);

CREATE TABLE store_daily_report_shift_breakdown (
    id               BIGSERIAL PRIMARY KEY,
    report_id        BIGINT NOT NULL REFERENCES store_daily_reports(id) ON DELETE CASCADE,
    shift_source_id  BIGINT NOT NULL,
    operator_id      BIGINT NOT NULL,
    operator_name    TEXT NOT NULL,
    status           TEXT NOT NULL,
    start_time       BIGINT NOT NULL,
    end_time         BIGINT,
    starting_cash    DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    expected_cash    DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    actual_cash      DOUBLE PRECISION,
    cash_variance    DOUBLE PRECISION,
    abnormal_close   BOOLEAN NOT NULL DEFAULT FALSE,
    total_orders     BIGINT NOT NULL DEFAULT 0,
    completed_orders BIGINT NOT NULL DEFAULT 0,
    void_orders      BIGINT NOT NULL DEFAULT 0,
    total_sales      DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    total_paid       DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    void_amount      DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    total_tax        DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    total_discount   DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    total_surcharge  DOUBLE PRECISION NOT NULL DEFAULT 0.0
);

CREATE INDEX idx_store_shift_breakdown_report ON store_daily_report_shift_breakdown(report_id);

-- ── Commands ──

CREATE TABLE IF NOT EXISTS store_commands (
    id BIGSERIAL PRIMARY KEY,
    store_id BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    tenant_id TEXT NOT NULL,
    command_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at BIGINT NOT NULL,
    executed_at BIGINT,
    result JSONB
);

CREATE INDEX IF NOT EXISTS idx_store_commands_pending
    ON store_commands (store_id, status) WHERE status = 'pending';
CREATE INDEX IF NOT EXISTS idx_store_commands_store ON store_commands(store_id);

-- ── Shifts ──

CREATE TABLE IF NOT EXISTS store_shifts (
    id              BIGSERIAL PRIMARY KEY,
    store_id        BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
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
    UNIQUE (store_id, source_id)
);
CREATE INDEX IF NOT EXISTS idx_store_shifts_store ON store_shifts(store_id);
CREATE INDEX IF NOT EXISTS idx_store_shifts_status ON store_shifts(store_id, status);

-- ── Employees ──

CREATE TABLE IF NOT EXISTS store_employees (
    id              BIGSERIAL PRIMARY KEY,
    store_id        BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    source_id       BIGINT NOT NULL,
    username        TEXT NOT NULL,
    hash_pass       TEXT NOT NULL,
    name            TEXT NOT NULL DEFAULT '',
    role_id         BIGINT NOT NULL,
    is_system       BOOLEAN NOT NULL DEFAULT FALSE,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      BIGINT NOT NULL,
    updated_at      BIGINT NOT NULL,
    UNIQUE (store_id, source_id),
    UNIQUE (store_id, username)
);
CREATE INDEX IF NOT EXISTS idx_store_employees_store ON store_employees(store_id);

-- ── Label Templates ──

CREATE TYPE label_field_type AS ENUM (
    'text', 'barcode', 'qrcode', 'image', 'separator', 'datetime', 'price', 'counter'
);

CREATE TYPE label_field_alignment AS ENUM (
    'left', 'center', 'right'
);

CREATE TABLE store_label_templates (
    id              BIGSERIAL PRIMARY KEY,
    store_id        BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    source_id       BIGINT NOT NULL DEFAULT 0,
    tenant_id       TEXT NOT NULL,
    name            TEXT NOT NULL,
    description     TEXT,
    width           REAL NOT NULL,
    height          REAL NOT NULL,
    padding         REAL NOT NULL DEFAULT 2.0,
    is_default      BOOLEAN NOT NULL DEFAULT false,
    is_active       BOOLEAN NOT NULL DEFAULT true,
    width_mm        REAL,
    height_mm       REAL,
    padding_mm_x    REAL,
    padding_mm_y    REAL,
    render_dpi      INTEGER,
    test_data       TEXT,
    created_at      BIGINT NOT NULL,
    updated_at      BIGINT NOT NULL,
    UNIQUE(store_id, source_id)
);

CREATE INDEX idx_store_label_templates_store ON store_label_templates(store_id);

CREATE TABLE store_label_fields (
    id              BIGSERIAL PRIMARY KEY,
    template_id     BIGINT NOT NULL REFERENCES store_label_templates(id) ON DELETE CASCADE,
    field_id        TEXT NOT NULL,
    name            TEXT NOT NULL,
    field_type      label_field_type NOT NULL DEFAULT 'text',
    x               REAL NOT NULL DEFAULT 0,
    y               REAL NOT NULL DEFAULT 0,
    width           REAL NOT NULL DEFAULT 100,
    height          REAL NOT NULL DEFAULT 30,
    font_size       INTEGER NOT NULL DEFAULT 10,
    font_weight     TEXT,
    font_family     TEXT,
    color           TEXT,
    rotate          INTEGER,
    alignment       label_field_alignment,
    data_source     TEXT NOT NULL DEFAULT '',
    format          TEXT,
    visible         BOOLEAN NOT NULL DEFAULT true,
    label           TEXT,
    template        TEXT,
    data_key        TEXT,
    source_type     TEXT,
    maintain_aspect_ratio BOOLEAN,
    style           TEXT,
    align           TEXT,
    vertical_align  TEXT,
    line_style      TEXT
);

CREATE INDEX idx_store_label_fields_template ON store_label_fields(template_id);

-- ── Pending Ops (Console edits when edge offline) ──

CREATE TABLE store_pending_ops (
    id BIGSERIAL PRIMARY KEY,
    store_id BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    op JSONB NOT NULL,
    changed_at BIGINT NOT NULL,
    created_at BIGINT NOT NULL
);

CREATE INDEX idx_pending_ops_store ON store_pending_ops(store_id);

-- ── Tenant Images (S3 orphan tracking) ──

CREATE TABLE tenant_images (
    tenant_id   TEXT    NOT NULL REFERENCES tenants(id),
    hash        TEXT    NOT NULL,
    ref_count   INTEGER NOT NULL DEFAULT 0,
    created_at  BIGINT  NOT NULL,
    orphaned_at BIGINT,
    PRIMARY KEY (tenant_id, hash)
);

CREATE INDEX idx_tenant_images_orphaned ON tenant_images (orphaned_at) WHERE orphaned_at IS NOT NULL;
