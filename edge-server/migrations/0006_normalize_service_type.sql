-- Normalize service_type values from PascalCase to SCREAMING_SNAKE_CASE
UPDATE archived_order SET service_type = 'DINE_IN' WHERE service_type = 'DineIn';
UPDATE archived_order SET service_type = 'TAKEOUT' WHERE service_type = 'Takeout';
