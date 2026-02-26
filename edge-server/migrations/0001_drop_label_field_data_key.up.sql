-- Remove redundant data_key column from label_field.
-- data_source is now the single source of truth for field data binding.
ALTER TABLE label_field DROP COLUMN data_key;
