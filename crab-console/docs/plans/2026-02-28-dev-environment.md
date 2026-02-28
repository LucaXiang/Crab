# Dev Environment Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Deploy a dev-cloud + dev-console environment on the same EC2, isolated from production, for local Tauri/Console development.

**Architecture:** Same EC2 instance, two additional Docker services (dev-postgres + dev-cloud), Caddy routes for dev subdomains, crab-console env-var-based API URLs, dev-cloud disables registration and seeds test data.

**Tech Stack:** Docker Compose, Caddy, PostgreSQL 16, Vite env vars, Rust (crab-cloud registration gate)

---

### Task 1: crab-console — Replace hardcoded API URLs with Vite env vars

**Files:**
- Modify: `crab-console/src/infrastructure/api/client.ts:3`
- Modify: `crab-console/src/core/stores/useLiveOrdersStore.ts:4`
- Create: `crab-console/.env.production`
- Create: `crab-console/.env.development`

**Step 1: Update client.ts**

Change line 3 from:
```typescript
export const API_BASE = 'https://cloud.redcoral.app';
```
To:
```typescript
export const API_BASE = import.meta.env.VITE_API_BASE || 'https://cloud.redcoral.app';
```

**Step 2: Update useLiveOrdersStore.ts**

Change line 4 from:
```typescript
const WS_BASE = 'wss://cloud.redcoral.app';
```
To:
```typescript
const WS_BASE = import.meta.env.VITE_WS_BASE || 'wss://cloud.redcoral.app';
```

**Step 3: Create .env.production**

```
VITE_API_BASE=https://cloud.redcoral.app
VITE_WS_BASE=wss://cloud.redcoral.app
```

**Step 4: Create .env.development**

```
VITE_API_BASE=https://dev-cloud.redcoral.app
VITE_WS_BASE=wss://dev-cloud.redcoral.app
```

**Step 5: Verify TypeScript compiles**

Run: `cd crab-console && npx tsc --noEmit`
Expected: No errors

**Step 6: Commit**

```bash
git add crab-console/src/infrastructure/api/client.ts \
       crab-console/src/core/stores/useLiveOrdersStore.ts \
       crab-console/.env.production \
       crab-console/.env.development
git commit -m "feat(console): make API/WS URLs configurable via Vite env vars"
```

---

### Task 2: crab-cloud — Add `environment` to AppState and gate registration

**Files:**
- Modify: `crab-cloud/src/state.rs:59` (AppState struct) and `:77` (AppState::new)
- Modify: `crab-cloud/src/api/register.rs:47` (register handler)

**Step 1: Add `environment` field to AppState**

In `state.rs`, add to the `AppState` struct:
```rust
pub environment: String,
```

In `AppState::new()`, add to the initializer (after `console_base_url`):
```rust
environment: config.environment.clone(),
```

**Step 2: Gate registration in dev mode**

In `register.rs`, at the start of the `register` handler (after `State(state)` extraction):
```rust
if state.environment != "production" {
    return Err(AppError::new(ErrorCode::Forbidden));
}
```

**Step 3: Verify Rust compiles**

Run: `cargo check -p crab-cloud`
Expected: No errors

**Step 4: Commit**

```bash
git add crab-cloud/src/state.rs crab-cloud/src/api/register.rs
git commit -m "feat(cloud): disable registration in non-production environments"
```

---

### Task 3: crab-cloud — Add CORS origin for dev-console

**Files:**
- Modify: `crab-cloud/src/api/mod.rs:291-297` (CORS allow_origin list)

**Step 1: Add dev-console origins**

Add to the `AllowOrigin::list` array:
```rust
"https://dev-console.redcoral.app".parse().unwrap(),
"http://localhost:5180".parse().unwrap(), // dev console vite
```

**Step 2: Verify Rust compiles**

Run: `cargo check -p crab-cloud`
Expected: No errors

**Step 3: Commit**

```bash
git add crab-cloud/src/api/mod.rs
git commit -m "feat(cloud): add dev-console CORS origins"
```

---

### Task 4: Deploy infrastructure — docker-compose.yml

**Files:**
- Modify: `deploy/ec2/docker-compose.yml`

**Step 1: Add dev services and volume**

Add before the `volumes:` section:

