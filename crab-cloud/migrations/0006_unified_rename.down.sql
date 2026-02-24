-- Reverse all renames from 0006

-- Restore catalog_* names
ALTER TABLE store_tags RENAME TO catalog_tags;
ALTER TABLE store_categories RENAME TO catalog_categories;
ALTER TABLE store_category_print_dest RENAME TO catalog_category_print_dest;
ALTER TABLE store_category_tag RENAME TO catalog_category_tag;
ALTER TABLE store_products RENAME TO catalog_products;
ALTER TABLE store_product_specs RENAME TO catalog_product_specs;
ALTER TABLE store_product_tag RENAME TO catalog_product_tag;
ALTER TABLE store_attributes RENAME TO catalog_attributes;
ALTER TABLE store_attribute_options RENAME TO catalog_attribute_options;
ALTER TABLE store_attribute_bindings RENAME TO catalog_attribute_bindings;
ALTER TABLE store_price_rules RENAME TO catalog_price_rules;
ALTER TABLE store_versions RENAME TO catalog_versions;

-- Restore cloud_* names
ALTER TABLE store_sync_cursors RENAME TO cloud_sync_cursors;
ALTER TABLE store_zones RENAME TO cloud_zones;
ALTER TABLE store_dining_tables RENAME TO cloud_dining_tables;
ALTER TABLE store_shifts RENAME TO cloud_shifts;
ALTER TABLE store_employees RENAME TO cloud_employees;
ALTER TABLE store_daily_reports RENAME TO cloud_daily_reports;
ALTER TABLE store_info RENAME TO cloud_store_info;
ALTER TABLE store_commands RENAME TO cloud_commands;
ALTER TABLE store_archived_orders RENAME TO cloud_archived_orders;
ALTER TABLE store_order_items RENAME TO cloud_order_items;
ALTER TABLE store_order_payments RENAME TO cloud_order_payments;
ALTER TABLE store_order_details RENAME TO cloud_order_details;
ALTER TABLE store_order_events RENAME TO cloud_order_events;
ALTER TABLE store_label_templates RENAME TO cloud_label_templates;
ALTER TABLE store_label_fields RENAME TO cloud_label_fields;

-- Restore global names
ALTER TABLE audit_logs RENAME TO cloud_audit_log;
ALTER TABLE edge_servers RENAME TO cloud_edge_servers;

-- Drop added indexes
DROP INDEX IF EXISTS idx_subscriptions_tenant_id;
DROP INDEX IF EXISTS idx_activations_replaced_by;
DROP INDEX IF EXISTS idx_client_connections_replaced_by;
DROP INDEX IF EXISTS idx_store_shifts_edge;
DROP INDEX IF EXISTS idx_store_employees_edge;
DROP INDEX IF EXISTS idx_store_info_edge;
DROP INDEX IF EXISTS idx_store_commands_edge;
DROP INDEX IF EXISTS idx_store_daily_reports_edge;
