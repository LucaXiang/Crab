-- =============================================================================
-- 0008_name_field_cleanup
-- Unify name fields: remove display_name, make receipt_name optional
-- =============================================================================

-- store_price_rules: name ← display_name, drop display_name, receipt_name nullable
UPDATE store_price_rules SET name = display_name;
ALTER TABLE store_price_rules DROP COLUMN display_name;
ALTER TABLE store_price_rules ALTER COLUMN receipt_name DROP NOT NULL;

-- store_employees: rename display_name → name
ALTER TABLE store_employees RENAME COLUMN display_name TO name;
