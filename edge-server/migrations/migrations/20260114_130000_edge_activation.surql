-- Initialize default record if not exists
INSERT INTO edge_activation (id, is_activated, updated_at) 
VALUES ('default', false, time::now()) 
ON DUPLICATE KEY UPDATE updated_at = time::now();
