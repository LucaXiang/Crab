DROP TABLE IF EXISTS cloud_order_details;
DROP TABLE IF EXISTS cloud_order_desglose;

-- Restore data column
ALTER TABLE cloud_archived_orders ADD COLUMN data JSONB NOT NULL DEFAULT '{}';

-- Remove added columns
ALTER TABLE cloud_archived_orders DROP COLUMN IF EXISTS order_key;
ALTER TABLE cloud_archived_orders DROP COLUMN IF EXISTS tax;
ALTER TABLE cloud_archived_orders DROP COLUMN IF EXISTS prev_hash;
ALTER TABLE cloud_archived_orders DROP COLUMN IF EXISTS curr_hash;

-- Restore original unique constraint
DROP INDEX IF EXISTS uq_cloud_archived_orders_key;
ALTER TABLE cloud_archived_orders ADD CONSTRAINT cloud_archived_orders_edge_server_id_source_id_key
    UNIQUE (edge_server_id, source_id);
