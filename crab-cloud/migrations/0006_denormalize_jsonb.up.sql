-- Denormalize JSONB detail columns into proper relational tables.
-- This migration aligns Cloud PostgreSQL with Edge SQLite's normalized schema.
-- Run with clean database (DROP pgdata + Edge re-sync).

-- ═══════════════════════════════════════════════════════════════
-- 1. store_archived_orders: promote scalar fields from detail JSONB
-- ═══════════════════════════════════════════════════════════════

ALTER TABLE store_archived_orders ADD COLUMN zone_name TEXT;
ALTER TABLE store_archived_orders ADD COLUMN table_name TEXT;
ALTER TABLE store_archived_orders ADD COLUMN is_retail BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE store_archived_orders ADD COLUMN original_total DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE store_archived_orders ADD COLUMN subtotal DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE store_archived_orders ADD COLUMN paid_amount DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE store_archived_orders ADD COLUMN surcharge_amount DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE store_archived_orders ADD COLUMN comp_total_amount DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE store_archived_orders ADD COLUMN order_manual_discount_amount DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE store_archived_orders ADD COLUMN order_manual_surcharge_amount DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE store_archived_orders ADD COLUMN order_rule_discount_amount DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE store_archived_orders ADD COLUMN order_rule_surcharge_amount DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE store_archived_orders ADD COLUMN operator_name TEXT;
ALTER TABLE store_archived_orders ADD COLUMN loss_reason TEXT;
ALTER TABLE store_archived_orders ADD COLUMN void_note TEXT;
ALTER TABLE store_archived_orders ADD COLUMN member_name TEXT;
ALTER TABLE store_archived_orders ADD COLUMN service_type TEXT NOT NULL DEFAULT 'DineIn';

-- Drop JSONB columns
ALTER TABLE store_archived_orders DROP COLUMN detail;
ALTER TABLE store_archived_orders DROP COLUMN desglose;

-- ═══════════════════════════════════════════════════════════════
-- 2. Order child tables
-- ═══════════════════════════════════════════════════════════════

CREATE TABLE store_order_items (
    id                BIGSERIAL PRIMARY KEY,
    order_id          BIGINT NOT NULL REFERENCES store_archived_orders(id) ON DELETE CASCADE,
    name              TEXT NOT NULL,
    spec_name         TEXT,
    category_name     TEXT,
    product_source_id BIGINT,
    price             DOUBLE PRECISION NOT NULL DEFAULT 0,
    quantity          INTEGER NOT NULL DEFAULT 1,
    unit_price        DOUBLE PRECISION NOT NULL DEFAULT 0,
    line_total        DOUBLE PRECISION NOT NULL DEFAULT 0,
    discount_amount   DOUBLE PRECISION NOT NULL DEFAULT 0,
    surcharge_amount  DOUBLE PRECISION NOT NULL DEFAULT 0,
    tax               DOUBLE PRECISION NOT NULL DEFAULT 0,
    tax_rate          INTEGER NOT NULL DEFAULT 0,
    is_comped         BOOLEAN NOT NULL DEFAULT false,
    note              TEXT
);
CREATE INDEX idx_soi_order ON store_order_items(order_id);
CREATE INDEX idx_soi_product ON store_order_items(product_source_id) WHERE product_source_id IS NOT NULL;

CREATE TABLE store_order_item_options (
    id              BIGSERIAL PRIMARY KEY,
    item_id         BIGINT NOT NULL REFERENCES store_order_items(id) ON DELETE CASCADE,
    attribute_name  TEXT NOT NULL,
    option_name     TEXT NOT NULL,
    price           DOUBLE PRECISION NOT NULL DEFAULT 0,
    quantity        INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_soio_item ON store_order_item_options(item_id);

CREATE TABLE store_order_payments (
    id              BIGSERIAL PRIMARY KEY,
    order_id        BIGINT NOT NULL REFERENCES store_archived_orders(id) ON DELETE CASCADE,
    seq             INTEGER NOT NULL DEFAULT 0,
    method          TEXT NOT NULL,
    amount          DOUBLE PRECISION NOT NULL DEFAULT 0,
    timestamp       BIGINT NOT NULL,
    cancelled       BOOLEAN NOT NULL DEFAULT false
);
CREATE INDEX idx_sop_order ON store_order_payments(order_id);
CREATE INDEX idx_sop_method ON store_order_payments(method);

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

CREATE TABLE store_order_desglose (
    id              BIGSERIAL PRIMARY KEY,
    order_id        BIGINT NOT NULL REFERENCES store_archived_orders(id) ON DELETE CASCADE,
    tax_rate        INTEGER NOT NULL,
    base_amount     DOUBLE PRECISION NOT NULL,
    tax_amount      DOUBLE PRECISION NOT NULL,
    UNIQUE(order_id, tax_rate)
);

-- ═══════════════════════════════════════════════════════════════
-- 3. Credit note items table
-- ═══════════════════════════════════════════════════════════════

CREATE TABLE store_credit_note_items (
    id              BIGSERIAL PRIMARY KEY,
    credit_note_id  BIGINT NOT NULL REFERENCES store_credit_notes(id) ON DELETE CASCADE,
    item_name       TEXT NOT NULL,
    quantity        INTEGER NOT NULL,
    unit_price      DOUBLE PRECISION NOT NULL,
    line_credit     DOUBLE PRECISION NOT NULL,
    tax_rate        INTEGER NOT NULL,
    tax_credit      DOUBLE PRECISION NOT NULL
);
CREATE INDEX idx_scni_cn ON store_credit_note_items(credit_note_id);

-- Drop credit_notes detail JSONB
ALTER TABLE store_credit_notes DROP COLUMN detail;

-- ═══════════════════════════════════════════════════════════════
-- 4. Invoice desglose table
-- ═══════════════════════════════════════════════════════════════

CREATE TABLE store_invoice_desglose (
    id              BIGSERIAL PRIMARY KEY,
    invoice_id      BIGINT NOT NULL REFERENCES store_invoices(id) ON DELETE CASCADE,
    tax_rate        INTEGER NOT NULL,
    base_amount     DOUBLE PRECISION NOT NULL,
    tax_amount      DOUBLE PRECISION NOT NULL,
    UNIQUE(invoice_id, tax_rate)
);

-- Drop invoices and anulaciones detail JSONB
ALTER TABLE store_invoices DROP COLUMN detail;
ALTER TABLE store_anulaciones DROP COLUMN detail;
