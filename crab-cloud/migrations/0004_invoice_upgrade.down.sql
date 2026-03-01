-- Revert F3 Sustitutiva columns from store_invoices

ALTER TABLE store_invoices DROP COLUMN IF EXISTS factura_sustituida_num;
ALTER TABLE store_invoices DROP COLUMN IF EXISTS factura_sustituida_id;
ALTER TABLE store_invoices DROP COLUMN IF EXISTS customer_phone;
ALTER TABLE store_invoices DROP COLUMN IF EXISTS customer_email;
ALTER TABLE store_invoices DROP COLUMN IF EXISTS customer_address;
ALTER TABLE store_invoices DROP COLUMN IF EXISTS customer_nombre;
ALTER TABLE store_invoices DROP COLUMN IF EXISTS customer_nif;
