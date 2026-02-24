-- Custom enum types for label fields
CREATE TYPE label_field_type AS ENUM (
    'text', 'barcode', 'qrcode', 'image', 'separator', 'datetime', 'price', 'counter'
);

CREATE TYPE label_field_alignment AS ENUM (
    'left', 'center', 'right'
);

-- Label templates (normalized, bidirectional sync via source_id)
CREATE TABLE store_label_templates (
    id              BIGSERIAL PRIMARY KEY,
    edge_server_id  BIGINT NOT NULL REFERENCES cloud_edge_servers(id) ON DELETE CASCADE,
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
    UNIQUE(edge_server_id, source_id)
);

CREATE INDEX idx_store_label_templates_edge ON store_label_templates(edge_server_id);

-- Label fields (independent relation table)
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
