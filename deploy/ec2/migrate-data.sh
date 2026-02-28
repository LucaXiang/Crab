#!/bin/bash
# ══════════════════════════════════════════════════════════════
# Cloud DB Migration: export data → drop → recreate → import
# Run this ON the EC2 instance (cd /opt/crab)
# ══════════════════════════════════════════════════════════════
set -euo pipefail

CONTAINER="crab-postgres-1"
DB="crab"
USER="crab"
DUMP_DIR="/tmp/crab_migration"

psql_exec() {
  docker exec "$CONTAINER" psql -U "$USER" -d "$DB" -c "$1"
}

table_exists() {
  docker exec "$CONTAINER" psql -U "$USER" -d "$DB" -tAc \
    "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = '$1');" | grep -q 't'
}

echo "=== Step 1: Stop crab-cloud (keep PG running) ==="
docker-compose stop crab-cloud || true

echo "=== Step 2: Export data (with schema adaptation) ==="
docker exec "$CONTAINER" mkdir -p "$DUMP_DIR"

# ── Normal tables (no schema changes needed) ──
NORMAL_TABLES=(
  tenants
  email_verifications
  activations
  client_connections
  p12_certificates
  processed_webhook_events
  audit_logs
  store_sync_cursors
  store_versions
  store_tags
  store_categories
  store_category_print_dest
  store_category_tag
  store_products
  store_product_specs
  store_product_tag
  store_attributes
  store_attribute_options
  store_attribute_bindings
  store_price_rules
  store_zones
  store_dining_tables
  store_daily_reports
  store_daily_report_tax_breakdown
  store_daily_report_payment_breakdown
  store_daily_report_shift_breakdown
  store_shifts
  store_employees
  store_label_templates
  store_commands
  store_pending_ops
  tenant_images
  store_credit_notes
  store_invoices
)

for tbl in "${NORMAL_TABLES[@]}"; do
  if table_exists "$tbl"; then
    echo "  Exporting $tbl..."
    docker exec "$CONTAINER" psql -U "$USER" -d "$DB" \
      -c "\\COPY $tbl TO '$DUMP_DIR/$tbl.csv' WITH (FORMAT csv, HEADER true)"
  else
    echo "  Skipping $tbl (table does not exist in old DB)"
  fi
done

# ── Adapted tables (schema changed between old and new) ──

# subscriptions: max_edge_servers → max_stores (same semantics), drop max_clients (unused)
echo "  Exporting subscriptions (adapted: max_edge_servers → max_stores)..."
docker exec "$CONTAINER" psql -U "$USER" -d "$DB" \
  -c "\\COPY (SELECT id, tenant_id, status, plan, max_edge_servers AS max_stores, features, current_period_end, cancel_at_period_end, billing_interval, created_at FROM subscriptions) TO '$DUMP_DIR/subscriptions.csv' WITH (FORMAT csv, HEADER true)"

# stores: status='active' (all existing stores are active), deleted_at=NULL (not deleted)
echo "  Exporting stores (adapted: add status=active, deleted_at=NULL)..."
docker exec "$CONTAINER" psql -U "$USER" -d "$DB" \
  -c "\\COPY (SELECT id, entity_id, tenant_id, device_id, store_number, alias, name, address, phone, nif, email, website, logo_url, business_day_cutoff, last_sync_at, registered_at, 'active'::TEXT AS status, NULL::BIGINT AS deleted_at, created_at, updated_at FROM stores) TO '$DUMP_DIR/stores.csv' WITH (FORMAT csv, HEADER true)"

# store_archived_orders: last_event_hash=NULL (historical orders don't have event hashes)
echo "  Exporting store_archived_orders (adapted: add last_event_hash=NULL)..."
docker exec "$CONTAINER" psql -U "$USER" -d "$DB" \
  -c "\\COPY (SELECT id, store_id, tenant_id, source_id, order_key, receipt_number, status, end_time, total, tax, desglose, guest_count, discount_amount, void_type, loss_amount, start_time, prev_hash, curr_hash, NULL::TEXT AS last_event_hash, version, detail, synced_at FROM store_archived_orders) TO '$DUMP_DIR/store_archived_orders.csv' WITH (FORMAT csv, HEADER true)"

# store_label_fields: drop data_key column (removed from new schema)
echo "  Exporting store_label_fields (adapted: drop data_key)..."
docker exec "$CONTAINER" psql -U "$USER" -d "$DB" \
  -c "\\COPY (SELECT id, template_id, field_id, name, field_type, x, y, width, height, font_size, font_weight, font_family, color, rotate, alignment, data_source, format, visible, label, template, source_type, maintain_aspect_ratio, style, align, vertical_align, line_style FROM store_label_fields) TO '$DUMP_DIR/store_label_fields.csv' WITH (FORMAT csv, HEADER true)"

