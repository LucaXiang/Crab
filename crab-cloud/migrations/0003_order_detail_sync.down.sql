DROP TABLE IF EXISTS cloud_order_details;
DROP TABLE IF EXISTS cloud_order_desglose;

-- Remove added columns
ALTER TABLE cloud_archived_orders DROP COLUMN IF EXISTS order_key;
ALTER TABLE cloud_archived_orders DROP COLUMN IF EXISTS tax;
ALTER TABLE cloud_archived_orders DROP COLUMN IF EXISTS prev_hash;
ALTER TABLE cloud_archived_orders DROP COLUMN IF EXISTS curr_hash;

-- Restore original unique constraint
DROP INDEX IF EXISTS uq_cloud_archived_orders_key;
CREATE UNIQUE INDEX IF NOT EXISTS cloud_archived_orders_edge_server_id_source_id_key
    ON cloud_archived_orders (edge_server_id, source_id);
