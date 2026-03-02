-- Add order-layer flags to store_archived_orders
-- These fields allow ANULACION/UPGRADE to be tracked directly on the order
-- instead of requiring separate invoice-layer tables.

ALTER TABLE store_archived_orders ADD COLUMN is_anulada BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE store_archived_orders ADD COLUMN is_upgraded BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE store_archived_orders ADD COLUMN customer_nif TEXT;
ALTER TABLE store_archived_orders ADD COLUMN customer_nombre TEXT;
ALTER TABLE store_archived_orders ADD COLUMN customer_address TEXT;
ALTER TABLE store_archived_orders ADD COLUMN customer_email TEXT;
ALTER TABLE store_archived_orders ADD COLUMN customer_phone TEXT;
