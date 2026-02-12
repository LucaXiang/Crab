-- Printer model redesign: separate connection, protocol, and purpose
-- 1. Rename printer fields: printer_type -> connection, printer_format -> protocol
-- 2. Add purpose to print_destination: 'kitchen' | 'label'
-- 3. Merge junction tables: category_kitchen_print_dest + category_label_print_dest -> category_print_dest

-- ── Step 1: Rename printer columns ──────────────────────────────────────
ALTER TABLE printer RENAME COLUMN printer_type TO connection;
ALTER TABLE printer RENAME COLUMN printer_format TO protocol;

-- ── Step 2: Add purpose to print_destination ────────────────────────────
ALTER TABLE print_destination ADD COLUMN purpose TEXT NOT NULL DEFAULT 'kitchen';

-- Infer purpose from existing junction table usage:
-- Destinations only in category_label_print_dest -> 'label'
UPDATE print_destination SET purpose = 'label'
WHERE id IN (
    SELECT DISTINCT print_destination_id FROM category_label_print_dest
)
AND id NOT IN (
    SELECT DISTINCT print_destination_id FROM category_kitchen_print_dest
);

-- ── Step 3: Merge junction tables ───────────────────────────────────────
CREATE TABLE category_print_dest (
    category_id          INTEGER NOT NULL REFERENCES category(id) ON DELETE CASCADE,
    print_destination_id INTEGER NOT NULL REFERENCES print_destination(id) ON DELETE CASCADE,
    PRIMARY KEY (category_id, print_destination_id)
);

-- Migrate data from both old tables
INSERT OR IGNORE INTO category_print_dest (category_id, print_destination_id)
SELECT category_id, print_destination_id FROM category_kitchen_print_dest;

INSERT OR IGNORE INTO category_print_dest (category_id, print_destination_id)
SELECT category_id, print_destination_id FROM category_label_print_dest;

-- Drop old junction tables
DROP TABLE category_kitchen_print_dest;
DROP TABLE category_label_print_dest;