# refresh_tokens: SKIP — old data lacks user_agent/ip_address, users simply re-login
echo "  Skipping refresh_tokens (users will re-login after migration)"

echo "=== Step 3: Drop and recreate database ==="
docker exec "$CONTAINER" psql -U "$USER" -d postgres \
  -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '$DB' AND pid != pg_backend_pid();" \
  -c "DROP DATABASE $DB;" \
  -c "CREATE DATABASE $DB OWNER $USER;"

echo "=== Step 4: Start crab-cloud (runs migrations automatically) ==="
docker-compose up -d crab-cloud

echo "  Waiting for migrations to complete..."
sleep 5

# Verify health
for i in {1..10}; do
  if curl -sf http://localhost:8080/health > /dev/null 2>&1; then
    echo "  crab-cloud is healthy!"
    break
  fi
  echo "  Waiting... ($i/10)"
  sleep 3
done

echo "=== Step 5: Stop crab-cloud for data import ==="
docker-compose stop crab-cloud

echo "=== Step 6: Import data ==="

# All tables to import (FK order — parents before children)
ALL_TABLES=(
  tenants
  subscriptions
  email_verifications
  activations
  client_connections
  p12_certificates
  processed_webhook_events
  audit_logs
  stores
  store_sync_cursors
  store_versions
  store_tags
  store_categories
  store_category_print_dest
  store_category_tag
  store_products
  store_product_specs
  store_product_tag
  store_attributes
  store_attribute_options
  store_attribute_bindings
  store_price_rules
  store_zones
  store_dining_tables
  store_archived_orders
  store_credit_notes
  store_invoices
  store_daily_reports
  store_daily_report_tax_breakdown
  store_daily_report_payment_breakdown
  store_daily_report_shift_breakdown
  store_shifts
  store_employees
  store_label_templates
  store_label_fields
  store_commands
  store_pending_ops
  tenant_images
)

SEQUENCE_TABLES=()

for tbl in "${ALL_TABLES[@]}"; do
  FILE="$DUMP_DIR/$tbl.csv"
  # Check if file exists and has data (more than just header)
  LINES=$(docker exec "$CONTAINER" wc -l < "$FILE" 2>/dev/null || echo "0")
  if [ "$LINES" -le 1 ]; then
    echo "  Skipping $tbl (empty or no export)"
    continue
  fi

  echo "  Importing $tbl..."
  # Get columns from CSV header (first line)
  COLS=$(docker exec "$CONTAINER" head -1 "$FILE")
  docker exec "$CONTAINER" psql -U "$USER" -d "$DB" \
    -c "\\COPY $tbl($COLS) FROM '$FILE' WITH (FORMAT csv, HEADER true)"
  SEQUENCE_TABLES+=("$tbl")
done

echo "=== Step 7: Fix sequences ==="
# Reset BIGSERIAL sequences to max(id) + 1
for tbl in "${SEQUENCE_TABLES[@]}"; do
  docker exec "$CONTAINER" psql -U "$USER" -d "$DB" -c "
    DO \$\$
    DECLARE
      seq_name TEXT;
      max_id BIGINT;
    BEGIN
      SELECT pg_get_serial_sequence('$tbl', 'id') INTO seq_name;
      IF seq_name IS NOT NULL THEN
        EXECUTE 'SELECT COALESCE(MAX(id), 0) FROM $tbl' INTO max_id;
        IF max_id > 0 THEN
          EXECUTE 'SELECT setval(''' || seq_name || ''', ' || max_id || ')';
          RAISE NOTICE 'Reset % to %', seq_name, max_id;
        END IF;
      END IF;
    END \$\$;
  " 2>&1 | grep -v "^$" || true
done

echo "=== Step 8: Cleanup and restart ==="
docker exec "$CONTAINER" rm -rf "$DUMP_DIR"
docker-compose up -d crab-cloud

echo "  Waiting for health check..."
sleep 5
curl -sf http://localhost:8080/health && echo "" || echo "WARNING: health check failed"

echo "=== Step 9: Verify data ==="
psql_exec "
SELECT 'tenants' as tbl, COUNT(*) FROM tenants
UNION ALL SELECT 'stores', COUNT(*) FROM stores
UNION ALL SELECT 'subscriptions', COUNT(*) FROM subscriptions
UNION ALL SELECT 'activations', COUNT(*) FROM activations
UNION ALL SELECT 'store_products', COUNT(*) FROM store_products
UNION ALL SELECT 'store_archived_orders', COUNT(*) FROM store_archived_orders
UNION ALL SELECT 'store_credit_notes', COUNT(*) FROM store_credit_notes
UNION ALL SELECT 'store_invoices', COUNT(*) FROM store_invoices
UNION ALL SELECT 'store_employees', COUNT(*) FROM store_employees
ORDER BY tbl;
"

echo ""
echo "=== Migration complete! ==="
echo "NOTE: refresh_tokens were NOT imported — users need to re-login."
