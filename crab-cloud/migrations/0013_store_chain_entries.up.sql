CREATE TABLE store_chain_entries (
    id          BIGSERIAL PRIMARY KEY,
    store_id    BIGINT NOT NULL,
    tenant_id   BIGINT NOT NULL,
    source_id   BIGINT NOT NULL,          -- chain_entry.id from edge
    entry_type  TEXT   NOT NULL,           -- ORDER | CREDIT_NOTE | ANULACION | UPGRADE | BREAK
    entry_pk    BIGINT NOT NULL,           -- resource pk (or failed entry id for BREAK)
    prev_hash   TEXT   NOT NULL,
    curr_hash   TEXT   NOT NULL,
    created_at  BIGINT NOT NULL,
    synced_at   BIGINT NOT NULL,
    UNIQUE(tenant_id, store_id, source_id)
);

CREATE INDEX idx_sce_store ON store_chain_entries(store_id, tenant_id);
CREATE INDEX idx_sce_created ON store_chain_entries(store_id, tenant_id, created_at DESC);
