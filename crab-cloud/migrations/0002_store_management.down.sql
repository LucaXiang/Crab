ALTER TABLE subscriptions ADD COLUMN max_clients INT NOT NULL DEFAULT 5;
ALTER TABLE subscriptions RENAME COLUMN max_stores TO max_edge_servers;
ALTER TABLE stores DROP COLUMN deleted_at;
ALTER TABLE stores DROP COLUMN status;
