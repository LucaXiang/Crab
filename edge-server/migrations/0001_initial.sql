-- ============================================================
-- Crab POS — SQLite initial schema
-- Money: REAL (f64), precision via rust_decimal in application
-- Timestamps: INTEGER (Unix milliseconds)
-- Nested objects: independent tables (no JSON blobs)
-- ============================================================

PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

-- ============================================================
-- Reference Data
-- ============================================================

CREATE TABLE role (
    id           INTEGER PRIMARY KEY,
    name         TEXT    NOT NULL,
    display_name TEXT    NOT NULL DEFAULT '',
    description  TEXT,
    permissions  TEXT    NOT NULL DEFAULT '[]',   -- JSON array of permission strings
    is_system    INTEGER NOT NULL DEFAULT 0,
    is_active    INTEGER NOT NULL DEFAULT 1
);
CREATE UNIQUE INDEX idx_role_name ON role(name);

CREATE TABLE employee (
    id           INTEGER PRIMARY KEY,
    username     TEXT    NOT NULL,
    hash_pass    TEXT    NOT NULL,
    display_name TEXT    NOT NULL DEFAULT '',
    role_id      INTEGER NOT NULL REFERENCES role(id),
    is_system    INTEGER NOT NULL DEFAULT 0,
    is_active    INTEGER NOT NULL DEFAULT 1,
    created_at   INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX idx_employee_username ON employee(username);

CREATE TABLE zone (
    id          INTEGER PRIMARY KEY,
    name        TEXT    NOT NULL,
    description TEXT,
    is_active   INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_zone_name ON zone(name);

CREATE TABLE dining_table (
    id        INTEGER PRIMARY KEY,
    name      TEXT    NOT NULL,
    zone_id   INTEGER NOT NULL REFERENCES zone(id),
    capacity  INTEGER NOT NULL DEFAULT 4,
    is_active INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_dining_table_zone ON dining_table(zone_id);
CREATE UNIQUE INDEX idx_dining_table_zone_name ON dining_table(zone_id, name);

CREATE TABLE tag (
    id            INTEGER PRIMARY KEY,
    name          TEXT    NOT NULL,
    color         TEXT    NOT NULL DEFAULT '#3B82F6',
    display_order INTEGER NOT NULL DEFAULT 0,
    is_active     INTEGER NOT NULL DEFAULT 1,
    is_system     INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX idx_tag_name ON tag(name);
CREATE INDEX idx_tag_display_order ON tag(display_order);

-- ── Print Destination + Printers ─────────────────────────────

CREATE TABLE print_destination (
    id          INTEGER PRIMARY KEY,
    name        TEXT    NOT NULL,
    description TEXT,
    is_active   INTEGER NOT NULL DEFAULT 1
);
CREATE UNIQUE INDEX idx_print_dest_name ON print_destination(name);

-- Printers: extracted from embedded array
CREATE TABLE printer (
    id                  INTEGER PRIMARY KEY,
    print_destination_id INTEGER NOT NULL REFERENCES print_destination(id) ON DELETE CASCADE,
    printer_type        TEXT    NOT NULL,     -- 'network' | 'driver'
    printer_format      TEXT    NOT NULL DEFAULT 'escpos',  -- 'escpos' | 'label'
    ip                  TEXT,
    port                INTEGER,
    driver_name         TEXT,
    priority            INTEGER NOT NULL DEFAULT 0,
    is_active           INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_printer_dest ON printer(print_destination_id);

-- ── Category ─────────────────────────────────────────────────

CREATE TABLE category (
    id                       INTEGER PRIMARY KEY,
    name                     TEXT    NOT NULL,
    sort_order               INTEGER NOT NULL DEFAULT 0,
    is_kitchen_print_enabled INTEGER NOT NULL DEFAULT 0,
    is_label_print_enabled   INTEGER NOT NULL DEFAULT 0,
    is_active                INTEGER NOT NULL DEFAULT 1,
    is_virtual               INTEGER NOT NULL DEFAULT 0,
    match_mode               TEXT    NOT NULL DEFAULT 'any',
    is_display               INTEGER NOT NULL DEFAULT 1
);
CREATE UNIQUE INDEX idx_category_name ON category(name);
CREATE INDEX idx_category_sort_order ON category(sort_order);

-- Category -> print_destination junction tables
CREATE TABLE category_kitchen_print_dest (
    category_id          INTEGER NOT NULL REFERENCES category(id) ON DELETE CASCADE,
    print_destination_id INTEGER NOT NULL REFERENCES print_destination(id) ON DELETE CASCADE,
    PRIMARY KEY (category_id, print_destination_id)
);

CREATE TABLE category_label_print_dest (
    category_id          INTEGER NOT NULL REFERENCES category(id) ON DELETE CASCADE,
    print_destination_id INTEGER NOT NULL REFERENCES print_destination(id) ON DELETE CASCADE,
    PRIMARY KEY (category_id, print_destination_id)
);

-- Category -> tag junction (for virtual category filtering)
CREATE TABLE category_tag (
    category_id INTEGER NOT NULL REFERENCES category(id) ON DELETE CASCADE,
    tag_id      INTEGER NOT NULL REFERENCES tag(id) ON DELETE CASCADE,
    PRIMARY KEY (category_id, tag_id)
);

-- ── Product ──────────────────────────────────────────────────

CREATE TABLE product (
    id                       INTEGER PRIMARY KEY,
    name                     TEXT    NOT NULL,
    image                    TEXT    NOT NULL DEFAULT '',
    category_id              INTEGER NOT NULL REFERENCES category(id),
    sort_order               INTEGER NOT NULL DEFAULT 0,
    tax_rate                 INTEGER NOT NULL DEFAULT 0,
    receipt_name             TEXT,
    kitchen_print_name       TEXT,
    is_kitchen_print_enabled INTEGER NOT NULL DEFAULT -1,
    is_label_print_enabled   INTEGER NOT NULL DEFAULT -1,
    is_active                INTEGER NOT NULL DEFAULT 1,
    external_id              INTEGER
);
CREATE INDEX idx_product_category ON product(category_id);
CREATE INDEX idx_product_sort_order ON product(sort_order);
CREATE UNIQUE INDEX idx_product_external_id ON product(external_id) WHERE external_id IS NOT NULL;

-- Product specs: extracted from embedded array
CREATE TABLE product_spec (
    id            INTEGER PRIMARY KEY,
    product_id    INTEGER NOT NULL REFERENCES product(id) ON DELETE CASCADE,
    name          TEXT    NOT NULL,
    price         REAL    NOT NULL DEFAULT 0.0,
    display_order INTEGER NOT NULL DEFAULT 0,
    is_default    INTEGER NOT NULL DEFAULT 0,
    is_active     INTEGER NOT NULL DEFAULT 1,
    receipt_name  TEXT,
    is_root       INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_product_spec_product ON product_spec(product_id);

-- Product -> tag junction table
CREATE TABLE product_tag (
    product_id INTEGER NOT NULL REFERENCES product(id) ON DELETE CASCADE,
    tag_id     INTEGER NOT NULL REFERENCES tag(id) ON DELETE CASCADE,
    PRIMARY KEY (product_id, tag_id)
);

-- ── Attribute ────────────────────────────────────────────────

CREATE TABLE attribute (
    id                     INTEGER PRIMARY KEY,
    name                   TEXT    NOT NULL,
    is_multi_select        INTEGER NOT NULL DEFAULT 0,
    max_selections         INTEGER,
    default_option_indices TEXT,                -- JSON array of int
    display_order          INTEGER NOT NULL DEFAULT 0,
    is_active              INTEGER NOT NULL DEFAULT 1,
    show_on_receipt        INTEGER NOT NULL DEFAULT 0,
    receipt_name           TEXT,
    show_on_kitchen_print  INTEGER NOT NULL DEFAULT 0,
    kitchen_print_name     TEXT
);
CREATE INDEX idx_attribute_display_order ON attribute(display_order);

-- Attribute options: extracted from embedded array
CREATE TABLE attribute_option (
    id                 INTEGER PRIMARY KEY,
    attribute_id       INTEGER NOT NULL REFERENCES attribute(id) ON DELETE CASCADE,
    name               TEXT    NOT NULL,
    price_modifier     REAL    NOT NULL DEFAULT 0.0,
    display_order      INTEGER NOT NULL DEFAULT 0,
    is_active          INTEGER NOT NULL DEFAULT 1,
    receipt_name       TEXT,
    kitchen_print_name TEXT,
    enable_quantity    INTEGER NOT NULL DEFAULT 0,
    max_quantity       INTEGER
);
CREATE INDEX idx_attribute_option_attribute ON attribute_option(attribute_id);

-- Attribute binding: replaces has_attribute graph edge
-- owner_type: 'product' or 'category'
CREATE TABLE attribute_binding (
    id                     INTEGER PRIMARY KEY,
    owner_type             TEXT    NOT NULL,     -- 'product' | 'category'
    owner_id               INTEGER NOT NULL,
    attribute_id           INTEGER NOT NULL REFERENCES attribute(id) ON DELETE CASCADE,
    is_required            INTEGER NOT NULL DEFAULT 0,
    display_order          INTEGER NOT NULL DEFAULT 0,
    default_option_indices TEXT                  -- JSON array of int
);
CREATE UNIQUE INDEX idx_attr_binding_unique ON attribute_binding(owner_type, owner_id, attribute_id);
CREATE INDEX idx_attr_binding_owner ON attribute_binding(owner_type, owner_id);

-- ── Price Rule ───────────────────────────────────────────────

CREATE TABLE price_rule (
    id                INTEGER PRIMARY KEY,
    name              TEXT    NOT NULL,
    display_name      TEXT    NOT NULL,
    receipt_name      TEXT    NOT NULL,
    description       TEXT,
    rule_type         TEXT    NOT NULL,          -- 'DISCOUNT' | 'SURCHARGE'
    product_scope     TEXT    NOT NULL,          -- 'GLOBAL' | 'PRODUCT' | 'CATEGORY' | 'TAG'
    target_id         INTEGER,                   -- FK depends on scope
    zone_scope        TEXT    NOT NULL DEFAULT 'all',
    adjustment_type   TEXT    NOT NULL,          -- 'PERCENTAGE' | 'FIXED_AMOUNT'
    adjustment_value  REAL    NOT NULL,          -- percentage: 30.0=30%, fixed: 5.00=€5
    is_stackable      INTEGER NOT NULL DEFAULT 0,
    is_exclusive      INTEGER NOT NULL DEFAULT 0,
    valid_from        INTEGER,
    valid_until       INTEGER,
    active_days       TEXT,                      -- JSON array of int (weekdays)
    active_start_time TEXT,
    active_end_time   TEXT,
    is_active         INTEGER NOT NULL DEFAULT 1,
    created_by        INTEGER REFERENCES employee(id),
    created_at        INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_price_rule_active ON price_rule(is_active, product_scope);

-- ── Singleton Tables ─────────────────────────────────────────

CREATE TABLE store_info (
    id                  INTEGER PRIMARY KEY,
    name                TEXT    NOT NULL DEFAULT '',
    address             TEXT    NOT NULL DEFAULT '',
    nif                 TEXT    NOT NULL DEFAULT '',
    logo_url            TEXT,
    phone               TEXT,
    email               TEXT,
    website             TEXT,
    business_day_cutoff TEXT    NOT NULL DEFAULT '00:00',
    created_at          INTEGER,
    updated_at          INTEGER
);

CREATE TABLE system_state (
    id                 INTEGER PRIMARY KEY,
    genesis_hash       TEXT,
    last_order_id      TEXT,
    last_order_hash    TEXT,
    synced_up_to_id    TEXT,
    synced_up_to_hash  TEXT,
    last_sync_time     INTEGER,
    order_count        INTEGER NOT NULL DEFAULT 0,
    created_at         INTEGER NOT NULL DEFAULT 0,
    updated_at         INTEGER NOT NULL DEFAULT 0
);

-- ── Label Template + Fields ──────────────────────────────────

CREATE TABLE label_template (
    id           INTEGER PRIMARY KEY,
    name         TEXT    NOT NULL,
    description  TEXT,
    width        REAL    NOT NULL,
    height       REAL    NOT NULL,
    padding      REAL    NOT NULL DEFAULT 2.0,
    is_default   INTEGER NOT NULL DEFAULT 0,
    is_active    INTEGER NOT NULL DEFAULT 1,
    width_mm     REAL,
    height_mm    REAL,
    padding_mm_x REAL,
    padding_mm_y REAL,
    render_dpi   INTEGER,
    test_data    TEXT,
    created_at   INTEGER,
    updated_at   INTEGER
);
CREATE INDEX idx_label_template_name ON label_template(name);
CREATE INDEX idx_label_template_active ON label_template(is_active);

-- Label fields: extracted from embedded array
CREATE TABLE label_field (
    id                   INTEGER PRIMARY KEY,
    template_id          INTEGER NOT NULL REFERENCES label_template(id) ON DELETE CASCADE,
    field_id             TEXT    NOT NULL,       -- client-generated UUID
    name                 TEXT    NOT NULL,
    field_type           TEXT    NOT NULL DEFAULT 'text',
    x                    REAL    NOT NULL DEFAULT 0.0,
    y                    REAL    NOT NULL DEFAULT 0.0,
    width                REAL    NOT NULL DEFAULT 0.0,
    height               REAL    NOT NULL DEFAULT 0.0,
    font_size            INTEGER NOT NULL DEFAULT 10,
    font_weight          TEXT,
    font_family          TEXT,
    color                TEXT,
    rotate               INTEGER,
    alignment            TEXT,
    data_source          TEXT    NOT NULL,
    format               TEXT,
    visible              INTEGER NOT NULL DEFAULT 1,
    label                TEXT,
    template             TEXT,
    data_key             TEXT,
    source_type          TEXT,
    maintain_aspect_ratio INTEGER,
    style                TEXT,
    align                TEXT,
    vertical_align       TEXT,
    line_style           TEXT
);
CREATE INDEX idx_label_field_template ON label_field(template_id);

-- ── Image Ref ────────────────────────────────────────────────

CREATE TABLE image_ref (
    id          INTEGER PRIMARY KEY,
    hash        TEXT    NOT NULL,
    entity_type TEXT    NOT NULL,
    entity_id   INTEGER NOT NULL,
    created_at  INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX idx_image_ref_unique ON image_ref(hash, entity_type, entity_id);
CREATE INDEX idx_image_ref_hash ON image_ref(hash);
CREATE INDEX idx_image_ref_entity ON image_ref(entity_type, entity_id);

-- ── Shift ────────────────────────────────────────────────────

CREATE TABLE shift (
    id             INTEGER PRIMARY KEY,
    operator_id    INTEGER NOT NULL REFERENCES employee(id),
    operator_name  TEXT    NOT NULL,
    status         TEXT    NOT NULL DEFAULT 'OPEN',
    start_time     INTEGER NOT NULL,
    end_time       INTEGER,
    starting_cash  REAL    NOT NULL DEFAULT 0.0,
    expected_cash  REAL    NOT NULL DEFAULT 0.0,
    actual_cash    REAL,
    cash_variance  REAL,
    abnormal_close INTEGER NOT NULL DEFAULT 0,
    last_active_at INTEGER,
    note           TEXT,
    created_at     INTEGER,
    updated_at     INTEGER
);
CREATE INDEX idx_shift_status ON shift(status);
CREATE INDEX idx_shift_operator ON shift(operator_id);
CREATE INDEX idx_shift_start_time ON shift(start_time);

-- ── Daily Report + Breakdowns ────────────────────────────────

CREATE TABLE daily_report (
    id                INTEGER PRIMARY KEY,
    business_date     TEXT    NOT NULL,
    total_orders      INTEGER NOT NULL DEFAULT 0,
    completed_orders  INTEGER NOT NULL DEFAULT 0,
    void_orders       INTEGER NOT NULL DEFAULT 0,
    total_sales       REAL    NOT NULL DEFAULT 0.0,
    total_paid        REAL    NOT NULL DEFAULT 0.0,
    total_unpaid      REAL    NOT NULL DEFAULT 0.0,
    void_amount       REAL    NOT NULL DEFAULT 0.0,
    total_tax         REAL    NOT NULL DEFAULT 0.0,
    total_discount    REAL    NOT NULL DEFAULT 0.0,
    total_surcharge   REAL    NOT NULL DEFAULT 0.0,
    generated_at      INTEGER,
    generated_by_id   INTEGER,
    generated_by_name TEXT,
    note              TEXT
);
CREATE UNIQUE INDEX idx_daily_report_date ON daily_report(business_date);

-- Tax breakdowns: extracted from embedded array
CREATE TABLE daily_report_tax_breakdown (
    id            INTEGER PRIMARY KEY,
    report_id     INTEGER NOT NULL REFERENCES daily_report(id) ON DELETE CASCADE,
    tax_rate      INTEGER NOT NULL,
    net_amount    REAL    NOT NULL DEFAULT 0.0,
    tax_amount    REAL    NOT NULL DEFAULT 0.0,
    gross_amount  REAL    NOT NULL DEFAULT 0.0,
    order_count   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_tax_breakdown_report ON daily_report_tax_breakdown(report_id);

-- Payment breakdowns: extracted from embedded array
CREATE TABLE daily_report_payment_breakdown (
    id        INTEGER PRIMARY KEY,
    report_id INTEGER NOT NULL REFERENCES daily_report(id) ON DELETE CASCADE,
    method    TEXT    NOT NULL,
    amount    REAL    NOT NULL DEFAULT 0.0,
    count     INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_payment_breakdown_report ON daily_report_payment_breakdown(report_id);

-- ── System Issue ─────────────────────────────────────────────

CREATE TABLE system_issue (
    id          INTEGER PRIMARY KEY,
    source      TEXT    NOT NULL,
    kind        TEXT    NOT NULL,
    blocking    INTEGER NOT NULL,
    target      TEXT,
    params      TEXT    NOT NULL DEFAULT '{}',   -- JSON object (flexible schema)
    title       TEXT,
    description TEXT,
    options     TEXT    NOT NULL DEFAULT '[]',   -- JSON array of strings
    status      TEXT    NOT NULL DEFAULT 'pending',
    response    TEXT,
    resolved_by TEXT,
    resolved_at INTEGER,
    created_at  INTEGER NOT NULL
);
CREATE INDEX idx_system_issue_status ON system_issue(status);
CREATE INDEX idx_system_issue_kind ON system_issue(kind);
CREATE INDEX idx_system_issue_source ON system_issue(source);

-- ============================================================
-- Archive Data (orders written by ArchiveWorker)
-- ============================================================

CREATE TABLE archived_order (
    id                              INTEGER PRIMARY KEY,
    receipt_number                  TEXT    NOT NULL,
    zone_name                       TEXT,
    table_name                      TEXT,
    status                          TEXT    NOT NULL,
    is_retail                       INTEGER NOT NULL DEFAULT 0,
    guest_count                     INTEGER,
    original_total                  REAL    NOT NULL DEFAULT 0.0,
    subtotal                        REAL    NOT NULL DEFAULT 0.0,
    total_amount                    REAL    NOT NULL DEFAULT 0.0,
    paid_amount                     REAL    NOT NULL DEFAULT 0.0,
    discount_amount                 REAL    NOT NULL DEFAULT 0.0,
    surcharge_amount                REAL    NOT NULL DEFAULT 0.0,
    comp_total_amount               REAL    NOT NULL DEFAULT 0.0,
    order_manual_discount_amount    REAL    NOT NULL DEFAULT 0.0,
    order_manual_surcharge_amount   REAL    NOT NULL DEFAULT 0.0,
    order_rule_discount_amount      REAL    NOT NULL DEFAULT 0.0,
    order_rule_surcharge_amount     REAL    NOT NULL DEFAULT 0.0,
    tax                             REAL    NOT NULL DEFAULT 0.0,
    start_time                      INTEGER NOT NULL,
    end_time                        INTEGER,
    operator_id                     INTEGER,
    operator_name                   TEXT,
    void_type                       TEXT,
    loss_reason                     TEXT,
    loss_amount                     REAL,
    void_note                       TEXT,
    related_order_id                TEXT,
    prev_hash                       TEXT    NOT NULL,
    curr_hash                       TEXT    NOT NULL,
    created_at                      INTEGER NOT NULL
);
CREATE UNIQUE INDEX idx_archived_order_receipt ON archived_order(receipt_number);
CREATE INDEX idx_archived_order_status ON archived_order(status);
CREATE INDEX idx_archived_order_end_time ON archived_order(end_time);
CREATE INDEX idx_archived_order_hash ON archived_order(curr_hash);
CREATE INDEX idx_archived_order_status_end ON archived_order(status, end_time);
CREATE INDEX idx_archived_order_created ON archived_order(created_at);

CREATE TABLE archived_order_item (
    id                     INTEGER PRIMARY KEY,
    order_pk               INTEGER NOT NULL REFERENCES archived_order(id),
    spec                   TEXT    NOT NULL,
    instance_id            TEXT    NOT NULL,
    name                   TEXT    NOT NULL,
    spec_name              TEXT,
    price                  REAL    NOT NULL DEFAULT 0.0,
    quantity               INTEGER NOT NULL DEFAULT 1,
    unpaid_quantity        INTEGER NOT NULL DEFAULT 0,
    unit_price             REAL    NOT NULL DEFAULT 0.0,
    line_total             REAL    NOT NULL DEFAULT 0.0,
    discount_amount        REAL    NOT NULL DEFAULT 0.0,
    surcharge_amount       REAL    NOT NULL DEFAULT 0.0,
    rule_discount_amount   REAL    NOT NULL DEFAULT 0.0,
    rule_surcharge_amount  REAL    NOT NULL DEFAULT 0.0,
    tax                    REAL    NOT NULL DEFAULT 0.0,
    tax_rate               INTEGER NOT NULL DEFAULT 0,
    category_name          TEXT,
    applied_rules          TEXT,        -- JSON string (AppliedRule array)
    note                   TEXT,
    is_comped              INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_archived_item_order ON archived_order_item(order_pk);
CREATE INDEX idx_archived_item_spec ON archived_order_item(spec);
CREATE INDEX idx_archived_item_instance ON archived_order_item(instance_id);

CREATE TABLE archived_order_item_option (
    id              INTEGER PRIMARY KEY,
    item_pk         INTEGER NOT NULL REFERENCES archived_order_item(id),
    attribute_name  TEXT    NOT NULL,
    option_name     TEXT    NOT NULL,
    price           REAL    NOT NULL DEFAULT 0.0,
    quantity        INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_archived_option_item ON archived_order_item_option(item_pk);

CREATE TABLE archived_order_payment (
    id              INTEGER PRIMARY KEY,
    order_pk        INTEGER NOT NULL REFERENCES archived_order(id),
    seq             INTEGER NOT NULL DEFAULT 0,
    payment_id      TEXT    NOT NULL,
    method          TEXT    NOT NULL,
    amount          REAL    NOT NULL DEFAULT 0.0,
    time            INTEGER NOT NULL,
    cancelled       INTEGER NOT NULL DEFAULT 0,
    cancel_reason   TEXT,
    tendered        REAL,
    change_amount   REAL,
    split_type      TEXT,
    split_items     TEXT,       -- JSON string (SplitItem array)
    aa_shares       INTEGER,
    aa_total_shares INTEGER
);
CREATE INDEX idx_archived_payment_order ON archived_order_payment(order_pk);
CREATE INDEX idx_archived_payment_method ON archived_order_payment(method);
CREATE INDEX idx_archived_payment_time ON archived_order_payment(time);

CREATE TABLE archived_order_event (
    id          INTEGER PRIMARY KEY,
    order_pk    INTEGER NOT NULL REFERENCES archived_order(id),
    seq         INTEGER NOT NULL DEFAULT 0,
    event_type  TEXT    NOT NULL,
    timestamp   INTEGER NOT NULL,
    data        TEXT,           -- JSON string (event payload)
    prev_hash   TEXT    NOT NULL,
    curr_hash   TEXT    NOT NULL
);
CREATE INDEX idx_archived_event_order ON archived_order_event(order_pk);
CREATE INDEX idx_archived_event_time ON archived_order_event(timestamp);

-- Independent payment table for statistics
CREATE TABLE payment (
    id            INTEGER PRIMARY KEY,
    payment_id    TEXT    NOT NULL,
    order_id      TEXT    NOT NULL,
    method        TEXT    NOT NULL,
    amount        REAL    NOT NULL,
    tendered      REAL,
    change_amount REAL,
    note          TEXT,
    split_type    TEXT,
    aa_shares     INTEGER,
    split_items   TEXT,            -- JSON string
    operator_id   INTEGER,
    operator_name TEXT,
    cancelled     INTEGER NOT NULL DEFAULT 0,
    cancel_reason TEXT,
    timestamp     INTEGER NOT NULL,
    created_at    INTEGER NOT NULL
);
CREATE UNIQUE INDEX idx_payment_id ON payment(payment_id);
CREATE INDEX idx_payment_order ON payment(order_id);
CREATE INDEX idx_payment_timestamp ON payment(timestamp);
CREATE INDEX idx_payment_operator ON payment(operator_id);

-- ── Archive Verification ─────────────────────────────────────

CREATE TABLE archive_verification (
    id                   INTEGER PRIMARY KEY,
    verification_type    TEXT    NOT NULL,
    date                 TEXT,
    total_orders         INTEGER NOT NULL DEFAULT 0,
    verified_orders      INTEGER NOT NULL DEFAULT 0,
    chain_intact         INTEGER NOT NULL DEFAULT 1,
    chain_resets_count   INTEGER NOT NULL DEFAULT 0,
    chain_breaks_count   INTEGER NOT NULL DEFAULT 0,
    invalid_orders_count INTEGER NOT NULL DEFAULT 0,
    details              TEXT,       -- JSON object
    created_at           INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_av_type ON archive_verification(verification_type);
CREATE INDEX idx_av_date ON archive_verification(date);
CREATE INDEX idx_av_created ON archive_verification(created_at);
CREATE INDEX idx_av_intact ON archive_verification(chain_intact);
CREATE UNIQUE INDEX idx_av_type_date ON archive_verification(verification_type, date);

-- ── Audit Log (append-only) ──────────────────────────────────

CREATE TABLE audit_log (
    id            INTEGER PRIMARY KEY,
    sequence      INTEGER NOT NULL,
    timestamp     INTEGER NOT NULL,
    action        TEXT    NOT NULL,
    resource_type TEXT    NOT NULL,
    resource_id   TEXT    NOT NULL,
    operator_id   INTEGER,
    operator_name TEXT,
    details       TEXT    NOT NULL DEFAULT '{}',  -- JSON object
    target        TEXT,
    prev_hash     TEXT    NOT NULL,
    curr_hash     TEXT    NOT NULL
);
CREATE UNIQUE INDEX idx_audit_sequence ON audit_log(sequence);
CREATE INDEX idx_audit_timestamp ON audit_log(timestamp);
CREATE INDEX idx_audit_action ON audit_log(action);
CREATE INDEX idx_audit_operator ON audit_log(operator_id);
CREATE INDEX idx_audit_resource_type ON audit_log(resource_type);

-- ============================================================
-- Extra FK Indexes + Safety Constraints
-- ============================================================

CREATE INDEX IF NOT EXISTS idx_employee_role ON employee(role_id);
CREATE INDEX IF NOT EXISTS idx_attr_binding_attribute ON attribute_binding(attribute_id);
CREATE INDEX IF NOT EXISTS idx_price_rule_creator ON price_rule(created_by);
-- Global single shift: only one OPEN shift allowed at a time
CREATE UNIQUE INDEX IF NOT EXISTS idx_shift_single_open
    ON shift(status) WHERE status = 'OPEN';

-- ============================================================
-- Seed Data
-- ============================================================

-- Admin role + user (password: 'admin')
INSERT INTO role (id, name, display_name, description, permissions, is_system, is_active)
VALUES (1, 'admin', 'admin', 'administrator', '["*"]', 1, 1);

INSERT INTO employee (id, username, hash_pass, display_name, role_id, is_system, is_active, created_at)
VALUES (1, 'admin', '$argon2id$v=19$m=19456,t=2,p=1$4K7SyBwr5d3uF4hroPQf2w$hPqq7x5rSE1d9TTf+hK3eipuaeeElC7GMHuSJIozDws', 'admin', 1, 1, 1, 0);

-- Store info + system state
INSERT INTO store_info (id, name, address, nif, business_day_cutoff, created_at, updated_at)
VALUES (1, '', '', '', '00:00', 0, 0);

INSERT INTO system_state (id, order_count, created_at, updated_at)
VALUES (1, 0, 0, 0);

-- Zonas
INSERT INTO zone (id, name, description, is_active) VALUES (1, 'Salón', 'Sala principal', 1);
INSERT INTO zone (id, name, description, is_active) VALUES (2, 'Terraza', 'Terraza exterior', 1);
INSERT INTO zone (id, name, description, is_active) VALUES (3, 'Barra', 'Zona de barra', 1);

-- Mesas: M=Mesa, T=Terraza, B=Barra
INSERT INTO dining_table (id, name, zone_id, capacity, is_active) VALUES (1, 'M1', 1, 4, 1);
INSERT INTO dining_table (id, name, zone_id, capacity, is_active) VALUES (2, 'M2', 1, 4, 1);
INSERT INTO dining_table (id, name, zone_id, capacity, is_active) VALUES (3, 'M3', 1, 6, 1);
INSERT INTO dining_table (id, name, zone_id, capacity, is_active) VALUES (4, 'M4', 1, 2, 1);
INSERT INTO dining_table (id, name, zone_id, capacity, is_active) VALUES (5, 'M5', 1, 8, 1);
INSERT INTO dining_table (id, name, zone_id, capacity, is_active) VALUES (6, 'T1', 2, 4, 1);
INSERT INTO dining_table (id, name, zone_id, capacity, is_active) VALUES (7, 'T2', 2, 4, 1);
INSERT INTO dining_table (id, name, zone_id, capacity, is_active) VALUES (8, 'T3', 2, 6, 1);
INSERT INTO dining_table (id, name, zone_id, capacity, is_active) VALUES (9, 'B1', 3, 2, 1);
INSERT INTO dining_table (id, name, zone_id, capacity, is_active) VALUES (10, 'B2', 3, 2, 1);

-- Categorías
INSERT INTO category (id, name, sort_order, is_kitchen_print_enabled, is_label_print_enabled, is_active) VALUES (1, 'Tapas y Raciones', 1, 1, 0, 1);
INSERT INTO category (id, name, sort_order, is_kitchen_print_enabled, is_label_print_enabled, is_active) VALUES (2, 'Platos Principales', 2, 1, 0, 1);
INSERT INTO category (id, name, sort_order, is_kitchen_print_enabled, is_label_print_enabled, is_active) VALUES (3, 'Postres', 3, 1, 0, 1);
INSERT INTO category (id, name, sort_order, is_kitchen_print_enabled, is_label_print_enabled, is_active) VALUES (4, 'Cafés', 4, 0, 0, 1);
INSERT INTO category (id, name, sort_order, is_kitchen_print_enabled, is_label_print_enabled, is_active) VALUES (5, 'Bebidas', 5, 0, 0, 1);

-- Productos (IVA: 10% comida/bebidas sin alcohol, 21% alcohol)
-- Tapas y Raciones
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (1, 'Patatas Bravas', 1, 1, 10, 1, 1);
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (2, 'Tortilla Española', 1, 2, 10, 1, 2);
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (3, 'Jamón Ibérico', 1, 3, 10, 1, 3);
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (4, 'Croquetas Caseras', 1, 4, 10, 1, 4);
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (5, 'Gambas al Ajillo', 1, 5, 10, 1, 5);
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (6, 'Pimientos de Padrón', 1, 6, 10, 1, 6);
-- Platos Principales
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (7, 'Paella Valenciana', 2, 1, 10, 1, 10);
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (8, 'Solomillo a la Plancha', 2, 2, 10, 1, 11);
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (9, 'Merluza a la Vasca', 2, 3, 10, 1, 12);
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (10, 'Secreto Ibérico', 2, 4, 10, 1, 13);
-- Postres
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (11, 'Crema Catalana', 3, 1, 10, 1, 20);
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (12, 'Tarta de Santiago', 3, 2, 10, 1, 21);
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (13, 'Churros con Chocolate', 3, 3, 10, 1, 22);
-- Cafés
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (14, 'Café Solo', 4, 1, 10, 1, 30);
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (15, 'Café con Leche', 4, 2, 10, 1, 31);
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (16, 'Cortado', 4, 3, 10, 1, 32);
-- Bebidas (sin alcohol 10%, con alcohol 21%)
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (17, 'Agua Mineral', 5, 1, 10, 1, 40);
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (18, 'Refresco', 5, 2, 10, 1, 41);
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (19, 'Zumo Natural', 5, 3, 10, 1, 42);
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (20, 'Caña', 5, 4, 21, 1, 43);
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (21, 'Copa de Vino Tinto', 5, 5, 21, 1, 44);
INSERT INTO product (id, name, category_id, sort_order, tax_rate, is_active, external_id) VALUES (22, 'Copa de Vino Blanco', 5, 6, 21, 1, 45);

-- Precios (主规格用空白名表示 default)
-- Tapas y Raciones
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (1, 1, '', 4.50, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (2, 2, '', 5.50, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (3, 3, '', 14.00, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (4, 4, '', 6.50, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (5, 5, '', 9.50, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (6, 6, '', 5.50, 1, 1);
-- Platos Principales
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (7, 7, '', 14.50, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (8, 8, '', 18.50, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (9, 9, '', 15.50, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (10, 10, '', 16.00, 1, 1);
-- Postres
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (11, 11, '', 5.50, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (12, 12, '', 5.00, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (13, 13, '', 4.50, 1, 1);
-- Cafés
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (14, 14, '', 1.30, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (15, 15, '', 1.60, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (16, 16, '', 1.40, 1, 1);
-- Bebidas (有多规格的用命名规格)
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (17, 17, '0.5L', 1.50, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, display_order, is_default, is_root) VALUES (18, 17, '1L', 2.50, 1, 0, 0);
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (19, 18, '', 2.50, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (20, 19, '', 3.50, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (21, 20, '', 2.00, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, display_order, is_default, is_root) VALUES (22, 20, 'Jarra', 5.00, 1, 0, 0);
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (23, 21, 'Copa', 3.00, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, display_order, is_default, is_root) VALUES (24, 21, 'Botella', 15.00, 1, 0, 0);
INSERT INTO product_spec (id, product_id, name, price, is_default, is_root) VALUES (25, 22, 'Copa', 3.00, 1, 1);
INSERT INTO product_spec (id, product_id, name, price, display_order, is_default, is_root) VALUES (26, 22, 'Botella', 14.00, 1, 0, 0);

-- Atributos
-- 1: Punto de carne (single select, cocina) — 无加价
INSERT INTO attribute (id, name, is_multi_select, display_order, is_active, show_on_receipt, show_on_kitchen_print)
VALUES (1, 'Punto', 0, 1, 1, 1, 1);
INSERT INTO attribute_option (id, attribute_id, name, price_modifier, display_order) VALUES (1, 1, 'Poco hecho', 0.0, 1);
INSERT INTO attribute_option (id, attribute_id, name, price_modifier, display_order) VALUES (2, 1, 'Al punto', 0.0, 2);
INSERT INTO attribute_option (id, attribute_id, name, price_modifier, display_order) VALUES (3, 1, 'Muy hecho', 0.0, 3);

-- 2: Complementos de bebida (multi select, ticket + cocina) — 小额加价
INSERT INTO attribute (id, name, is_multi_select, display_order, is_active, show_on_receipt, show_on_kitchen_print)
VALUES (2, 'Complementos', 1, 2, 1, 1, 1);
INSERT INTO attribute_option (id, attribute_id, name, price_modifier, display_order) VALUES (4, 2, 'Con hielo', 0.10, 1);
INSERT INTO attribute_option (id, attribute_id, name, price_modifier, display_order) VALUES (5, 2, 'Con limón', 0.10, 2);
INSERT INTO attribute_option (id, attribute_id, name, price_modifier, display_order) VALUES (6, 2, 'Con aceituna', 0.10, 3);

-- 3: Tipo de Leche (single select, cocina)
INSERT INTO attribute (id, name, is_multi_select, display_order, is_active, show_on_receipt, show_on_kitchen_print)
VALUES (3, 'Tipo de Leche', 0, 3, 1, 0, 1);
INSERT INTO attribute_option (id, attribute_id, name, price_modifier, display_order) VALUES (7, 3, 'Normal', 0.0, 1);
INSERT INTO attribute_option (id, attribute_id, name, price_modifier, display_order) VALUES (8, 3, 'Desnatada', 0.0, 2);
INSERT INTO attribute_option (id, attribute_id, name, price_modifier, display_order) VALUES (9, 3, 'Avena', 0.30, 3);

-- 4: Extras de tapas (multi select, cocina + ticket) — 加配料
INSERT INTO attribute (id, name, is_multi_select, display_order, is_active, show_on_receipt, show_on_kitchen_print)
VALUES (4, 'Extras', 1, 4, 1, 1, 1);
INSERT INTO attribute_option (id, attribute_id, name, price_modifier, display_order) VALUES (10, 4, 'Pan', 0.50, 1);
INSERT INTO attribute_option (id, attribute_id, name, price_modifier, display_order) VALUES (11, 4, 'Alioli', 0.50, 2);
INSERT INTO attribute_option (id, attribute_id, name, price_modifier, display_order) VALUES (12, 4, 'Queso extra', 0.80, 3);

-- Vinculaciones de atributos
-- Punto → Solomillo (product 8), Secreto Ibérico (product 10)
INSERT INTO attribute_binding (id, owner_type, owner_id, attribute_id, is_required, display_order, default_option_indices)
VALUES (1, 'product', 8, 1, 1, 1, '[1]');
INSERT INTO attribute_binding (id, owner_type, owner_id, attribute_id, is_required, display_order, default_option_indices)
VALUES (2, 'product', 10, 1, 1, 1, '[1]');
-- Complementos → Bebidas (category 5): 加冰/柠檬/橄榄
INSERT INTO attribute_binding (id, owner_type, owner_id, attribute_id, is_required, display_order)
VALUES (3, 'category', 5, 2, 0, 1);
-- Tipo de Leche → Café con Leche (product 15), Cortado (product 16)
INSERT INTO attribute_binding (id, owner_type, owner_id, attribute_id, is_required, display_order, default_option_indices)
VALUES (4, 'product', 15, 3, 0, 1, '[0]');
INSERT INTO attribute_binding (id, owner_type, owner_id, attribute_id, is_required, display_order, default_option_indices)
VALUES (5, 'product', 16, 3, 0, 1, '[0]');
-- Extras → Tapas y Raciones (category 1): 加面包/蒜泥蛋黄酱/芝士
INSERT INTO attribute_binding (id, owner_type, owner_id, attribute_id, is_required, display_order)
VALUES (6, 'category', 1, 4, 0, 2);

-- Reglas de precio: Terraza 附加费
-- 1: Terraza 全品 +€0.30 (GLOBAL scope, zone_scope=Terraza zone_id=2)
INSERT INTO price_rule (id, name, display_name, receipt_name, description, rule_type, product_scope, zone_scope, adjustment_type, adjustment_value, is_stackable, is_exclusive, is_active, created_at)
VALUES (1, 'terraza_surcharge', 'Supl. Terraza', 'TERRAZA', 'Suplemento terraza para todos los productos', 'SURCHARGE', 'GLOBAL', '2', 'FIXED_AMOUNT', 0.30, 0, 0, 1, 0);
-- 2: Terraza 酒水 +€0.20 (CATEGORY scope, target_id=5 Bebidas, zone_scope=Terraza)
--    CATEGORY 优先级 (11) > GLOBAL (10)，non-stackable winner 机制自动覆盖
INSERT INTO price_rule (id, name, display_name, receipt_name, description, rule_type, product_scope, target_id, zone_scope, adjustment_type, adjustment_value, is_stackable, is_exclusive, is_active, created_at)
VALUES (2, 'terraza_bebidas_surcharge', 'Supl. Terraza Bebidas', 'TERRAZA BEB', 'Suplemento terraza para bebidas (prioridad sobre regla global)', 'SURCHARGE', 'CATEGORY', 5, '2', 'FIXED_AMOUNT', 0.20, 0, 0, 1, 0);
