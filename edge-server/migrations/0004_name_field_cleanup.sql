-- =============================================================================
-- 0004_name_field_cleanup.sql
-- Unify name fields: remove display_name, make receipt_name optional
-- =============================================================================

-- 1. role: name ← display_name, drop display_name
UPDATE role SET name = display_name WHERE display_name != '';
ALTER TABLE role DROP COLUMN display_name;

-- 2. employee: rename display_name → name
ALTER TABLE employee RENAME COLUMN display_name TO name;

-- 3. price_rule: name ← display_name value, drop both old columns, recreate name
ALTER TABLE price_rule ADD COLUMN new_name TEXT NOT NULL DEFAULT '';
UPDATE price_rule SET new_name = display_name;
ALTER TABLE price_rule DROP COLUMN name;
ALTER TABLE price_rule DROP COLUMN display_name;
ALTER TABLE price_rule RENAME COLUMN new_name TO name;

-- 4. marketing_group: name ← display_name, drop display_name
UPDATE marketing_group SET name = display_name;
ALTER TABLE marketing_group DROP COLUMN display_name;

-- 5. mg_discount_rule: same pattern as price_rule
ALTER TABLE mg_discount_rule ADD COLUMN new_name TEXT NOT NULL DEFAULT '';
UPDATE mg_discount_rule SET new_name = display_name;
ALTER TABLE mg_discount_rule DROP COLUMN name;
ALTER TABLE mg_discount_rule DROP COLUMN display_name;
ALTER TABLE mg_discount_rule RENAME COLUMN new_name TO name;

-- 6. stamp_activity: same pattern
ALTER TABLE stamp_activity ADD COLUMN new_name TEXT NOT NULL DEFAULT '';
UPDATE stamp_activity SET new_name = display_name;
ALTER TABLE stamp_activity DROP COLUMN name;
ALTER TABLE stamp_activity DROP COLUMN display_name;
ALTER TABLE stamp_activity RENAME COLUMN new_name TO name;
