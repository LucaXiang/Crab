-- Add global kitchen/label printing toggles to print_config
ALTER TABLE print_config ADD COLUMN kitchen_enabled INTEGER NOT NULL DEFAULT 1;
ALTER TABLE print_config ADD COLUMN label_enabled INTEGER NOT NULL DEFAULT 1;
