-- Prevent duplicate items per order by enforcing unique (order_pk, instance_id)
CREATE UNIQUE INDEX idx_archived_item_order_instance ON archived_order_item(order_pk, instance_id);
