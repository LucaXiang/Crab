-- Add MG discount tracking to archived orders
ALTER TABLE archived_order ADD COLUMN mg_discount_amount REAL NOT NULL DEFAULT 0.0;
ALTER TABLE archived_order ADD COLUMN marketing_group_name TEXT;
ALTER TABLE archived_order_item ADD COLUMN mg_discount_amount REAL NOT NULL DEFAULT 0.0;