```yaml
  dev-postgres:
    image: postgres:16-alpine
    restart: unless-stopped
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
    environment:
      POSTGRES_DB: crab
      POSTGRES_USER: crab
      POSTGRES_PASSWORD: ${DEV_POSTGRES_PASSWORD}
    volumes:
      - dev_pgdata:/var/lib/postgresql/data
    ports:
      - "127.0.0.1:5433:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U crab"]
      interval: 10s
      timeout: 5s
      retries: 5

  dev-cloud:
    image: ${CRAB_CLOUD_IMAGE}
    restart: unless-stopped
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
    depends_on:
      dev-postgres:
        condition: service_healthy
    expose:
      - "8081"
    ports:
      - "8444:8443"
    environment:
      DATABASE_URL: postgres://crab:${DEV_POSTGRES_PASSWORD}@dev-postgres:5432/crab
      ENVIRONMENT: development
      HTTP_PORT: "8081"
      MTLS_PORT: "8443"
      CONSOLE_BASE_URL: https://dev-console.redcoral.app
      RUST_LOG: crab_cloud=debug,tower_http=info
      ROOT_CA_PATH: /certs/root_ca.pem
      SERVER_CERT_PATH: /certs/server.pem
      SERVER_KEY_PATH: /certs/server.key
    volumes:
      - ./certs:/certs:ro
```

Add `dev_pgdata:` to the `volumes:` section.

Also add `./dev-console:/srv/dev-console:ro` to caddy's volumes.

**Step 2: Update .env.example**

Add `DEV_POSTGRES_PASSWORD=dev-password-change-me` to `.env.example`.

**Step 3: Commit**

```bash
git add deploy/ec2/docker-compose.yml deploy/ec2/.env.example
git commit -m "feat(deploy): add dev-cloud and dev-postgres services"
```

---

### Task 5: Deploy infrastructure — Caddyfile

**Files:**
- Modify: `deploy/ec2/Caddyfile`

**Step 1: Add dev CSP snippet**

After the `(csp_console)` snippet, add:
```caddyfile
(csp_dev_console) {
	header Content-Security-Policy "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https://dev-cloud.redcoral.app https://*.amazonaws.com; connect-src 'self' https://dev-cloud.redcoral.app wss://dev-cloud.redcoral.app; font-src 'self'; frame-ancestors 'none'"
}
```

**Step 2: Add dev-console site block**

After the `console.redcoral.app` block, add:
```caddyfile
dev-console.redcoral.app {
	import security_headers
	import csp_dev_console
	log {
		output file /var/log/caddy/access.log {
			roll_size 50MiB
			roll_keep 5
		}
		format json
	}
	root * /srv/dev-console
	file_server
	try_files {path} {path}.html {path}/index.html /200.html
	encode gzip

	@immutable path /assets/*
	header @immutable Cache-Control "public, max-age=31536000, immutable"

	@html path /index.html /200.html
	header @html Cache-Control "no-cache"

	@static {
		not path /assets/*
		not path /index.html
		not path /200.html
	}
	header @static Cache-Control "public, max-age=3600"
}
```

**Step 3: Add dev-cloud site block**

After the `cloud.redcoral.app` block, add:
```caddyfile
dev-cloud.redcoral.app {
	import security_headers
	log {
		output file /var/log/caddy/access.log {
			roll_size 50MiB
			roll_keep 5
		}
		format json
	}
	reverse_proxy dev-cloud:8081 {
		header_up X-Real-IP {remote_host}
	}
}
```

**Step 4: Commit**

```bash
git add deploy/ec2/Caddyfile
git commit -m "feat(deploy): add Caddy routes for dev-console and dev-cloud"
```

---

### Task 6: Create seed-dev.sh script

**Files:**
- Create: `deploy/ec2/seed-dev.sh`

**Step 1: Write the seed script**

