-- ════════════════════════════════════════════════════════════════
-- Crab Cloud — Unified Schema (consolidated 0001–0014)
-- ════════════════════════════════════════════════════════════════
-- Naming: store_* = store-scoped (has store_id)
--         no prefix = global infrastructure

-- ── Tenants & Auth ──

CREATE TABLE IF NOT EXISTS tenants (
    id                BIGINT PRIMARY KEY,
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
    tenant_id          BIGINT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    status             TEXT NOT NULL DEFAULT 'active',
    plan               TEXT NOT NULL,
    max_stores         INT NOT NULL DEFAULT 1,
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
    tenant_id BIGINT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL,
    expires_at BIGINT NOT NULL,
    user_agent TEXT NOT NULL DEFAULT '',
    ip_address TEXT NOT NULL DEFAULT '',
    created_at BIGINT NOT NULL DEFAULT 0,
    revoked BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX idx_refresh_tokens_active
    ON refresh_tokens (tenant_id, expires_at DESC)
    WHERE NOT revoked;

-- ── PKI / Activations ──

CREATE TABLE IF NOT EXISTS activations (
    entity_id         TEXT PRIMARY KEY,
    tenant_id         BIGINT NOT NULL,
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
    tenant_id         BIGINT NOT NULL,
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
    tenant_id         BIGINT PRIMARY KEY,
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
    tenant_id BIGINT NOT NULL,
    action TEXT NOT NULL,
    detail JSONB,
    ip_address TEXT,
    created_at BIGINT NOT NULL
);

CREATE INDEX idx_audit_logs_tenant ON audit_logs (tenant_id, created_at);
CREATE INDEX idx_audit_logs_tenant_action ON audit_logs (tenant_id, action, created_at DESC);

-- ── Stores ──

CREATE TABLE IF NOT EXISTS stores (
    id BIGINT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    tenant_id BIGINT NOT NULL,
    device_id TEXT NOT NULL,
    store_number INT NOT NULL,
    alias TEXT NOT NULL DEFAULT 'Store01',
    name TEXT,
    address TEXT,
    phone TEXT,
    nif TEXT,
    email TEXT,
    website TEXT,
    logo_url TEXT,
    business_day_cutoff INTEGER DEFAULT 0,
    currency_code TEXT,
    currency_symbol TEXT,
    currency_decimal_places INTEGER,
    timezone TEXT,
    receipt_header TEXT,
    receipt_footer TEXT,
    receipt_locale TEXT,
    last_sync_at BIGINT,
    last_daily_count INTEGER NOT NULL DEFAULT 0,
    last_business_date TEXT NOT NULL DEFAULT '',
    registered_at BIGINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    deleted_at BIGINT,
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
CREATE INDEX idx_store_bindings_owner
    ON store_attribute_bindings (store_id, owner_type, owner_source_id);

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
CREATE INDEX idx_store_price_rules_store ON store_price_rules(store_id);

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

-- ── Archived Orders ──

CREATE TABLE IF NOT EXISTS store_archived_orders (
    id BIGSERIAL PRIMARY KEY,
    store_id BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    tenant_id BIGINT NOT NULL,
    source_id BIGINT NOT NULL,
    order_id BIGINT NOT NULL,
    receipt_number TEXT,
    status TEXT NOT NULL,
    end_time BIGINT,
    total NUMERIC(12,2),
    tax NUMERIC(12,2),
    guest_count INTEGER,
    discount_amount NUMERIC(12,2) NOT NULL DEFAULT 0,
    void_type TEXT,
    loss_amount NUMERIC(12,2),
    start_time BIGINT,
    prev_hash TEXT,
    curr_hash TEXT,
    last_event_hash TEXT,
    version BIGINT NOT NULL DEFAULT 0,
    synced_at BIGINT NOT NULL,
    -- Denormalized fields (from 0006/0008/0009/0012)
    zone_name TEXT,
    table_name TEXT,
    is_retail BOOLEAN NOT NULL DEFAULT false,
    original_total NUMERIC(12,2) NOT NULL DEFAULT 0,
    subtotal NUMERIC(12,2) NOT NULL DEFAULT 0,
    paid_amount NUMERIC(12,2) NOT NULL DEFAULT 0,
    surcharge_amount NUMERIC(12,2) NOT NULL DEFAULT 0,
    comp_total_amount NUMERIC(12,2) NOT NULL DEFAULT 0,
    order_manual_discount_amount NUMERIC(12,2) NOT NULL DEFAULT 0,
    order_manual_surcharge_amount NUMERIC(12,2) NOT NULL DEFAULT 0,
    order_rule_discount_amount NUMERIC(12,2) NOT NULL DEFAULT 0,
    order_rule_surcharge_amount NUMERIC(12,2) NOT NULL DEFAULT 0,
    operator_name TEXT,
    loss_reason TEXT,
    void_note TEXT,
    member_name TEXT,
    service_type TEXT,
    created_at BIGINT,
    queue_number TEXT,
    shift_id BIGINT,
    operator_id BIGINT,
    member_id BIGINT,
    is_voided BOOLEAN NOT NULL DEFAULT false,
    is_upgraded BOOLEAN NOT NULL DEFAULT false,
    customer_nif TEXT,
    customer_nombre TEXT,
    customer_address TEXT,
    customer_email TEXT,
    customer_phone TEXT,
    mg_discount_amount NUMERIC(12,2) NOT NULL DEFAULT 0,
    marketing_group_name TEXT
);

CREATE UNIQUE INDEX uq_store_archived_orders_key
    ON store_archived_orders (tenant_id, store_id, order_id);
CREATE INDEX idx_store_archived_orders_tenant ON store_archived_orders (tenant_id);
CREATE INDEX idx_store_archived_orders_receipt ON store_archived_orders (tenant_id, receipt_number);
CREATE INDEX idx_store_archived_orders_end_time ON store_archived_orders (tenant_id, end_time);
CREATE INDEX idx_store_archived_orders_status ON store_archived_orders (tenant_id, status);
CREATE INDEX idx_store_archived_orders_list
    ON store_archived_orders (store_id, tenant_id, status, end_time DESC);
CREATE INDEX idx_archived_orders_overview
    ON store_archived_orders (tenant_id, end_time)
    INCLUDE (store_id, status, total, tax, guest_count,
             discount_amount, start_time, void_type, loss_amount)
    WHERE end_time IS NOT NULL;
CREATE INDEX idx_archived_orders_page
    ON store_archived_orders (store_id, tenant_id, end_time DESC NULLS LAST);
CREATE INDEX idx_sao_overview
    ON store_archived_orders(tenant_id, store_id, end_time)
    WHERE status = 'COMPLETED';

-- ── Order Items ──

CREATE TABLE store_order_items (
    id                BIGSERIAL PRIMARY KEY,
    order_id          BIGINT NOT NULL REFERENCES store_archived_orders(id) ON DELETE CASCADE,
    instance_id       TEXT NOT NULL DEFAULT '',
    name              TEXT NOT NULL,
    spec_name         TEXT,
    category_name     TEXT,
    product_source_id BIGINT,
    price             NUMERIC(12,2) NOT NULL DEFAULT 0,
    quantity          INTEGER NOT NULL DEFAULT 1,
    unit_price        NUMERIC(12,2) NOT NULL DEFAULT 0,
    line_total        NUMERIC(12,2) NOT NULL DEFAULT 0,
    discount_amount   NUMERIC(12,2) NOT NULL DEFAULT 0,
    surcharge_amount  NUMERIC(12,2) NOT NULL DEFAULT 0,
    tax               NUMERIC(12,2) NOT NULL DEFAULT 0,
    tax_rate          INTEGER NOT NULL DEFAULT 0,
    is_comped         BOOLEAN NOT NULL DEFAULT false,
    note              TEXT,
    rule_discount_amount   NUMERIC(12,2) NOT NULL DEFAULT 0,
    rule_surcharge_amount  NUMERIC(12,2) NOT NULL DEFAULT 0,
    mg_discount_amount     NUMERIC(12,2) NOT NULL DEFAULT 0
);
CREATE INDEX idx_soi_order ON store_order_items(order_id);
CREATE INDEX idx_soi_product ON store_order_items(product_source_id) WHERE product_source_id IS NOT NULL;

CREATE TABLE store_order_item_options (
    id              BIGSERIAL PRIMARY KEY,
    item_id         BIGINT NOT NULL REFERENCES store_order_items(id) ON DELETE CASCADE,
    attribute_name  TEXT NOT NULL,
    option_name     TEXT NOT NULL,
    price           NUMERIC(12,2) NOT NULL DEFAULT 0,
    quantity        INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_soio_item ON store_order_item_options(item_id);

-- ── Order Adjustments ──

CREATE TABLE store_order_adjustments (
    id                BIGSERIAL PRIMARY KEY,
    order_id          BIGINT NOT NULL REFERENCES store_archived_orders(id) ON DELETE CASCADE,
    item_id           BIGINT REFERENCES store_order_items(id) ON DELETE CASCADE,
    -- NULL item_id = order-level adjustment

    -- Source type: PRICE_RULE, MANUAL, MEMBER_GROUP, COMP
    source_type       TEXT NOT NULL,
    -- Direction: DISCOUNT or SURCHARGE
    direction         TEXT NOT NULL,

    -- Price rule specifics (NULL for non-rule sources)
    rule_id           BIGINT,
    rule_name         TEXT,
    rule_receipt_name TEXT,
    adjustment_type   TEXT,          -- PERCENTAGE / FIXED_AMOUNT (for rules)

    -- Amount
    amount            NUMERIC(12,2) NOT NULL DEFAULT 0,
    skipped           BOOLEAN NOT NULL DEFAULT false
);

CREATE INDEX idx_soa_order ON store_order_adjustments(order_id);
CREATE INDEX idx_soa_item ON store_order_adjustments(item_id) WHERE item_id IS NOT NULL;
CREATE INDEX idx_soa_source ON store_order_adjustments(order_id, source_type);

-- ── Order Payments ──

CREATE TABLE store_order_payments (
    id              BIGSERIAL PRIMARY KEY,
    order_id        BIGINT NOT NULL REFERENCES store_archived_orders(id) ON DELETE CASCADE,
    seq             INTEGER NOT NULL DEFAULT 0,
    payment_id      TEXT NOT NULL DEFAULT '',
    method          TEXT NOT NULL,
    amount          NUMERIC(12,2) NOT NULL DEFAULT 0,
    timestamp       BIGINT NOT NULL,
    cancelled       BOOLEAN NOT NULL DEFAULT false,
    cancel_reason   TEXT,
    tendered        NUMERIC(12,2),
    change_amount   NUMERIC(12,2)
);
CREATE INDEX idx_sop_order ON store_order_payments(order_id);
CREATE INDEX idx_sop_method ON store_order_payments(method);
CREATE INDEX idx_sop_active
    ON store_order_payments(order_id, method, amount)
    WHERE cancelled IS NOT TRUE;

-- ── Order Events ──

CREATE TABLE store_order_events (
    id              BIGSERIAL PRIMARY KEY,
    order_id        BIGINT NOT NULL REFERENCES store_archived_orders(id) ON DELETE CASCADE,
    seq             INTEGER NOT NULL DEFAULT 0,
    event_type      TEXT NOT NULL,
    timestamp       BIGINT NOT NULL,
    operator_id     BIGINT,
    operator_name   TEXT,
    data            TEXT
);
CREATE INDEX idx_soe_order ON store_order_events(order_id);
CREATE INDEX idx_soe_type ON store_order_events(event_type);

-- ── Order Desglose ──

CREATE TABLE store_order_desglose (
    id              BIGSERIAL PRIMARY KEY,
    order_id        BIGINT NOT NULL REFERENCES store_archived_orders(id) ON DELETE CASCADE,
    tax_rate        INTEGER NOT NULL,
    base_amount     NUMERIC(12,2) NOT NULL,
    tax_amount      NUMERIC(12,2) NOT NULL,
    UNIQUE(order_id, tax_rate)
);

-- ── Daily Reports ──

CREATE TABLE IF NOT EXISTS store_daily_reports (
    id               BIGSERIAL PRIMARY KEY,
    store_id         BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    tenant_id        BIGINT NOT NULL,
    source_id        BIGINT NOT NULL,
    business_date    TEXT NOT NULL,
    total_orders     BIGINT NOT NULL DEFAULT 0,
    net_revenue      DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    refund_amount    DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    refund_count     BIGINT NOT NULL DEFAULT 0,
    auto_generated   BOOLEAN NOT NULL DEFAULT FALSE,
    generated_at     BIGINT,
    generated_by_id  BIGINT,
    generated_by_name TEXT,
    note             TEXT,
    updated_at       BIGINT NOT NULL,
    UNIQUE (store_id, source_id),
    UNIQUE (store_id, business_date)
);

CREATE INDEX idx_store_daily_reports_store ON store_daily_reports(store_id);

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
    tenant_id BIGINT NOT NULL,
    command_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at BIGINT NOT NULL,
    executed_at BIGINT,
    result JSONB
);

CREATE INDEX idx_store_commands_pending
    ON store_commands (store_id, status) WHERE status = 'pending';
CREATE INDEX idx_store_commands_history
    ON store_commands (store_id, tenant_id, created_at DESC);

-- ── Shifts ──

CREATE TABLE IF NOT EXISTS store_shifts (
    id              BIGSERIAL PRIMARY KEY,
    store_id        BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    tenant_id       BIGINT NOT NULL,
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
CREATE INDEX idx_store_shifts_store ON store_shifts(store_id);
CREATE INDEX idx_store_shifts_status ON store_shifts(store_id, status);

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
CREATE INDEX idx_store_employees_store ON store_employees(store_id);

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
    tenant_id       BIGINT NOT NULL,
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
    source_type     TEXT,
    maintain_aspect_ratio BOOLEAN,
    style           TEXT,
    align           TEXT,
    vertical_align  TEXT,
    line_style      TEXT
);

CREATE INDEX idx_store_label_fields_template ON store_label_fields(template_id);

-- ── Pending Ops ──

CREATE TABLE store_pending_ops (
    id BIGSERIAL PRIMARY KEY,
    store_id BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    tenant_id BIGINT NOT NULL,
    op JSONB NOT NULL,
    changed_at BIGINT NOT NULL,
    created_at BIGINT NOT NULL
);

CREATE INDEX idx_pending_ops_store ON store_pending_ops(store_id);

-- ── Tenant Images ──

CREATE TABLE tenant_images (
    tenant_id   BIGINT  NOT NULL REFERENCES tenants(id),
    hash        TEXT    NOT NULL,
    ref_count   INTEGER NOT NULL DEFAULT 0,
    created_at  BIGINT  NOT NULL,
    orphaned_at BIGINT,
    PRIMARY KEY (tenant_id, hash)
);

CREATE INDEX idx_tenant_images_orphaned ON tenant_images (orphaned_at) WHERE orphaned_at IS NOT NULL;

-- ── Credit Notes ──

CREATE TABLE IF NOT EXISTS store_credit_notes (
    id BIGSERIAL PRIMARY KEY,
    store_id BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    tenant_id BIGINT NOT NULL,
    source_id BIGINT NOT NULL,
    credit_note_number TEXT NOT NULL,
    original_order_id BIGINT NOT NULL,
    original_receipt TEXT NOT NULL,
    subtotal_credit NUMERIC(12,2) NOT NULL,
    tax_credit NUMERIC(12,2) NOT NULL,
    total_credit NUMERIC(12,2) NOT NULL,
    refund_method TEXT NOT NULL,
    reason TEXT NOT NULL,
    note TEXT,
    operator_name TEXT NOT NULL,
    authorizer_name TEXT,
    prev_hash TEXT NOT NULL,
    curr_hash TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    synced_at BIGINT NOT NULL
);

CREATE UNIQUE INDEX uq_store_credit_notes_source
    ON store_credit_notes (tenant_id, store_id, source_id);
CREATE INDEX idx_store_credit_notes_order
    ON store_credit_notes (tenant_id, store_id, original_order_id);
CREATE INDEX idx_store_credit_notes_tenant
    ON store_credit_notes (tenant_id, created_at DESC);
CREATE INDEX idx_credit_notes_time
    ON store_credit_notes (tenant_id, store_id, created_at)
    INCLUDE (total_credit, refund_method);

-- ── Credit Note Items ──

CREATE TABLE store_credit_note_items (
    id              BIGSERIAL PRIMARY KEY,
    credit_note_id  BIGINT NOT NULL REFERENCES store_credit_notes(id) ON DELETE CASCADE,
    item_name       TEXT NOT NULL,
    quantity        INTEGER NOT NULL,
    unit_price      NUMERIC(12,2) NOT NULL,
    line_credit     NUMERIC(12,2) NOT NULL,
    tax_rate        INTEGER NOT NULL,
    tax_credit      NUMERIC(12,2) NOT NULL,
    original_instance_id TEXT
);
CREATE INDEX idx_scni_cn ON store_credit_note_items(credit_note_id);

-- ── Verifactu Invoices ──

CREATE TABLE IF NOT EXISTS store_invoices (
    id BIGSERIAL PRIMARY KEY,
    store_id BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    tenant_id BIGINT NOT NULL,
    source_id BIGINT NOT NULL,
    invoice_number TEXT NOT NULL,
    serie TEXT NOT NULL,
    tipo_factura TEXT NOT NULL,
    source_type TEXT NOT NULL,
    source_pk BIGINT NOT NULL,
    subtotal NUMERIC(12,2) NOT NULL,
    tax NUMERIC(12,2) NOT NULL,
    total NUMERIC(12,2) NOT NULL,
    huella TEXT NOT NULL,
    prev_huella TEXT,
    fecha_expedicion TEXT NOT NULL,
    fecha_hora_registro TEXT,
    nif TEXT NOT NULL,
    nombre_razon TEXT NOT NULL,
    factura_rectificada_id BIGINT,
    factura_rectificada_num TEXT,
    customer_nif TEXT,
    customer_nombre TEXT,
    customer_address TEXT,
    customer_email TEXT,
    customer_phone TEXT,
    factura_sustituida_id BIGINT,
    factura_sustituida_num TEXT,
    aeat_status TEXT NOT NULL DEFAULT 'PENDING',
    aeat_csv TEXT,
    aeat_submitted_at BIGINT,
    aeat_response_at BIGINT,
    created_at BIGINT NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    synced_at BIGINT NOT NULL
);

CREATE UNIQUE INDEX uq_store_invoices_source
    ON store_invoices (tenant_id, store_id, source_id);
CREATE UNIQUE INDEX uq_store_invoices_number
    ON store_invoices (store_id, invoice_number);
CREATE INDEX idx_store_invoices_tenant
    ON store_invoices (tenant_id, created_at DESC);
CREATE INDEX idx_store_invoices_aeat_pending
    ON store_invoices (store_id, aeat_status, created_at)
    WHERE aeat_status != 'ACCEPTED';
CREATE INDEX idx_store_invoices_order
    ON store_invoices (store_id, source_type, source_pk);

-- ── Invoice Desglose ──

CREATE TABLE store_invoice_desglose (
    id              BIGSERIAL PRIMARY KEY,
    invoice_id      BIGINT NOT NULL REFERENCES store_invoices(id) ON DELETE CASCADE,
    tax_rate        INTEGER NOT NULL,
    base_amount     NUMERIC(12,2) NOT NULL,
    tax_amount      NUMERIC(12,2) NOT NULL,
    UNIQUE(invoice_id, tax_rate)
);

-- ── Anulaciones ──

CREATE TABLE store_anulaciones (
    id              BIGSERIAL PRIMARY KEY,
    store_id        BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    tenant_id       BIGINT NOT NULL,
    source_id       BIGINT NOT NULL,
    anulacion_number TEXT NOT NULL,
    serie           TEXT NOT NULL,
    original_invoice_id BIGINT NOT NULL,
    original_invoice_number TEXT NOT NULL,
    original_order_id BIGINT NOT NULL,
    huella          TEXT NOT NULL,
    prev_huella     TEXT,
    fecha_expedicion TEXT NOT NULL,
    fecha_hora_registro TEXT NOT NULL,
    nif             TEXT NOT NULL,
    nombre_razon    TEXT NOT NULL,
    reason          TEXT NOT NULL,
    note            TEXT,
    operator_name   TEXT NOT NULL,
    operator_id     BIGINT NOT NULL DEFAULT 0,
    prev_hash       TEXT NOT NULL,
    curr_hash       TEXT NOT NULL,
    aeat_status     TEXT NOT NULL DEFAULT 'PENDING',
    version         BIGINT NOT NULL DEFAULT 0,
    created_at      BIGINT NOT NULL,
    synced_at       BIGINT NOT NULL,
    UNIQUE(tenant_id, store_id, source_id)
);

CREATE INDEX idx_store_anulaciones_store ON store_anulaciones(store_id);
CREATE INDEX idx_store_anulaciones_order ON store_anulaciones(store_id, original_order_id);
CREATE INDEX idx_store_anulaciones_tenant ON store_anulaciones(tenant_id, created_at DESC);

-- ── Chain Entries ──

CREATE TABLE store_chain_entries (
    id          BIGSERIAL PRIMARY KEY,
    store_id    BIGINT NOT NULL,
    tenant_id   BIGINT NOT NULL,
    source_id   BIGINT NOT NULL,
    entry_type  TEXT   NOT NULL,
    entry_pk    BIGINT NOT NULL,
    prev_hash   TEXT   NOT NULL,
    curr_hash   TEXT   NOT NULL,
    created_at  BIGINT NOT NULL,
    synced_at   BIGINT NOT NULL,
    UNIQUE(tenant_id, store_id, source_id)
);

CREATE INDEX idx_sce_store ON store_chain_entries(store_id, tenant_id);
CREATE INDEX idx_sce_created ON store_chain_entries(store_id, tenant_id, created_at DESC);
