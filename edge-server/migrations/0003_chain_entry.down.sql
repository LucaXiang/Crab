-- Reverse: restore hash columns to archived_order, drop chain_entry

-- 1. Re-add columns to system_state
ALTER TABLE system_state ADD COLUMN last_order_hash TEXT;
UPDATE system_state SET last_order_hash = last_chain_hash;
ALTER TABLE system_state DROP COLUMN last_chain_hash;

-- 2. Re-add hash columns to archived_order
ALTER TABLE archived_order ADD COLUMN prev_hash TEXT NOT NULL DEFAULT '';
ALTER TABLE archived_order ADD COLUMN curr_hash TEXT NOT NULL DEFAULT '';

-- 3. Restore hash data from chain_entry
UPDATE archived_order
SET prev_hash = (SELECT ce.prev_hash FROM chain_entry ce WHERE ce.entry_type = 'ORDER' AND ce.entry_pk = archived_order.id),
    curr_hash = (SELECT ce.curr_hash FROM chain_entry ce WHERE ce.entry_type = 'ORDER' AND ce.entry_pk = archived_order.id);

-- 4. Restore index
CREATE INDEX idx_archived_order_hash ON archived_order(curr_hash);

-- 5. Drop chain_entry
DROP TABLE chain_entry;
