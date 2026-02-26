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
    name         TEXT    NOT NULL DEFAULT '',
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
    purpose     TEXT    NOT NULL DEFAULT 'kitchen',  -- 'kitchen' | 'label'
    is_active   INTEGER NOT NULL DEFAULT 1
);
CREATE UNIQUE INDEX idx_print_dest_name ON print_destination(name);

CREATE TABLE printer (
    id                   INTEGER PRIMARY KEY,
    print_destination_id INTEGER NOT NULL REFERENCES print_destination(id) ON DELETE CASCADE,
    connection           TEXT    NOT NULL,     -- 'network' | 'driver'
    protocol             TEXT    NOT NULL DEFAULT 'escpos',  -- 'escpos' | 'label'
    ip                   TEXT,
    port                 INTEGER,
    driver_name          TEXT,
    priority             INTEGER NOT NULL DEFAULT 0,
    is_active            INTEGER NOT NULL DEFAULT 1
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

-- Category -> print_destination junction table (unified)
CREATE TABLE category_print_dest (
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
    default_option_ids TEXT,                -- JSON array of option IDs
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
    default_option_ids TEXT                  -- JSON array of option IDs
);
CREATE UNIQUE INDEX idx_attr_binding_unique ON attribute_binding(owner_type, owner_id, attribute_id);
CREATE INDEX idx_attr_binding_owner ON attribute_binding(owner_type, owner_id);

-- ── Price Rule ───────────────────────────────────────────────

CREATE TABLE price_rule (
    id                INTEGER PRIMARY KEY,
    name              TEXT    NOT NULL,
    receipt_name      TEXT,
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

-- ── Marketing Groups (营销组 = 会员等级) ─────────────────────

CREATE TABLE marketing_group (
    id               INTEGER PRIMARY KEY,
    name             TEXT    NOT NULL UNIQUE,
    description      TEXT,
    sort_order       INTEGER NOT NULL DEFAULT 0,
    points_earn_rate REAL,
    created_at       INTEGER NOT NULL DEFAULT 0,
    updated_at       INTEGER NOT NULL DEFAULT 0
);

-- ── Members (会员) ──────────────────────────────────────────

CREATE TABLE member (
    id                 INTEGER PRIMARY KEY,
    name               TEXT    NOT NULL,
    phone              TEXT,
    card_number        TEXT,
    marketing_group_id INTEGER NOT NULL REFERENCES marketing_group(id),
    birthday           TEXT,
    email              TEXT,
    points_balance     INTEGER NOT NULL DEFAULT 0,
    total_spent        REAL    NOT NULL DEFAULT 0,
    notes              TEXT,
    is_active          INTEGER NOT NULL DEFAULT 1,
    created_at         INTEGER NOT NULL DEFAULT 0,
    updated_at         INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_member_phone ON member(phone);
CREATE INDEX idx_member_card_number ON member(card_number);
CREATE INDEX idx_member_marketing_group ON member(marketing_group_id);

-- ── MG Discount Rules (MG 折扣规则) ────────────────────────

CREATE TABLE mg_discount_rule (
    id                 INTEGER PRIMARY KEY,
    marketing_group_id INTEGER NOT NULL REFERENCES marketing_group(id),
    name               TEXT    NOT NULL,
    receipt_name       TEXT,
    product_scope      TEXT    NOT NULL,
    target_id          INTEGER,
    adjustment_type    TEXT    NOT NULL,
    adjustment_value   REAL    NOT NULL,
    is_active          INTEGER NOT NULL DEFAULT 1,
    created_at         INTEGER NOT NULL DEFAULT 0,
    updated_at         INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_mg_discount_rule_group ON mg_discount_rule(marketing_group_id);

-- ── Stamp Activities (集章活动) ─────────────────────────────

CREATE TABLE stamp_activity (
    id                    INTEGER PRIMARY KEY,
    marketing_group_id    INTEGER NOT NULL REFERENCES marketing_group(id),
    name                  TEXT    NOT NULL,
    stamps_required       INTEGER NOT NULL,
    reward_quantity       INTEGER NOT NULL DEFAULT 1,
    reward_strategy       TEXT    NOT NULL DEFAULT 'ECONOMIZADOR',
    designated_product_id INTEGER,
    is_cyclic             INTEGER NOT NULL DEFAULT 1,
    is_active             INTEGER NOT NULL DEFAULT 1,
    created_at            INTEGER NOT NULL DEFAULT 0,
    updated_at            INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_stamp_activity_group ON stamp_activity(marketing_group_id);

-- ── Stamp Targets (集章条件目标) ────────────────────────────

CREATE TABLE stamp_target (
    id                INTEGER PRIMARY KEY,
    stamp_activity_id INTEGER NOT NULL REFERENCES stamp_activity(id) ON DELETE CASCADE,
    target_type       TEXT    NOT NULL,
    target_id         INTEGER NOT NULL
);
CREATE INDEX idx_stamp_target_activity ON stamp_target(stamp_activity_id);

-- ── Stamp Reward Targets (集章奖励目标) ─────────────────────

CREATE TABLE stamp_reward_target (
    id                INTEGER PRIMARY KEY,
    stamp_activity_id INTEGER NOT NULL REFERENCES stamp_activity(id) ON DELETE CASCADE,
    target_type       TEXT    NOT NULL,
    target_id         INTEGER NOT NULL
);
CREATE INDEX idx_stamp_reward_target_activity ON stamp_reward_target(stamp_activity_id);

-- ── Member Stamp Progress (会员集章进度) ────────────────────

CREATE TABLE member_stamp_progress (
    id                INTEGER PRIMARY KEY,
    member_id         INTEGER NOT NULL REFERENCES member(id),
    stamp_activity_id INTEGER NOT NULL REFERENCES stamp_activity(id),
    current_stamps    INTEGER NOT NULL DEFAULT 0,
    completed_cycles  INTEGER NOT NULL DEFAULT 0,
    last_stamp_at     INTEGER,
    updated_at        INTEGER NOT NULL DEFAULT 0,
    UNIQUE(member_id, stamp_activity_id)
);

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

CREATE TABLE print_config (
    id                      INTEGER PRIMARY KEY,
    default_kitchen_printer TEXT,
    default_label_printer   TEXT,
    updated_at              INTEGER NOT NULL DEFAULT 0
);
INSERT INTO print_config (id) VALUES (1);

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

-- Shift breakdowns: per-shift stats within daily report
CREATE TABLE daily_report_shift_breakdown (
    id              INTEGER PRIMARY KEY,
    report_id       INTEGER NOT NULL REFERENCES daily_report(id) ON DELETE CASCADE,
    shift_id        INTEGER NOT NULL REFERENCES shift(id),
    operator_id     INTEGER NOT NULL,
    operator_name   TEXT    NOT NULL,
    status          TEXT    NOT NULL,
    start_time      INTEGER NOT NULL,
    end_time        INTEGER,
    starting_cash   REAL    NOT NULL DEFAULT 0.0,
    expected_cash   REAL    NOT NULL DEFAULT 0.0,
    actual_cash     REAL,
    cash_variance   REAL,
    abnormal_close  INTEGER NOT NULL DEFAULT 0,
    total_orders      INTEGER NOT NULL DEFAULT 0,
    completed_orders  INTEGER NOT NULL DEFAULT 0,
    void_orders       INTEGER NOT NULL DEFAULT 0,
    total_sales       REAL NOT NULL DEFAULT 0.0,
    total_paid        REAL NOT NULL DEFAULT 0.0,
    void_amount       REAL NOT NULL DEFAULT 0.0,
    total_tax         REAL NOT NULL DEFAULT 0.0,
    total_discount    REAL NOT NULL DEFAULT 0.0,
    total_surcharge   REAL NOT NULL DEFAULT 0.0
);
CREATE INDEX idx_shift_breakdown_report ON daily_report_shift_breakdown(report_id);

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
    member_id                       INTEGER,
    member_name                     TEXT,
    void_type                       TEXT,
    loss_reason                     TEXT,
    loss_amount                     REAL,
    void_note                       TEXT,
    related_order_id                TEXT,
    prev_hash                       TEXT    NOT NULL,
    curr_hash                       TEXT    NOT NULL,
    order_key                       TEXT    NOT NULL DEFAULT '',
    queue_number                    INTEGER,
    shift_id                        INTEGER REFERENCES shift(id),
    cloud_synced                    INTEGER NOT NULL DEFAULT 0,
    created_at                      INTEGER NOT NULL
);
CREATE UNIQUE INDEX idx_archived_order_receipt ON archived_order(receipt_number);
CREATE UNIQUE INDEX idx_archived_order_order_key ON archived_order(order_key);
CREATE INDEX idx_archived_order_status ON archived_order(status);
CREATE INDEX idx_archived_order_end_time ON archived_order(end_time);
CREATE INDEX idx_archived_order_hash ON archived_order(curr_hash);
CREATE INDEX idx_archived_order_cloud_synced ON archived_order(cloud_synced);
CREATE INDEX idx_archived_order_shift ON archived_order(shift_id);
CREATE INDEX idx_archived_order_status_end ON archived_order(status, end_time);
CREATE INDEX idx_archived_order_created ON archived_order(created_at);
CREATE INDEX idx_archived_order_member ON archived_order(member_id);

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
    category_id            INTEGER,
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
    id            INTEGER PRIMARY KEY,
    order_pk      INTEGER NOT NULL REFERENCES archived_order(id),
    seq           INTEGER NOT NULL DEFAULT 0,
    event_type    TEXT    NOT NULL,
    timestamp     INTEGER NOT NULL,
    data          TEXT,           -- JSON string (event payload)
    prev_hash     TEXT    NOT NULL,
    curr_hash     TEXT    NOT NULL,
    operator_id   INTEGER,
    operator_name TEXT
);
CREATE INDEX idx_archived_event_order ON archived_order_event(order_pk);
CREATE INDEX idx_archived_event_time ON archived_order_event(timestamp);
CREATE INDEX idx_archived_event_operator ON archived_order_event(operator_id);

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
INSERT INTO role (id, name, description, permissions, is_system, is_active)
VALUES (1, 'admin', 'administrator', '["*"]', 1, 1);

INSERT INTO employee (id, username, hash_pass, name, role_id, is_system, is_active, created_at)
VALUES (1, 'admin', '$argon2id$v=19$m=19456,t=2,p=1$4K7SyBwr5d3uF4hroPQf2w$hPqq7x5rSE1d9TTf+hK3eipuaeeElC7GMHuSJIozDws', 'admin', 1, 1, 1, 0);

-- Store info + system state
INSERT INTO store_info (id, name, address, nif, business_day_cutoff, created_at, updated_at)
VALUES (1, '', '', '', '00:00', 0, 0);

INSERT INTO system_state (id, order_count, created_at, updated_at)
VALUES (1, 0, 0, 0);
