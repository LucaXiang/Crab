-- Performance optimization indexes (2026-02-28)
-- Addresses: overview JSONB scans, missing composite indexes, cleanup support

-- ── store_archived_orders: overview covering index ──
-- Enables Index-Only Scan for tenant-wide aggregations (SUM/COUNT/AVG)
CREATE INDEX IF NOT EXISTS idx_archived_orders_overview
    ON store_archived_orders (tenant_id, end_time)
    INCLUDE (store_id, status, total, tax, guest_count,
             discount_amount, start_time, void_type, loss_amount)
    WHERE end_time IS NOT NULL;

-- Page listing without status filter
CREATE INDEX IF NOT EXISTS idx_archived_orders_page
    ON store_archived_orders (store_id, tenant_id, end_time DESC NULLS LAST);

-- ── store_credit_notes: time range queries ──
CREATE INDEX IF NOT EXISTS idx_credit_notes_time
    ON store_credit_notes (tenant_id, store_id, created_at)
    INCLUDE (total_credit, refund_method);

-- ── audit_logs: action filter + cleanup ──
CREATE INDEX IF NOT EXISTS idx_audit_logs_tenant_action
    ON audit_logs (tenant_id, action, created_at DESC);

-- ── refresh_tokens: replace separate indexes with composite conditional ──
DROP INDEX IF EXISTS idx_refresh_tokens_tenant;
DROP INDEX IF EXISTS idx_refresh_tokens_expires;
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_active
    ON refresh_tokens (tenant_id, expires_at DESC)
    WHERE NOT revoked;

-- ── store_attribute_bindings: add store_id for multi-tenant safety ──
-- New composite (store_id, owner_type, owner_source_id) subsumes both old indexes
DROP INDEX IF EXISTS idx_store_bindings_owner;
DROP INDEX IF EXISTS idx_store_bindings_store;
CREATE INDEX IF NOT EXISTS idx_store_bindings_owner
    ON store_attribute_bindings (store_id, owner_type, owner_source_id);

-- ── store_invoices: AEAT pending with store_id ──
DROP INDEX IF EXISTS idx_store_invoices_aeat_pending;
CREATE INDEX IF NOT EXISTS idx_store_invoices_aeat_pending
    ON store_invoices (store_id, aeat_status, created_at)
    WHERE aeat_status != 'ACCEPTED';

-- ── store_commands: history query with tenant_id ──
DROP INDEX IF EXISTS idx_store_commands_store;
CREATE INDEX IF NOT EXISTS idx_store_commands_history
    ON store_commands (store_id, tenant_id, created_at DESC);

-- Note: store_pending_ops intentionally has no TTL cleanup index.
-- Rows represent undelivered Console edits and are deleted on delivery.
