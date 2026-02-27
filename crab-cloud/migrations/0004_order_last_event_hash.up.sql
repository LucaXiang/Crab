-- Add last_event_hash column for cloud-side order hash re-verification
ALTER TABLE store_archived_orders ADD COLUMN last_event_hash TEXT;
