ALTER TABLE stores ADD COLUMN status TEXT NOT NULL DEFAULT 'active';
ALTER TABLE stores ADD COLUMN deleted_at BIGINT;
ALTER TABLE subscriptions RENAME COLUMN max_edge_servers TO max_stores;
ALTER TABLE subscriptions DROP COLUMN max_clients;
