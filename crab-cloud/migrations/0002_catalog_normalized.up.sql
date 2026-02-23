-- ════════════════════════════════════════════════════════════════
-- Crab Cloud — Catalog Normalized Tables (replacing JSONB blobs)
-- ════════════════════════════════════════════════════════════════

-- Drop JSONB blob tables
DROP TABLE IF EXISTS cloud_categories CASCADE;
DROP TABLE IF EXISTS cloud_products CASCADE;

-- ── Tags ───────────────────────────────────────────────────────

CREATE TABLE catalog_tags (
    id             BIGSERIAL PRIMARY KEY,
    edge_server_id BIGINT  NOT NULL REFERENCES cloud_edge_servers(id) ON DELETE CASCADE,
    source_id      BIGINT  NOT NULL,  -- edge 本地 ID
    name           TEXT    NOT NULL,
    color          TEXT    NOT NULL DEFAULT '#3B82F6',
    display_order  INTEGER NOT NULL DEFAULT 0,
    is_active      BOOLEAN NOT NULL DEFAULT TRUE,
    is_system      BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at     BIGINT  NOT NULL,
    UNIQUE (edge_server_id, source_id)
);
CREATE INDEX idx_catalog_tags_edge ON catalog_tags (edge_server_id);

-- ── Categories ─────────────────────────────────────────────────

CREATE TABLE catalog_categories (
    id                       BIGSERIAL PRIMARY KEY,
    edge_server_id           BIGINT  NOT NULL REFERENCES cloud_edge_servers(id) ON DELETE CASCADE,
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
    UNIQUE (edge_server_id, source_id)
);
CREATE INDEX idx_catalog_categories_edge ON catalog_categories (edge_server_id);

-- Category → print_destination junction
CREATE TABLE catalog_category_print_dest (
    id             BIGSERIAL PRIMARY KEY,
    category_id    BIGINT  NOT NULL REFERENCES catalog_categories(id) ON DELETE CASCADE,
    dest_source_id BIGINT  NOT NULL  -- edge 本地 print_destination ID
);
CREATE INDEX idx_cat_print_dest_category ON catalog_category_print_dest (category_id);

-- Category → tag junction (virtual category filtering)
CREATE TABLE catalog_category_tag (
    category_id    BIGINT NOT NULL REFERENCES catalog_categories(id) ON DELETE CASCADE,
    tag_source_id  BIGINT NOT NULL,  -- edge 本地 tag ID
    PRIMARY KEY (category_id, tag_source_id)
);

-- ── Products ───────────────────────────────────────────────────

CREATE TABLE catalog_products (
    id                       BIGSERIAL PRIMARY KEY,
    edge_server_id           BIGINT  NOT NULL REFERENCES cloud_edge_servers(id) ON DELETE CASCADE,
    source_id                BIGINT  NOT NULL,
    name                     TEXT    NOT NULL,
    image                    TEXT    NOT NULL DEFAULT '',
    category_source_id       BIGINT  NOT NULL,  -- edge 本地 category ID
    sort_order               INTEGER NOT NULL DEFAULT 0,
    tax_rate                 INTEGER NOT NULL DEFAULT 0,
    receipt_name             TEXT,
    kitchen_print_name       TEXT,
    is_kitchen_print_enabled INTEGER NOT NULL DEFAULT -1,
    is_label_print_enabled   INTEGER NOT NULL DEFAULT -1,
    is_active                BOOLEAN NOT NULL DEFAULT TRUE,
    external_id              BIGINT,
    updated_at               BIGINT  NOT NULL,
    UNIQUE (edge_server_id, source_id)
);
CREATE INDEX idx_catalog_products_edge ON catalog_products (edge_server_id);

