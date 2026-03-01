-- Invoice Anulación (RegistroFacturaBaja) — legal invoice revocation
-- Shares huella chain with invoice (Alta) via system_state.last_huella

CREATE TABLE invoice_anulacion (
    id                       INTEGER PRIMARY KEY,
    anulacion_number         TEXT    NOT NULL UNIQUE,
    serie                    TEXT    NOT NULL,
    original_invoice_id      INTEGER NOT NULL REFERENCES invoice(id),
    original_invoice_number  TEXT    NOT NULL,
    huella                   TEXT    NOT NULL,
    prev_huella              TEXT,
    fecha_expedicion         TEXT    NOT NULL,
    fecha_hora_registro      TEXT    NOT NULL,
    nif                      TEXT    NOT NULL,
    nombre_razon             TEXT    NOT NULL,
    original_order_pk        INTEGER NOT NULL REFERENCES archived_order(id),
    reason                   TEXT    NOT NULL,  -- TEST_ORDER | WRONG_CUSTOMER | DUPLICATE | OTHER
    note                     TEXT,
    operator_id              INTEGER NOT NULL,
    operator_name            TEXT    NOT NULL,
    cloud_synced             INTEGER NOT NULL DEFAULT 0,
    aeat_status              TEXT    NOT NULL DEFAULT 'PENDING',
    created_at               INTEGER NOT NULL
);

CREATE INDEX idx_anulacion_order ON invoice_anulacion(original_order_pk);
CREATE INDEX idx_anulacion_invoice ON invoice_anulacion(original_invoice_id);
CREATE INDEX idx_anulacion_cloud_synced ON invoice_anulacion(cloud_synced);

-- Anulación counter (crash-safe numbering, reuses invoice_counter pattern)
CREATE TABLE anulacion_counter (
    serie       TEXT PRIMARY KEY,
    date_str    TEXT NOT NULL,
    last_number INTEGER NOT NULL
);

-- Mark archived orders as ANULADA
ALTER TABLE archived_order ADD COLUMN is_anulada INTEGER NOT NULL DEFAULT 0;
