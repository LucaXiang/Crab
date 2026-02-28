#!/usr/bin/env bash
# Seed dev-cloud database with test tenant + user
#
# Usage: ./seed-dev.sh
# Requires: docker compose running with dev-postgres healthy
#
# Test account: dev@redcoral.app / devpassword123

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Load DEV_POSTGRES_PASSWORD from .env
if [ -f "$SCRIPT_DIR/.env" ]; then
    DEV_POSTGRES_PASSWORD=$(grep -E '^DEV_POSTGRES_PASSWORD=' "$SCRIPT_DIR/.env" | cut -d'=' -f2 || true)
fi

if [ -z "${DEV_POSTGRES_PASSWORD:-}" ]; then
    echo "ERROR: DEV_POSTGRES_PASSWORD not set in .env"
    exit 1
fi

# Argon2id hash for "devpassword123"
ARGON_HASH='$argon2id$v=19$m=19456,t=2,p=1$hjdUjOcSHtOtfZMhFtkiIw$48KuANCYJhmFtK3P+tt89hIJf091lxPS8B2igsNPm7c'
NOW_MS=$(date +%s)000

echo "==> Seeding dev database..."

docker-compose exec -T dev-postgres psql -U crab -d crab <<SQL
-- Clean slate
DELETE FROM subscriptions WHERE tenant_id = 'dev-tenant-001';
DELETE FROM tenants WHERE id = 'dev-tenant-001';

-- Tenant
INSERT INTO tenants (id, email, hashed_password, status, created_at)
VALUES ('dev-tenant-001', 'dev@redcoral.app', '${ARGON_HASH}', 'active', ${NOW_MS});

-- Subscription (Pro plan, no expiry)
INSERT INTO subscriptions (id, tenant_id, plan, status, max_stores, created_at)
VALUES ('dev-sub-001', 'dev-tenant-001', 'pro', 'active', 5, ${NOW_MS});

SELECT 'Seeded: dev@redcoral.app / devpassword123 (Pro plan, 5 stores)' AS result;
SQL

echo "==> Done!"
