# Tenant Self-Registration Design

## Context

crab-auth 的 `tenants` 和 `subscriptions` 表当前无数据源 — 没有注册 API、没有 Stripe 集成。生产环境无法创建租户。

需求：餐厅老板自助注册 → 邮箱验证 → Stripe 付款 → 激活设备。

## Decision

在 **crab-cloud** 中实现（非 crab-auth）。

理由：
- crab-cloud 定位是"云端租户管理中心"，CLAUDE.md 已标注 "Future: Stripe integration"
- crab-cloud 是长驻 Axum 服务，适合处理 Stripe webhook
- crab-cloud 与 crab-auth 共享同一个 PostgreSQL 实例
- crab-auth 保持纯粹的 PKI + 激活职责（只读 tenants/subscriptions）

## Registration Flow

```
餐厅老板                    crab-cloud                     AWS SES          Stripe
   |                            |                              |               |
   |-- POST /api/register ----->|                              |               |
   |   { email, password }      |-- 生成 6位验证码             |               |
   |                            |-- INSERT tenants (pending)   |               |
   |                            |-- 发验证码 ----------------->|               |
   |<-- 200 "验证码已发送" -----|                              |               |
   |                            |                              |               |
   |-- POST /api/verify-email ->|                              |               |
   |   { email, code }          |-- 校验验证码                 |               |
   |                            |-- UPDATE tenant → verified   |               |
   |                            |-- 创建 Stripe Customer       |-------------->|
   |                            |-- 创建 Checkout Session      |-------------->|
   |<-- 200 { checkout_url } ---|                              |               |
   |                            |                              |               |
   |-- 浏览器跳转 Stripe ------>|                              |               |
   |-- 付款完成                 |                              |               |
   |                            |<-- webhook: checkout.completed --------------|
   |                            |-- INSERT subscriptions (active)              |
   |                            |-- UPDATE tenant → active                    |
   |<-- 重定向到成功页 ---------|                              |               |
```

### Tenant Status Flow

```
pending → verified → active → suspended (欠费)
                                  ↓
                              canceled (主动取消)
```

- `pending`: 刚注册，未验证邮箱
- `verified`: 邮箱已验证，等待付款
- `active`: 付款完成，可以激活设备
- `suspended`: 付款失败 (invoice.payment_failed)
- `canceled`: 订阅取消 (subscription.deleted)

## Database Schema

### tenants (crab-cloud owns, crab-auth reads)

```sql
CREATE TABLE tenants (
    id                TEXT PRIMARY KEY,           -- UUID v4
    email             TEXT NOT NULL UNIQUE,
    hashed_password   TEXT NOT NULL,              -- argon2
    name              TEXT,                       -- 餐厅名，可后补
    status            TEXT NOT NULL DEFAULT 'pending',
    stripe_customer_id TEXT UNIQUE,
    created_at        BIGINT NOT NULL,
    verified_at       BIGINT
);
```

### subscriptions (crab-cloud owns, crab-auth reads)

```sql
CREATE TABLE subscriptions (
    id                 TEXT PRIMARY KEY,          -- Stripe Subscription ID
    tenant_id          TEXT NOT NULL REFERENCES tenants(id),
    status             TEXT NOT NULL DEFAULT 'active',
    plan               TEXT NOT NULL,             -- basic/pro/enterprise
    max_edge_servers   INT NOT NULL DEFAULT 1,
    max_clients        INT NOT NULL DEFAULT 5,
    features           TEXT[] NOT NULL DEFAULT '{}',
    current_period_end BIGINT,
    created_at         BIGINT NOT NULL
);
```

### email_verifications (temporary)

```sql
CREATE TABLE email_verifications (
    email      TEXT PRIMARY KEY,
    code       TEXT NOT NULL,                    -- argon2 hashed
    attempts   INT NOT NULL DEFAULT 0,
    expires_at BIGINT NOT NULL,
    created_at BIGINT NOT NULL
);
```

All migrations live in `crab-cloud/migrations/`. crab-auth migrations unchanged.

## API Endpoints

### POST /api/register

Request: `{ "email": "...", "password": "..." }`

1. Validate email format, password >= 8 chars
2. Check email not already registered (UNIQUE constraint)
3. Hash password with argon2
4. INSERT tenant (status=pending)
5. Generate 6-digit code, hash with argon2, INSERT email_verifications (5min TTL)
6. Send code via AWS SES
7. Return `{ "success": true, "message": "Verification code sent" }`

