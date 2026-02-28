# Dev Environment Design

Date: 2026-02-28

## Purpose

Local Tauri/Console development against a stable remote dev-cloud, isolated from production data and users.

## Infrastructure

Same EC2 instance as production. Two new DNS CNAME records pointing to the same EC2:
- `dev-cloud.redcoral.app` — dev API + WebSocket
- `dev-console.redcoral.app` — dev console SPA

### Port & Domain Mapping

| Env | HTTP API | mTLS | Console | Database |
|-----|----------|------|---------|----------|
| prod | `cloud.redcoral.app` → `:8080` | `:8443` | `console.redcoral.app` | PG `:5432` |
| dev | `dev-cloud.redcoral.app` → `:8081` | `:8444` | `dev-console.redcoral.app` | PG `:5433` |

### docker-compose.yml additions

Two new services added to existing compose:

- **dev-postgres**: `postgres:16-alpine`, port `127.0.0.1:5433:5432`, separate `dev_pgdata` volume
- **dev-cloud**: Same ECR image as prod, `HTTP_PORT=8081`, mTLS mapped `8444:8443`, `ENVIRONMENT=development` (secrets auto-fallback), `CONSOLE_BASE_URL=https://dev-console.redcoral.app`, reuses same mTLS certs

### Caddyfile additions

Two new site blocks:
- `dev-console.redcoral.app` → file_server from `/srv/dev-console`
- `dev-cloud.redcoral.app` → reverse_proxy to `dev-cloud:8081`

Caddy auto-provisions HTTPS for both dev domains.

## Code Changes

### crab-console: Environment-based API URLs

Replace hardcoded URLs with Vite env vars:

- `client.ts`: `API_BASE = import.meta.env.VITE_API_BASE || 'https://cloud.redcoral.app'`
- `useLiveOrdersStore.ts`: `WS_BASE = import.meta.env.VITE_WS_BASE || 'wss://cloud.redcoral.app'`

New files:
- `.env.production`: prod URLs (default for `npm run build`)
- `.env.development`: dev URLs (default for `npm run dev`)

### Tauri dev: Already supported

Set env vars before `npm run tauri:dev`:
```
AUTH_SERVER_URL=https://dev-cloud.redcoral.app
CRAB_CLOUD_URL=https://dev-cloud.redcoral.app:8444
```

### crab-cloud: No code changes needed

`ENVIRONMENT=development` already allows missing Stripe/JWT/Email secrets.

## Dev Environment Restrictions

- **Registration disabled** in dev mode — registration API returns 403 when `ENVIRONMENT=development`
- **Test accounts seeded** via `deploy/ec2/seed-dev.sh`:
  - Tenant: `dev-tenant-001`, status=active
  - User: `dev@redcoral.app`, verified, fixed password
  - Subscription: Pro plan, no expiry, max_stores=5
  - P12: skipped (not enforced in dev mode)

## Deployment Workflows

### Initial Setup (one-time)
1. Add DNS CNAME records in Cloudflare
2. Update docker-compose.yml and Caddyfile on EC2
3. `docker-compose up -d` — Caddy auto-provisions HTTPS
4. Run `seed-dev.sh` to initialize test data

### Update dev-cloud
```bash
docker pull <ecr-image>:latest
docker-compose restart dev-cloud
```

### Deploy dev-console
```bash
cd crab-console
VITE_API_BASE=https://dev-cloud.redcoral.app VITE_WS_BASE=wss://dev-cloud.redcoral.app npm run build
scp -r build/* ec2-user@EC2:/opt/crab/dev-console/
```

### Local Tauri dev
```bash
cd red_coral
AUTH_SERVER_URL=https://dev-cloud.redcoral.app \
CRAB_CLOUD_URL=https://dev-cloud.redcoral.app:8444 \
npm run tauri:dev
```
