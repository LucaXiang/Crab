-- Pending ops queue: stores Console edits when edge is offline
-- Drained on edge reconnect (WebSocket Welcome)
CREATE TABLE store_pending_ops (
    id BIGSERIAL PRIMARY KEY,
    edge_server_id BIGINT NOT NULL REFERENCES cloud_edge_servers(id) ON DELETE CASCADE,
    op JSONB NOT NULL,
    changed_at BIGINT NOT NULL,
    created_at BIGINT NOT NULL
);

CREATE INDEX idx_pending_ops_edge ON store_pending_ops(edge_server_id);