### POST /api/verify-email

Request: `{ "email": "...", "code": "..." }`

1. Lookup email_verifications, check not expired
2. Verify code (argon2), increment attempts (max 3)
3. UPDATE tenant status → verified
4. Create Stripe Customer (email)
5. Create Stripe Checkout Session (with plan selection)
6. DELETE email_verifications row
7. Return `{ "success": true, "checkout_url": "https://checkout.stripe.com/..." }`

### POST /api/resend-code

Request: `{ "email": "..." }`

1. Check tenant exists and status=pending
2. Check rate limit (5min cooldown)
3. Generate new code, replace in email_verifications
4. Send via SES
5. Return `{ "success": true }`

### POST /stripe/webhook

Stripe signature verified via `stripe_webhook_secret`.

Events handled:

| Event | Action |
|---|---|
| `checkout.session.completed` | INSERT subscription, tenant → active |
| `customer.subscription.updated` | UPDATE subscription (plan/status/quota) |
| `customer.subscription.deleted` | subscription → canceled, tenant → canceled |
| `invoice.payment_failed` | subscription → past_due, tenant → suspended |

## Stripe Integration

### Approach: Stripe Checkout (hosted) + reqwest

Use Stripe's hosted Checkout page — no custom payment UI needed. Call Stripe REST API directly with reqwest (already a workspace dependency) instead of adding `async-stripe` crate.

### Products (pre-configured in Stripe Dashboard)

| Plan | Monthly | max_edge_servers | max_clients |
|---|---|---|---|
| Basic | €X | 1 | 5 |
| Pro | €X | 3 | 10 |
| Enterprise | €X | 10 | 50 |

Plan → quota mapping hardcoded in crab-cloud (plans change very infrequently).

### Webhook Signature Verification

```rust
// Verify Stripe-Signature header
fn verify_stripe_signature(payload: &[u8], sig_header: &str, secret: &str) -> bool {
    // HMAC-SHA256 of timestamp + payload against webhook secret
}
```

## Security

| Risk | Mitigation |
|---|---|
| Brute-force registration | Same email limited to 1 attempt per 5min |
| Code brute-force | 3 wrong attempts → code invalidated |
| Duplicate registration | `tenants.email` UNIQUE constraint |
| Webhook forgery | Stripe signature verification |
| Stale verifications | Background cleanup or PG cron |
| Password weakness | Minimum 8 chars, argon2 hash |

## Module Structure

```
crab-cloud/src/
├── api/
│   ├── register.rs          # NEW: register + verify-email + resend-code
│   └── stripe_webhook.rs    # NEW: Stripe webhook handler
├── db/
│   ├── tenants.rs           # NEW: tenant CRUD
│   ├── subscriptions.rs     # NEW: subscription write
│   └── email_verifications.rs  # NEW: verification code store
├── email/
│   └── mod.rs               # NEW: SES verification email
└── stripe/
    └── mod.rs               # NEW: Checkout Session + event parsing
```

## Dependencies

```toml
# workspace Cargo.toml additions
aws-sdk-sesv2 = "1"
# No stripe crate — use reqwest directly
```

## AppState Changes

```rust
pub struct AppState {
    pub pool: PgPool,
    pub ca_store: CaStore,           // existing
    pub ses: aws_sdk_sesv2::Client,  // new
    pub stripe_secret: String,       // new: Stripe Secret Key
    pub stripe_webhook_secret: String, // new: Webhook signing secret
    pub from_email: String,          // new: SES sender address
}
```

## Config Changes

New environment variables:

| Variable | Required | Description |
|---|---|---|
| `STRIPE_SECRET_KEY` | Yes | Stripe API secret key |
| `STRIPE_WEBHOOK_SECRET` | Yes | Webhook endpoint signing secret |
| `SES_FROM_EMAIL` | Yes | Verified sender email (e.g. noreply@crab.es) |

## Frontend

Independent web page (not in red_coral). Scope for this design: **backend only**. Frontend registration page is a separate project.

## Verification

1. `cargo check -p crab-cloud` — compiles
2. `cargo clippy -p crab-cloud -- -D warnings` — zero warnings
3. Manual test: register → verify email → Stripe checkout → webhook → tenant active
4. crab-auth can authenticate the newly created tenant (password + subscription check)
