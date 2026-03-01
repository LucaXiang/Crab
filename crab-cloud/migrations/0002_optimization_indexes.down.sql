-- Revert optimization indexes

DROP INDEX IF EXISTS idx_archived_orders_overview;
DROP INDEX IF EXISTS idx_archived_orders_page;
DROP INDEX IF EXISTS idx_credit_notes_time;
DROP INDEX IF EXISTS idx_audit_logs_tenant_action;

-- Restore original refresh_tokens indexes
DROP INDEX IF EXISTS idx_refresh_tokens_active;
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_tenant ON refresh_tokens(tenant_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires ON refresh_tokens(expires_at) WHERE NOT revoked;

-- Restore original bindings indexes
DROP INDEX IF EXISTS idx_store_bindings_owner;
CREATE INDEX IF NOT EXISTS idx_store_bindings_store ON store_attribute_bindings (store_id);
CREATE INDEX IF NOT EXISTS idx_store_bindings_owner ON store_attribute_bindings (owner_type, owner_source_id);

-- Restore original AEAT index
DROP INDEX IF EXISTS idx_store_invoices_aeat_pending;
CREATE INDEX IF NOT EXISTS idx_store_invoices_aeat_pending ON store_invoices (aeat_status) WHERE aeat_status != 'ACCEPTED';

-- Restore original commands index
DROP INDEX IF EXISTS idx_store_commands_history;
CREATE INDEX IF NOT EXISTS idx_store_commands_store ON store_commands(store_id);

