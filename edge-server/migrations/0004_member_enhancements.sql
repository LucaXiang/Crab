-- Member enhancements: email, total_spent, remove points_per_unit, archive member tracking

-- Add email and total_spent to member
ALTER TABLE member ADD COLUMN email TEXT;
ALTER TABLE member ADD COLUMN total_spent REAL NOT NULL DEFAULT 0;

-- Remove points_per_unit from marketing_group (simplified to rate-only model)
ALTER TABLE marketing_group DROP COLUMN points_per_unit;

-- Add member tracking to archived_order (for member spending history queries)
ALTER TABLE archived_order ADD COLUMN member_id INTEGER;
ALTER TABLE archived_order ADD COLUMN member_name TEXT;
CREATE INDEX idx_archived_order_member ON archived_order(member_id);