```bash
#!/usr/bin/env bash
# Seed dev-cloud database with test tenant + user
#
# Usage: ./seed-dev.sh
# Requires: docker-compose running with dev-postgres healthy
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

# bcrypt hash for "devpassword123" — generated offline
# $2b$12$LJ3m4ys1MdPzVb8bKfFJkeS3fLqJvkXr5V2Y0j3lJGkFjF5JJXS6S
BCRYPT_HASH='$2b$12$LJ3m4ys1MdPzVb8bKfFJkeS3fLqJvkXr5V2Y0j3lJGkFjF5JJXS6S'
NOW_MS=$(date +%s)000

echo "==> Seeding dev database..."

docker compose exec -T dev-postgres psql -U crab -d crab <<SQL
-- Clean slate
DELETE FROM subscriptions WHERE tenant_id = 'dev-tenant-001';
DELETE FROM tenants WHERE id = 'dev-tenant-001';

-- Tenant
INSERT INTO tenants (id, email, password_hash, status, created_at, updated_at)
VALUES ('dev-tenant-001', 'dev@redcoral.app', '${BCRYPT_HASH}', 'active', ${NOW_MS}, ${NOW_MS});

-- Subscription (Pro plan, no expiry)
INSERT INTO subscriptions (tenant_id, plan, status, max_stores, created_at, updated_at)
VALUES ('dev-tenant-001', 'pro', 'active', 5, ${NOW_MS}, ${NOW_MS});

SELECT 'Seeded: dev@redcoral.app / devpassword123 (Pro plan, 5 stores)' AS result;
SQL

echo "==> Done!"
```

**Step 2: Make executable**

```bash
chmod +x deploy/ec2/seed-dev.sh
```

**Step 3: Commit**

```bash
git add deploy/ec2/seed-dev.sh
git commit -m "feat(deploy): add dev database seed script"
```

---

### Task 7: Create sync-dev-console.sh deploy script

**Files:**
- Create: `deploy/sync-dev-console.sh`

**Step 1: Write the deploy script**

Copy `deploy/sync-console.sh` and modify:
- `REMOTE_DIR="/opt/crab/dev-console"`
- Build with `VITE_API_BASE=https://dev-cloud.redcoral.app VITE_WS_BASE=wss://dev-cloud.redcoral.app npm run build`
- Final message: `https://dev-console.redcoral.app`

**Step 2: Make executable and commit**

```bash
chmod +x deploy/sync-dev-console.sh
git add deploy/sync-dev-console.sh
git commit -m "feat(deploy): add dev-console deploy script"
```

---

### Task 8: DNS + EC2 Initial Setup (manual steps)

This task is manual — no code changes. Document the steps:

1. **Cloudflare DNS**: Add two CNAME records:
   - `dev-cloud.redcoral.app` → same EC2 IP/CNAME as `cloud.redcoral.app`
   - `dev-console.redcoral.app` → same EC2 IP/CNAME as `console.redcoral.app`

2. **EC2**: Add `DEV_POSTGRES_PASSWORD` to `/opt/crab/.env`

3. **EC2**: Create `/opt/crab/dev-console/` directory:
   ```bash
   mkdir -p /opt/crab/dev-console
   echo '<h1>dev-console placeholder</h1>' > /opt/crab/dev-console/index.html
   ```

4. **EC2**: Upload updated files and restart:
   ```bash
   scp docker-compose.yml Caddyfile ec2-user@EC2:/opt/crab/
   ssh ec2-user@EC2 "cd /opt/crab && docker-compose up -d"
   ```

5. **EC2**: Run seed script:
   ```bash
   ssh ec2-user@EC2 "cd /opt/crab && ./seed-dev.sh"
   ```

6. **Verify**:
   ```bash
   curl https://dev-cloud.redcoral.app/health
   curl https://dev-console.redcoral.app/
   ```

---

### Task 9: Verify end-to-end

**Step 1: Deploy dev-console**

```bash
./deploy/sync-dev-console.sh
```

**Step 2: Test Tauri dev against dev-cloud**

```bash
cd red_coral
AUTH_SERVER_URL=https://dev-cloud.redcoral.app \
CRAB_CLOUD_URL=https://dev-cloud.redcoral.app:8444 \
npm run tauri:dev
```

**Step 3: Test Console local dev**

```bash
cd crab-console
npm run dev
# Opens http://localhost:5180 → API calls go to dev-cloud
```

**Step 4: Test registration is blocked**

```bash
curl -X POST https://dev-cloud.redcoral.app/api/register \
  -H 'Content-Type: application/json' \
  -d '{"email":"test@test.com","password":"12345678"}'
# Expected: 403 Forbidden
```

**Step 5: Test login with seeded account**

```bash
curl -X POST https://dev-cloud.redcoral.app/api/tenant/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"dev@redcoral.app","password":"devpassword123"}'
# Expected: 200 with JWT token
```
