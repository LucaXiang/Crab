#!/usr/bin/env bash
# Seed dev-cloud database with test tenant + user
#
# Usage: ./seed-dev.sh
# Requires: docker compose running with dev-postgres healthy
#
# Test account: dev@redcoral.app / W4xzy123

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

NOW_MS=$(date +%s)000

echo "==> Seeding dev database..."

# NOTE: heredoc uses <<'SQL' (single-quoted) to prevent shell expansion of $ in argon2 hash
docker-compose exec -T dev-postgres psql -U crab -d crab <<'SQL'
-- Clean slate
DELETE FROM subscriptions WHERE tenant_id = 1;
DELETE FROM tenants WHERE id = 1;

-- Tenant (id=1, status=active, skipping email verification)
INSERT INTO tenants (id, email, hashed_password, status, created_at)
VALUES (1, 'dev@redcoral.app', '$argon2id$v=19$m=65536,t=3,p=4$8QH3Qo2kWMWpphWxZM/Fxw$PLjsR9Z+YDT+lSQmF8fL6+SopFv+o+1d/Zlktf38u74', 'active', 1740787200000);

-- Subscription (Pro plan, no expiry)
INSERT INTO subscriptions (id, tenant_id, plan, status, max_stores, created_at)
VALUES ('dev-sub-001', 1, 'pro', 'active', 5, 1740787200000);

SELECT 'Seeded: dev@redcoral.app / W4xzy123 (Pro plan, 5 stores)' AS result;
SQL

echo "==> Done!"
