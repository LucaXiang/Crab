-- Add product_source_id to cloud_order_items for tag-based statistics
ALTER TABLE cloud_order_items ADD COLUMN product_source_id BIGINT;

CREATE INDEX idx_cloud_order_items_product ON cloud_order_items (product_source_id);