-- Product specs
CREATE TABLE catalog_product_specs (
    id            BIGSERIAL PRIMARY KEY,
    product_id    BIGINT  NOT NULL REFERENCES catalog_products(id) ON DELETE CASCADE,
    source_id     BIGINT  NOT NULL,  -- edge 本地 spec ID
    name          TEXT    NOT NULL,
    price         DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    display_order INTEGER NOT NULL DEFAULT 0,
    is_default    BOOLEAN NOT NULL DEFAULT FALSE,
    is_active     BOOLEAN NOT NULL DEFAULT TRUE,
    receipt_name  TEXT,
    is_root       BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX idx_catalog_specs_product ON catalog_product_specs (product_id);

-- Product → tag junction
CREATE TABLE catalog_product_tag (
    product_id    BIGINT NOT NULL REFERENCES catalog_products(id) ON DELETE CASCADE,
    tag_source_id BIGINT NOT NULL,  -- edge 本地 tag ID
    PRIMARY KEY (product_id, tag_source_id)
);

-- ── Attributes ─────────────────────────────────────────────────

CREATE TABLE catalog_attributes (
    id                    BIGSERIAL PRIMARY KEY,
    edge_server_id        BIGINT  NOT NULL REFERENCES cloud_edge_servers(id) ON DELETE CASCADE,
    source_id             BIGINT  NOT NULL,
    name                  TEXT    NOT NULL,
    is_multi_select       BOOLEAN NOT NULL DEFAULT FALSE,
    max_selections        INTEGER,
    default_option_ids    JSONB,   -- JSON array of int
    display_order         INTEGER NOT NULL DEFAULT 0,
    is_active             BOOLEAN NOT NULL DEFAULT TRUE,
    show_on_receipt       BOOLEAN NOT NULL DEFAULT FALSE,
    receipt_name          TEXT,
    show_on_kitchen_print BOOLEAN NOT NULL DEFAULT FALSE,
    kitchen_print_name    TEXT,
    updated_at            BIGINT  NOT NULL,
    UNIQUE (edge_server_id, source_id)
);
CREATE INDEX idx_catalog_attributes_edge ON catalog_attributes (edge_server_id);

-- Attribute options
CREATE TABLE catalog_attribute_options (
    id                 BIGSERIAL PRIMARY KEY,
    attribute_id       BIGINT  NOT NULL REFERENCES catalog_attributes(id) ON DELETE CASCADE,
    source_id          BIGINT  NOT NULL,  -- edge 本地 option ID
    name               TEXT    NOT NULL,
    price_modifier     DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    display_order      INTEGER NOT NULL DEFAULT 0,
    is_active          BOOLEAN NOT NULL DEFAULT TRUE,
    receipt_name       TEXT,
    kitchen_print_name TEXT,
    enable_quantity    BOOLEAN NOT NULL DEFAULT FALSE,
    max_quantity       INTEGER
);
CREATE INDEX idx_catalog_options_attribute ON catalog_attribute_options (attribute_id);

-- Attribute bindings (polymorphic: product | category)
CREATE TABLE catalog_attribute_bindings (
    id                 BIGSERIAL PRIMARY KEY,
    edge_server_id     BIGINT  NOT NULL REFERENCES cloud_edge_servers(id) ON DELETE CASCADE,
    source_id          BIGINT  NOT NULL,  -- edge 本地 binding ID
    owner_type         TEXT    NOT NULL,   -- 'product' | 'category'
    owner_source_id    BIGINT  NOT NULL,   -- edge 本地 owner ID
    attribute_source_id BIGINT NOT NULL,   -- edge 本地 attribute ID
    is_required        BOOLEAN NOT NULL DEFAULT FALSE,
    display_order      INTEGER NOT NULL DEFAULT 0,
    default_option_ids JSONB,
    UNIQUE (edge_server_id, source_id)
);
CREATE INDEX idx_catalog_bindings_edge ON catalog_attribute_bindings (edge_server_id);
CREATE INDEX idx_catalog_bindings_owner ON catalog_attribute_bindings (owner_type, owner_source_id);

-- ── Price Rules ───────────────────────────────────────────────

CREATE TABLE catalog_price_rules (
    id               BIGSERIAL PRIMARY KEY,
    edge_server_id   BIGINT NOT NULL REFERENCES cloud_edge_servers(id) ON DELETE CASCADE,
    source_id        BIGINT NOT NULL,
    name             TEXT NOT NULL,
    display_name     TEXT NOT NULL,
    receipt_name     TEXT NOT NULL,
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
    active_days      JSONB,
    active_start_time TEXT,
    active_end_time  TEXT,
    is_active        BOOLEAN NOT NULL DEFAULT TRUE,
    created_by       BIGINT,
    created_at       BIGINT NOT NULL,
    updated_at       BIGINT NOT NULL,
    UNIQUE (edge_server_id, source_id)
);

-- ── Catalog Version Tracking ───────────────────────────────────

CREATE TABLE catalog_versions (
    id             BIGSERIAL PRIMARY KEY,
    edge_server_id BIGINT NOT NULL REFERENCES cloud_edge_servers(id) ON DELETE CASCADE UNIQUE,
    version        BIGINT NOT NULL DEFAULT 0,
    updated_at     BIGINT NOT NULL
);
