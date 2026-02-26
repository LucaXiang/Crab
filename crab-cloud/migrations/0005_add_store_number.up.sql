-- Add store_number to edge_servers (per-tenant sequential number)
ALTER TABLE edge_servers ADD COLUMN store_number INT;

-- Backfill: assign store_number by registration order within each tenant
UPDATE edge_servers SET store_number = sub.rn
FROM (
    SELECT id, ROW_NUMBER() OVER (PARTITION BY tenant_id ORDER BY registered_at) AS rn
    FROM edge_servers
) sub
WHERE edge_servers.id = sub.id;

ALTER TABLE edge_servers ALTER COLUMN store_number SET NOT NULL;
