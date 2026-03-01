-- Add operator_id to store_anulaciones for audit completeness
ALTER TABLE store_anulaciones ADD COLUMN operator_id BIGINT NOT NULL DEFAULT 0;
