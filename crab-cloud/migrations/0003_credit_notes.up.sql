-- ── Credit Notes (synced from edge, summary + detail JSONB) ──

CREATE TABLE IF NOT EXISTS store_credit_notes (
    id BIGSERIAL PRIMARY KEY,
    store_id BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    tenant_id TEXT NOT NULL,
    source_id BIGINT NOT NULL,
    credit_note_number TEXT NOT NULL,
    original_order_key TEXT NOT NULL,
    original_receipt TEXT NOT NULL,
    subtotal_credit DOUBLE PRECISION NOT NULL,
    tax_credit DOUBLE PRECISION NOT NULL,
    total_credit DOUBLE PRECISION NOT NULL,
    refund_method TEXT NOT NULL,
    reason TEXT NOT NULL,
    note TEXT,
    operator_name TEXT NOT NULL,
    authorizer_name TEXT,
    prev_hash TEXT NOT NULL,
    curr_hash TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    detail JSONB,
    version BIGINT NOT NULL DEFAULT 0,
    synced_at BIGINT NOT NULL
);

CREATE UNIQUE INDEX uq_store_credit_notes_source
    ON store_credit_notes (tenant_id, store_id, source_id);
CREATE INDEX idx_store_credit_notes_order
    ON store_credit_notes (tenant_id, store_id, original_order_key);
CREATE INDEX idx_store_credit_notes_tenant
    ON store_credit_notes (tenant_id, created_at DESC);
