-- 0006: Unified table rename + missing indexes + FK cascade fixes
--
-- Naming convention:
--   store_*  = store-scoped data (has edge_server_id)
--   no prefix = global infrastructure (tenants, subscriptions, etc.)

-- ════════════════════════════════════════════════════════════════
-- 1. Rename global infrastructure tables (drop redundant cloud_ prefix)
-- ════════════════════════════════════════════════════════════════

ALTER TABLE cloud_audit_log RENAME TO audit_logs;
ALTER TABLE cloud_edge_servers RENAME TO edge_servers;

-- ════════════════════════════════════════════════════════════════
-- 2. Rename store-scoped cloud_* tables → store_*
-- ════════════════════════════════════════════════════════════════

ALTER TABLE cloud_sync_cursors RENAME TO store_sync_cursors;
ALTER TABLE cloud_zones RENAME TO store_zones;
ALTER TABLE cloud_dining_tables RENAME TO store_dining_tables;
ALTER TABLE cloud_shifts RENAME TO store_shifts;
ALTER TABLE cloud_employees RENAME TO store_employees;
ALTER TABLE cloud_daily_reports RENAME TO store_daily_reports;
ALTER TABLE cloud_daily_report_tax_breakdown RENAME TO store_daily_report_tax_breakdown;
ALTER TABLE cloud_daily_report_payment_breakdown RENAME TO store_daily_report_payment_breakdown;
ALTER TABLE cloud_store_info RENAME TO store_info;
ALTER TABLE cloud_commands RENAME TO store_commands;
ALTER TABLE cloud_archived_orders RENAME TO store_archived_orders;
ALTER TABLE cloud_order_items RENAME TO store_order_items;
ALTER TABLE cloud_order_payments RENAME TO store_order_payments;
ALTER TABLE cloud_order_details RENAME TO store_order_details;
ALTER TABLE cloud_order_events RENAME TO store_order_events;
ALTER TABLE cloud_label_templates RENAME TO store_label_templates;
ALTER TABLE cloud_label_fields RENAME TO store_label_fields;

-- ════════════════════════════════════════════════════════════════
-- 3. Rename catalog_* tables → store_*
-- ════════════════════════════════════════════════════════════════

ALTER TABLE catalog_tags RENAME TO store_tags;
ALTER TABLE catalog_categories RENAME TO store_categories;
ALTER TABLE catalog_category_print_dest RENAME TO store_category_print_dest;
ALTER TABLE catalog_category_tag RENAME TO store_category_tag;
ALTER TABLE catalog_products RENAME TO store_products;
ALTER TABLE catalog_product_specs RENAME TO store_product_specs;
ALTER TABLE catalog_product_tag RENAME TO store_product_tag;
ALTER TABLE catalog_attributes RENAME TO store_attributes;
ALTER TABLE catalog_attribute_options RENAME TO store_attribute_options;
ALTER TABLE catalog_attribute_bindings RENAME TO store_attribute_bindings;
ALTER TABLE catalog_price_rules RENAME TO store_price_rules;
ALTER TABLE catalog_versions RENAME TO store_versions;

-- ════════════════════════════════════════════════════════════════
-- 4. Add missing indexes
-- ════════════════════════════════════════════════════════════════

-- FK index: subscriptions → tenants
CREATE INDEX IF NOT EXISTS idx_subscriptions_tenant_id ON subscriptions(tenant_id);

-- FK index: self-references (partial index, only non-null)
CREATE INDEX IF NOT EXISTS idx_activations_replaced_by ON activations(replaced_by) WHERE replaced_by IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_client_connections_replaced_by ON client_connections(replaced_by) WHERE replaced_by IS NOT NULL;

-- FK indexes: store-scoped tables by edge_server_id
CREATE INDEX IF NOT EXISTS idx_store_shifts_edge ON store_shifts(edge_server_id);
CREATE INDEX IF NOT EXISTS idx_store_employees_edge ON store_employees(edge_server_id);
CREATE INDEX IF NOT EXISTS idx_store_info_edge ON store_info(edge_server_id);
CREATE INDEX IF NOT EXISTS idx_store_commands_edge ON store_commands(edge_server_id);
CREATE INDEX IF NOT EXISTS idx_store_daily_reports_edge ON store_daily_reports(edge_server_id);

-- ════════════════════════════════════════════════════════════════
-- 5. Fix missing ON DELETE CASCADE on FK constraints
-- ════════════════════════════════════════════════════════════════

-- subscriptions → tenants
ALTER TABLE subscriptions DROP CONSTRAINT IF EXISTS subscriptions_tenant_id_fkey;
ALTER TABLE subscriptions ADD CONSTRAINT subscriptions_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE;

-- refresh_tokens → tenants
ALTER TABLE refresh_tokens DROP CONSTRAINT IF EXISTS refresh_tokens_tenant_id_fkey;
ALTER TABLE refresh_tokens ADD CONSTRAINT refresh_tokens_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE;

-- store_sync_cursors → edge_servers
ALTER TABLE store_sync_cursors DROP CONSTRAINT IF EXISTS cloud_sync_cursors_edge_server_id_fkey;
ALTER TABLE store_sync_cursors ADD CONSTRAINT store_sync_cursors_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

-- store_archived_orders → edge_servers
ALTER TABLE store_archived_orders DROP CONSTRAINT IF EXISTS cloud_archived_orders_edge_server_id_fkey;
ALTER TABLE store_archived_orders ADD CONSTRAINT store_archived_orders_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

-- store_commands → edge_servers
ALTER TABLE store_commands DROP CONSTRAINT IF EXISTS cloud_commands_edge_server_id_fkey;
ALTER TABLE store_commands ADD CONSTRAINT store_commands_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

