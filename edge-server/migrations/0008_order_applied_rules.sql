-- Add order-level applied rules JSON to archived orders for cloud sync
ALTER TABLE archived_order ADD COLUMN order_applied_rules TEXT;
