-- Revert F3 Sustitutiva columns
-- SQLite doesn't support DROP COLUMN before 3.35.0, but we target newer versions.

ALTER TABLE archived_order DROP COLUMN is_upgraded;

ALTER TABLE invoice DROP COLUMN factura_sustituida_num;
ALTER TABLE invoice DROP COLUMN factura_sustituida_id;
ALTER TABLE invoice DROP COLUMN customer_phone;
ALTER TABLE invoice DROP COLUMN customer_email;
ALTER TABLE invoice DROP COLUMN customer_address;
ALTER TABLE invoice DROP COLUMN customer_nombre;
ALTER TABLE invoice DROP COLUMN customer_nif;
