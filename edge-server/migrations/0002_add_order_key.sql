-- Add order_key column (UUID from OrderSnapshot.order_id) for cloud sync
ALTER TABLE archived_order ADD COLUMN order_key TEXT NOT NULL DEFAULT '';
CREATE UNIQUE INDEX IF NOT EXISTS idx_archived_order_order_key ON archived_order(order_key);

-- Track cloud sync status per order (more robust than cursor-based sync)
ALTER TABLE archived_order ADD COLUMN cloud_synced INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_archived_order_cloud_synced ON archived_order(cloud_synced);
