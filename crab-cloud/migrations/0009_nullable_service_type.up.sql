-- service_type can be NULL for orders created before service type was introduced
ALTER TABLE store_archived_orders ALTER COLUMN service_type DROP NOT NULL;
ALTER TABLE store_archived_orders ALTER COLUMN service_type DROP DEFAULT;
