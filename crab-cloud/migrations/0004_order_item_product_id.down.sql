DROP INDEX IF EXISTS idx_cloud_order_items_product;
ALTER TABLE cloud_order_items DROP COLUMN IF EXISTS product_source_id;