-- store_shifts → edge_servers
ALTER TABLE store_shifts DROP CONSTRAINT IF EXISTS cloud_shifts_edge_server_id_fkey;
ALTER TABLE store_shifts ADD CONSTRAINT store_shifts_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

-- store_employees → edge_servers
ALTER TABLE store_employees DROP CONSTRAINT IF EXISTS cloud_employees_edge_server_id_fkey;
ALTER TABLE store_employees ADD CONSTRAINT store_employees_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

-- store_daily_reports → edge_servers
ALTER TABLE store_daily_reports DROP CONSTRAINT IF EXISTS cloud_daily_reports_edge_server_id_fkey;
ALTER TABLE store_daily_reports ADD CONSTRAINT store_daily_reports_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

-- store_info → edge_servers
ALTER TABLE store_info DROP CONSTRAINT IF EXISTS cloud_store_info_edge_server_id_fkey;
ALTER TABLE store_info ADD CONSTRAINT store_info_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

-- store_zones → edge_servers
ALTER TABLE store_zones DROP CONSTRAINT IF EXISTS cloud_zones_edge_server_id_fkey;
ALTER TABLE store_zones ADD CONSTRAINT store_zones_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

-- store_dining_tables → edge_servers
ALTER TABLE store_dining_tables DROP CONSTRAINT IF EXISTS cloud_dining_tables_edge_server_id_fkey;
ALTER TABLE store_dining_tables ADD CONSTRAINT store_dining_tables_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

-- store_label_templates → edge_servers (already has CASCADE, update constraint name)
ALTER TABLE store_label_templates DROP CONSTRAINT IF EXISTS cloud_label_templates_edge_server_id_fkey;
ALTER TABLE store_label_templates ADD CONSTRAINT store_label_templates_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

-- store_label_fields → store_label_templates (already has CASCADE, update constraint name)
ALTER TABLE store_label_fields DROP CONSTRAINT IF EXISTS cloud_label_fields_template_id_fkey;
ALTER TABLE store_label_fields ADD CONSTRAINT store_label_fields_template_id_fkey
    FOREIGN KEY (template_id) REFERENCES store_label_templates(id) ON DELETE CASCADE;

-- Fix catalog FK constraints to reference new edge_servers table name
-- (PostgreSQL auto-updates FK references when target table is renamed, but constraint names stay old)
-- Re-create with clean names for the catalog/store tables
ALTER TABLE store_tags DROP CONSTRAINT IF EXISTS catalog_tags_edge_server_id_fkey;
ALTER TABLE store_tags ADD CONSTRAINT store_tags_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

ALTER TABLE store_categories DROP CONSTRAINT IF EXISTS catalog_categories_edge_server_id_fkey;
ALTER TABLE store_categories ADD CONSTRAINT store_categories_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

ALTER TABLE store_products DROP CONSTRAINT IF EXISTS catalog_products_edge_server_id_fkey;
ALTER TABLE store_products ADD CONSTRAINT store_products_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

ALTER TABLE store_attributes DROP CONSTRAINT IF EXISTS catalog_attributes_edge_server_id_fkey;
ALTER TABLE store_attributes ADD CONSTRAINT store_attributes_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

ALTER TABLE store_attribute_bindings DROP CONSTRAINT IF EXISTS catalog_attribute_bindings_edge_server_id_fkey;
ALTER TABLE store_attribute_bindings ADD CONSTRAINT store_attribute_bindings_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

ALTER TABLE store_price_rules DROP CONSTRAINT IF EXISTS catalog_price_rules_edge_server_id_fkey;
ALTER TABLE store_price_rules ADD CONSTRAINT store_price_rules_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

ALTER TABLE store_versions DROP CONSTRAINT IF EXISTS catalog_versions_edge_server_id_fkey;
ALTER TABLE store_versions ADD CONSTRAINT store_versions_edge_server_id_fkey
    FOREIGN KEY (edge_server_id) REFERENCES edge_servers(id) ON DELETE CASCADE;

-- Child table FK renames (parent table changed name)
ALTER TABLE store_category_print_dest DROP CONSTRAINT IF EXISTS catalog_category_print_dest_category_id_fkey;
ALTER TABLE store_category_print_dest ADD CONSTRAINT store_category_print_dest_category_id_fkey
    FOREIGN KEY (category_id) REFERENCES store_categories(id) ON DELETE CASCADE;

ALTER TABLE store_category_tag DROP CONSTRAINT IF EXISTS catalog_category_tag_category_id_fkey;
ALTER TABLE store_category_tag ADD CONSTRAINT store_category_tag_category_id_fkey
    FOREIGN KEY (category_id) REFERENCES store_categories(id) ON DELETE CASCADE;

ALTER TABLE store_product_specs DROP CONSTRAINT IF EXISTS catalog_product_specs_product_id_fkey;
ALTER TABLE store_product_specs ADD CONSTRAINT store_product_specs_product_id_fkey
    FOREIGN KEY (product_id) REFERENCES store_products(id) ON DELETE CASCADE;

ALTER TABLE store_product_tag DROP CONSTRAINT IF EXISTS catalog_product_tag_product_id_fkey;
ALTER TABLE store_product_tag ADD CONSTRAINT store_product_tag_product_id_fkey
    FOREIGN KEY (product_id) REFERENCES store_products(id) ON DELETE CASCADE;

ALTER TABLE store_attribute_options DROP CONSTRAINT IF EXISTS catalog_attribute_options_attribute_id_fkey;
ALTER TABLE store_attribute_options ADD CONSTRAINT store_attribute_options_attribute_id_fkey
    FOREIGN KEY (attribute_id) REFERENCES store_attributes(id) ON DELETE CASCADE;
