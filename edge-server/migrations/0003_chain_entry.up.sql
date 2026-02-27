-- =============================================================================
-- chain_entry: unified hash chain index for orders and future credit notes
-- =============================================================================

-- 1. Create chain_entry table
CREATE TABLE chain_entry (
    id          INTEGER PRIMARY KEY,
    entry_type  TEXT    NOT NULL,         -- 'ORDER' | 'CREDIT_NOTE' (extensible)
    entry_pk    INTEGER NOT NULL,         -- FK to archived_order.id (or credit_note.id)
    prev_hash   TEXT    NOT NULL,
    curr_hash   TEXT    NOT NULL,
    created_at  INTEGER NOT NULL
);

CREATE INDEX idx_chain_entry_created ON chain_entry(created_at);
CREATE INDEX idx_chain_entry_type ON chain_entry(entry_type);
CREATE INDEX idx_chain_entry_entry ON chain_entry(entry_type, entry_pk);

-- 2. Migrate existing hash data from archived_order into chain_entry
INSERT INTO chain_entry (entry_type, entry_pk, prev_hash, curr_hash, created_at)
SELECT 'ORDER', id, prev_hash, curr_hash, created_at
FROM archived_order
ORDER BY id ASC;

-- 3. Add last_chain_hash to system_state (copy from last_order_hash)
ALTER TABLE system_state ADD COLUMN last_chain_hash TEXT;
UPDATE system_state SET last_chain_hash = last_order_hash;

-- 4. Remove hash columns from archived_order
-- Drop the index first, then the columns
DROP INDEX IF EXISTS idx_archived_order_hash;
ALTER TABLE archived_order DROP COLUMN prev_hash;
ALTER TABLE archived_order DROP COLUMN curr_hash;

-- 5. Remove last_order_hash from system_state
ALTER TABLE system_state DROP COLUMN last_order_hash;
