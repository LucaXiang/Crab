-- Add service_type to archived_order (DineIn, Takeout, etc.)
ALTER TABLE archived_order ADD COLUMN service_type TEXT;
