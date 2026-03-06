-- Add MG discount + per-item rule breakdown fields

ALTER TABLE store_archived_orders ADD COLUMN mg_discount_amount NUMERIC(12,2) NOT NULL DEFAULT 0;
ALTER TABLE store_archived_orders ADD COLUMN marketing_group_name TEXT;

ALTER TABLE store_order_items ADD COLUMN rule_discount_amount NUMERIC(12,2) NOT NULL DEFAULT 0;
ALTER TABLE store_order_items ADD COLUMN rule_surcharge_amount NUMERIC(12,2) NOT NULL DEFAULT 0;
ALTER TABLE store_order_items ADD COLUMN mg_discount_amount NUMERIC(12,2) NOT NULL DEFAULT 0;

-- Unified adjustment tracking table
-- Records ALL sources of price adjustments: price rules, manual discounts, MG discounts, etc.
-- Both item-level and order-level adjustments.
CREATE TABLE store_order_adjustments (
    id                BIGSERIAL PRIMARY KEY,
    order_id          BIGINT NOT NULL REFERENCES store_archived_orders(id) ON DELETE CASCADE,
    item_id           BIGINT REFERENCES store_order_items(id) ON DELETE CASCADE,
    -- NULL item_id = order-level adjustment

    -- Source type: PRICE_RULE, MANUAL, MEMBER_GROUP, COMP
    source_type       TEXT NOT NULL,
    -- Direction: DISCOUNT or SURCHARGE
    direction         TEXT NOT NULL,

    -- Price rule specifics (NULL for non-rule sources)
    rule_id           BIGINT,
    rule_name         TEXT,
    rule_receipt_name TEXT,
    adjustment_type   TEXT,          -- PERCENTAGE / FIXED_AMOUNT (for rules)

    -- Amount
    amount            NUMERIC(12,2) NOT NULL DEFAULT 0,
    skipped           BOOLEAN NOT NULL DEFAULT false
);

CREATE INDEX idx_soa_order ON store_order_adjustments(order_id);
CREATE INDEX idx_soa_item ON store_order_adjustments(item_id) WHERE item_id IS NOT NULL;
CREATE INDEX idx_soa_source ON store_order_adjustments(order_id, source_type);
