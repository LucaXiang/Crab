DROP TABLE IF EXISTS store_order_adjustments;

ALTER TABLE store_order_items DROP COLUMN mg_discount_amount;
ALTER TABLE store_order_items DROP COLUMN rule_surcharge_amount;
ALTER TABLE store_order_items DROP COLUMN rule_discount_amount;

ALTER TABLE store_archived_orders DROP COLUMN marketing_group_name;
ALTER TABLE store_archived_orders DROP COLUMN mg_discount_amount;
