-- F3 Sustitutiva (Invoice Upgrade) support
-- Adds customer information fields to invoice table for F3 invoices
-- and factura_sustituida reference to the original F2 invoice.

-- Customer info (F3 only)
ALTER TABLE invoice ADD COLUMN customer_nif TEXT;
ALTER TABLE invoice ADD COLUMN customer_nombre TEXT;
ALTER TABLE invoice ADD COLUMN customer_address TEXT;
ALTER TABLE invoice ADD COLUMN customer_email TEXT;
ALTER TABLE invoice ADD COLUMN customer_phone TEXT;

-- F3 replaces original F2 (different from factura_rectificada used by R5)
ALTER TABLE invoice ADD COLUMN factura_sustituida_id INTEGER REFERENCES invoice(id);
ALTER TABLE invoice ADD COLUMN factura_sustituida_num TEXT;

-- Track whether an order has been upgraded to F3
ALTER TABLE archived_order ADD COLUMN is_upgraded INTEGER NOT NULL DEFAULT 0;
