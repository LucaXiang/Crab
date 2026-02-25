-- Migration 0002: Simplify order storage
--
-- Merge store_order_details into store_archived_orders (single table).
-- Drop 4 redundant sub-tables. All detail data stored as JSONB in store_archived_orders.detail.

-- 1. Add detail JSONB column to store_archived_orders
ALTER TABLE store_archived_orders ADD COLUMN IF NOT EXISTS detail JSONB;

-- 2. Migrate existing detail data
UPDATE store_archived_orders o
SET detail = d.detail
FROM store_order_details d
WHERE d.archived_order_id = o.id;

-- 3. Drop all sub-tables (CASCADE handles FK + indexes)
DROP TABLE IF EXISTS store_order_details;
DROP TABLE IF EXISTS store_order_events;
DROP TABLE IF EXISTS store_order_payments;
DROP TABLE IF EXISTS store_order_items;
