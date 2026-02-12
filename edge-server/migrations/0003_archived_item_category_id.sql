-- Add category_id to archived_order_item for category-based grouping in history view
ALTER TABLE archived_order_item ADD COLUMN category_id INTEGER;
