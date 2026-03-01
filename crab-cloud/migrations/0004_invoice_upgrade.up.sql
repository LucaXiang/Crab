-- F3 Sustitutiva support: add customer info + factura_sustituida columns to store_invoices

ALTER TABLE store_invoices ADD COLUMN customer_nif TEXT;
ALTER TABLE store_invoices ADD COLUMN customer_nombre TEXT;
ALTER TABLE store_invoices ADD COLUMN customer_address TEXT;
ALTER TABLE store_invoices ADD COLUMN customer_email TEXT;
ALTER TABLE store_invoices ADD COLUMN customer_phone TEXT;
ALTER TABLE store_invoices ADD COLUMN factura_sustituida_id BIGINT;
ALTER TABLE store_invoices ADD COLUMN factura_sustituida_num TEXT;
