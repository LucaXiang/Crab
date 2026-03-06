-- Add customer fields to archived_order for UPGRADE support
ALTER TABLE archived_order ADD COLUMN customer_nif     TEXT;
ALTER TABLE archived_order ADD COLUMN customer_nombre  TEXT;
ALTER TABLE archived_order ADD COLUMN customer_address TEXT;
ALTER TABLE archived_order ADD COLUMN customer_email   TEXT;
ALTER TABLE archived_order ADD COLUMN customer_phone   TEXT;
