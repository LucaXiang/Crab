UPDATE store_archived_orders SET service_type = 'DineIn' WHERE service_type IS NULL;
ALTER TABLE store_archived_orders ALTER COLUMN service_type SET NOT NULL;
ALTER TABLE store_archived_orders ALTER COLUMN service_type SET DEFAULT 'DineIn';
