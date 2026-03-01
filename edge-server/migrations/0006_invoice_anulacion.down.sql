DROP TABLE IF EXISTS anulacion_counter;
DROP TABLE IF EXISTS invoice_anulacion;
-- SQLite doesn't support DROP COLUMN in older versions, but we're on new enough
ALTER TABLE archived_order DROP COLUMN is_anulada;
