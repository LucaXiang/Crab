-- Add queue_number to archived_order for retail order call numbers
ALTER TABLE archived_order ADD COLUMN queue_number INTEGER;
