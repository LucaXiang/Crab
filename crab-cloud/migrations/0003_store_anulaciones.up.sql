-- Store invoice anulaciones (RegistroFacturaBaja) synced from edge
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
    prev_hash       TEXT NOT NULL,
    curr_hash       TEXT NOT NULL,
    detail          JSONB NOT NULL DEFAULT '{}',
    aeat_status     TEXT NOT NULL DEFAULT 'PENDING',
    version         BIGINT NOT NULL DEFAULT 0,
    created_at      BIGINT NOT NULL,
    synced_at       BIGINT NOT NULL,
    UNIQUE(tenant_id, store_id, source_id)
);

CREATE INDEX idx_store_anulaciones_store ON store_anulaciones(store_id);
CREATE INDEX idx_store_anulaciones_order ON store_anulaciones(store_id, original_order_id);
CREATE INDEX idx_store_anulaciones_tenant ON store_anulaciones(tenant_id, created_at DESC);
