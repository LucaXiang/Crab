-- Add instance_id to store_order_items for cross-referencing with credit note items.
ALTER TABLE store_order_items ADD COLUMN instance_id TEXT NOT NULL DEFAULT '';
