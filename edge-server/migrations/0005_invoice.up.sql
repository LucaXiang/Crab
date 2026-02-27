-- Verifactu invoice table
CREATE TABLE invoice (
    id              INTEGER PRIMARY KEY,
    invoice_number  TEXT NOT NULL UNIQUE,
    serie           TEXT NOT NULL,
    tipo_factura    TEXT NOT NULL,
    source_type     TEXT NOT NULL,
    source_pk       INTEGER NOT NULL,
    subtotal        REAL NOT NULL,
    tax             REAL NOT NULL,
    total           REAL NOT NULL,
    huella          TEXT NOT NULL,
    prev_huella     TEXT,
    fecha_expedicion TEXT NOT NULL,
    nif             TEXT NOT NULL,
    nombre_razon    TEXT NOT NULL,
    factura_rectificada_id  INTEGER,
    factura_rectificada_num TEXT,
    cloud_synced    INTEGER NOT NULL DEFAULT 0,
    aeat_status     TEXT NOT NULL DEFAULT 'PENDING',
    created_at      INTEGER NOT NULL
);

CREATE INDEX idx_invoice_source ON invoice(source_type, source_pk);
CREATE INDEX idx_invoice_cloud_synced ON invoice(cloud_synced);
CREATE INDEX idx_invoice_serie_number ON invoice(serie, invoice_number);

-- Invoice tax breakdown (desglose)
CREATE TABLE invoice_desglose (
    id          INTEGER PRIMARY KEY,
    invoice_id  INTEGER NOT NULL REFERENCES invoice(id),
    tax_rate    INTEGER NOT NULL,
    base_amount REAL NOT NULL,
    tax_amount  REAL NOT NULL,
    UNIQUE(invoice_id, tax_rate)
);

-- Invoice counter (crash-safe numbering per Serie)
CREATE TABLE invoice_counter (
    serie       TEXT PRIMARY KEY,
    date_str    TEXT NOT NULL,
    last_number INTEGER NOT NULL
);

-- Add Verifactu huella chain to system_state
ALTER TABLE system_state ADD COLUMN last_huella TEXT;
