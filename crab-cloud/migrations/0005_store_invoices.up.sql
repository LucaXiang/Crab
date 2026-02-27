-- ── Verifactu Invoices (synced from edge, F2/R5 with AEAT status) ──

CREATE TABLE IF NOT EXISTS store_invoices (
    id BIGSERIAL PRIMARY KEY,
    store_id BIGINT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    tenant_id TEXT NOT NULL,
    source_id BIGINT NOT NULL,
    invoice_number TEXT NOT NULL,
    serie TEXT NOT NULL,
    tipo_factura TEXT NOT NULL,
    source_type TEXT NOT NULL,
    source_pk BIGINT NOT NULL,
    subtotal DOUBLE PRECISION NOT NULL,
    tax DOUBLE PRECISION NOT NULL,
    total DOUBLE PRECISION NOT NULL,
    huella TEXT NOT NULL,
    prev_huella TEXT,
    fecha_expedicion TEXT NOT NULL,
    nif TEXT NOT NULL,
    nombre_razon TEXT NOT NULL,
    factura_rectificada_id BIGINT,
    factura_rectificada_num TEXT,
    aeat_status TEXT NOT NULL DEFAULT 'PENDING',
    aeat_csv TEXT,
    aeat_submitted_at BIGINT,
    aeat_response_at BIGINT,
    created_at BIGINT NOT NULL,
    detail JSONB,
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
    ON store_invoices (aeat_status) WHERE aeat_status != 'ACCEPTED';
CREATE INDEX idx_store_invoices_order
    ON store_invoices (store_id, source_type, source_pk);
