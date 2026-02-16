# Tenant Self-Registration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable restaurant owners to self-register via email + password, verify email with a 6-digit code, and pay through Stripe Checkout to activate their tenant account.

**Architecture:** Extend crab-cloud (long-running Axum service, shared PG with crab-auth) with registration endpoints, AWS SES email verification, and Stripe Checkout integration. crab-auth remains read-only for tenants/subscriptions.

**Tech Stack:** Axum, sqlx (PostgreSQL), argon2 (password hashing), AWS SES v2 (email), Stripe REST API via reqwest (payments), HMAC-SHA256 (webhook verification)

---

### Task 1: Database Migration — tenants, subscriptions, email_verifications

**Files:**
- Create: `crab-cloud/migrations/0002_tenants.up.sql`
- Create: `crab-cloud/migrations/0002_tenants.down.sql`

**Step 1: Write the up migration**

```sql
-- Tenants (crab-cloud owns, crab-auth reads)
CREATE TABLE IF NOT EXISTS tenants (
    id                TEXT PRIMARY KEY,
    email             TEXT NOT NULL UNIQUE,
    hashed_password   TEXT NOT NULL,
    name              TEXT,
    status            TEXT NOT NULL DEFAULT 'pending',
    stripe_customer_id TEXT UNIQUE,
    created_at        BIGINT NOT NULL,
    verified_at       BIGINT
);

CREATE INDEX IF NOT EXISTS idx_tenants_email ON tenants (email);
CREATE INDEX IF NOT EXISTS idx_tenants_status ON tenants (status);

-- Subscriptions (crab-cloud owns, crab-auth reads)
CREATE TABLE IF NOT EXISTS subscriptions (
    id                 TEXT PRIMARY KEY,
    tenant_id          TEXT NOT NULL REFERENCES tenants(id),
    status             TEXT NOT NULL DEFAULT 'active',
    plan               TEXT NOT NULL,
    max_edge_servers   INT NOT NULL DEFAULT 1,
    max_clients        INT NOT NULL DEFAULT 5,
    features           TEXT[] NOT NULL DEFAULT '{}',
    current_period_end BIGINT,
    created_at         BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_subscriptions_tenant ON subscriptions (tenant_id);

-- Email verification codes (temporary)
CREATE TABLE IF NOT EXISTS email_verifications (
    email      TEXT PRIMARY KEY,
    code       TEXT NOT NULL,
    attempts   INT NOT NULL DEFAULT 0,
    expires_at BIGINT NOT NULL,
    created_at BIGINT NOT NULL
);
```

**Step 2: Write the down migration**

```sql
DROP TABLE IF EXISTS email_verifications;
DROP TABLE IF EXISTS subscriptions;
DROP TABLE IF EXISTS tenants;
```

**Step 3: Run migration**

```bash
# 需要确保 DATABASE_URL 指向正确的 PG 实例
cd crab-cloud && sqlx migrate run --source migrations
```

**Step 4: Commit**

```bash
git add crab-cloud/migrations/0002_tenants.*
git commit -m "feat(crab-cloud): add tenants, subscriptions, email_verifications tables"
```

---

### Task 2: Dependencies — add argon2, aws-sdk-sesv2, reqwest, hmac, sha2

**Files:**
- Modify: `Cargo.toml` (workspace deps)
- Modify: `crab-cloud/Cargo.toml`

**Step 1: Add workspace dependencies**

Add to `Cargo.toml` under `[workspace.dependencies]`:
```toml
# Email section
aws-sdk-sesv2 = "1"

# HMAC (Stripe webhook verification) — sha2 and hex already exist
hmac = "0.12"
```

Note: `argon2`, `reqwest`, `sha2`, `hex`, `uuid`, `rand` already exist in workspace deps.

**Step 2: Add crab-cloud dependencies**

Add to `crab-cloud/Cargo.toml` under `[dependencies]`:
```toml
# Auth
argon2.workspace = true
rand.workspace = true

# Email
aws-sdk-sesv2.workspace = true

# HTTP client (Stripe API calls)
reqwest.workspace = true

# Crypto (webhook signature)
hmac.workspace = true
sha2.workspace = true
hex.workspace = true
```

**Step 3: Verify compilation**

```bash
cargo check -p crab-cloud
```

**Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock crab-cloud/Cargo.toml
git commit -m "feat(crab-cloud): add registration dependencies (argon2, ses, reqwest, hmac)"
```

---

### Task 3: DB Layer — tenants CRUD

**Files:**
- Create: `crab-cloud/src/db/tenants.rs`
- Modify: `crab-cloud/src/db/mod.rs`

**Step 1: Implement tenants DB module**

`crab-cloud/src/db/tenants.rs`:
```rust
use sqlx::PgPool;

#[derive(sqlx::FromRow)]
pub struct Tenant {
    pub id: String,
    pub email: String,
    pub hashed_password: String,
    pub name: Option<String>,
    pub status: String,
    pub stripe_customer_id: Option<String>,
    pub created_at: i64,
    pub verified_at: Option<i64>,
}

pub async fn create(
    pool: &PgPool,
    id: &str,
    email: &str,
    hashed_password: &str,
    now: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO tenants (id, email, hashed_password, status, created_at)
         VALUES ($1, $2, $3, 'pending', $4)"
    )
    .bind(id)
    .bind(email)
    .bind(hashed_password)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<Tenant>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM tenants WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await
}

pub async fn update_status(
    pool: &PgPool,
    tenant_id: &str,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tenants SET status = $1 WHERE id = $2")
        .bind(status)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_verified(
    pool: &PgPool,
    tenant_id: &str,
    now: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE tenants SET status = 'verified', verified_at = $1 WHERE id = $2"
    )
    .bind(now)
    .bind(tenant_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_stripe_customer(
    pool: &PgPool,
    tenant_id: &str,
    stripe_customer_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tenants SET stripe_customer_id = $1 WHERE id = $2")
        .bind(stripe_customer_id)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(())
}
```

**Step 2: Register module in db/mod.rs**

Add `pub mod tenants;` to `crab-cloud/src/db/mod.rs`.

**Step 3: Verify**

```bash
cargo check -p crab-cloud
```

**Step 4: Commit**

```bash
git add crab-cloud/src/db/tenants.rs crab-cloud/src/db/mod.rs
git commit -m "feat(crab-cloud): add tenants DB layer"
```

---

### Task 4: DB Layer — subscriptions + email_verifications

**Files:**
- Create: `crab-cloud/src/db/subscriptions.rs`
- Create: `crab-cloud/src/db/email_verifications.rs`
- Modify: `crab-cloud/src/db/mod.rs`

**Step 1: Implement subscriptions DB module**

`crab-cloud/src/db/subscriptions.rs`:
```rust
use sqlx::PgPool;

pub async fn create(
    pool: &PgPool,
    id: &str,
    tenant_id: &str,
    plan: &str,
    max_edge_servers: i32,
    max_clients: i32,
    current_period_end: Option<i64>,
    now: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO subscriptions (id, tenant_id, status, plan, max_edge_servers, max_clients, current_period_end, created_at)
         VALUES ($1, $2, 'active', $3, $4, $5, $6, $7)
         ON CONFLICT (id) DO UPDATE SET
            status = 'active', plan = $3, max_edge_servers = $4,
            max_clients = $5, current_period_end = $6"
    )
    .bind(id).bind(tenant_id).bind(plan)
    .bind(max_edge_servers).bind(max_clients)
    .bind(current_period_end).bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_status(
    pool: &PgPool,
    subscription_id: &str,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE subscriptions SET status = $1 WHERE id = $2")
        .bind(status).bind(subscription_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_by_stripe_sub_id(
    pool: &PgPool,
    stripe_sub_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    // Returns tenant_id
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT tenant_id FROM subscriptions WHERE id = $1"
    )
    .bind(stripe_sub_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.0))
}
```

**Step 2: Implement email_verifications DB module**

`crab-cloud/src/db/email_verifications.rs`:
```rust
use sqlx::PgPool;

pub async fn upsert(
    pool: &PgPool,
    email: &str,
    code_hash: &str,
    expires_at: i64,
    now: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO email_verifications (email, code, attempts, expires_at, created_at)
         VALUES ($1, $2, 0, $3, $4)
         ON CONFLICT (email) DO UPDATE SET
            code = $2, attempts = 0, expires_at = $3, created_at = $4"
    )
    .bind(email).bind(code_hash).bind(expires_at).bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
pub struct EmailVerification {
    pub email: String,
    pub code: String,
    pub attempts: i32,
    pub expires_at: i64,
    pub created_at: i64,
}

pub async fn find(pool: &PgPool, email: &str) -> Result<Option<EmailVerification>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM email_verifications WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await
}

pub async fn increment_attempts(pool: &PgPool, email: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE email_verifications SET attempts = attempts + 1 WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, email: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM email_verifications WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await?;
    Ok(())
}
```

**Step 3: Register modules in db/mod.rs**

Add `pub mod subscriptions;` and `pub mod email_verifications;` to `crab-cloud/src/db/mod.rs`.

**Step 4: Verify + Commit**

```bash
cargo check -p crab-cloud
git add crab-cloud/src/db/
git commit -m "feat(crab-cloud): add subscriptions and email_verifications DB layers"
```

---

### Task 5: Email Module — SES verification code sender

**Files:**
- Create: `crab-cloud/src/email/mod.rs`
- Modify: `crab-cloud/src/main.rs` (add `mod email;`)

**Step 1: Implement SES email sender**

`crab-cloud/src/email/mod.rs`:
```rust
use aws_sdk_sesv2::Client as SesClient;
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};

pub async fn send_verification_code(
    ses: &SesClient,
    from: &str,
    to: &str,
    code: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subject = Content::builder()
        .data("Tu código de verificación / Your verification code")
        .build()?;

    let body_text = format!(
        "Tu código de verificación es: {code}\n\
         Válido durante 5 minutos.\n\n\
         Your verification code is: {code}\n\
         Valid for 5 minutes."
    );

    let body = Body::builder()
        .text(Content::builder().data(body_text).build()?)
        .build();

    let message = Message::builder()
        .subject(subject)
        .body(body)
        .build();

    ses.send_email()
        .from_email_address(from)
        .destination(Destination::builder().to_addresses(to).build())
        .content(EmailContent::builder().simple(message).build())
        .send()
        .await?;

    tracing::info!(to = to, "Verification code sent");
    Ok(())
}
```

**Step 2: Add `mod email;` to main.rs**

**Step 3: Verify + Commit**

```bash
cargo check -p crab-cloud
git add crab-cloud/src/email/ crab-cloud/src/main.rs
git commit -m "feat(crab-cloud): add SES email verification code sender"
```

---

### Task 6: Stripe Module — Checkout Session + webhook verification

**Files:**
- Create: `crab-cloud/src/stripe/mod.rs`
- Modify: `crab-cloud/src/main.rs` (add `mod stripe;`)

**Step 1: Implement Stripe REST API calls via reqwest**

`crab-cloud/src/stripe/mod.rs`:
```rust
//! Stripe integration via REST API (no SDK dependency)

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Plan → quota mapping
pub struct PlanQuota {
    pub max_edge_servers: i32,
    pub max_clients: i32,
}

pub fn plan_quota(plan: &str) -> PlanQuota {
    match plan {
        "basic" => PlanQuota { max_edge_servers: 1, max_clients: 5 },
        "pro" => PlanQuota { max_edge_servers: 3, max_clients: 10 },
        "enterprise" => PlanQuota { max_edge_servers: 10, max_clients: 50 },
        _ => PlanQuota { max_edge_servers: 1, max_clients: 5 },
    }
}

/// Create a Stripe Customer
pub async fn create_customer(
    secret_key: &str,
    email: &str,
    tenant_id: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .post("https://api.stripe.com/v1/customers")
        .basic_auth(secret_key, None::<&str>)
        .form(&[
            ("email", email),
            ("metadata[tenant_id]", tenant_id),
        ])
        .send()
        .await?
        .json()
        .await?;

    resp["id"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| format!("Stripe create_customer failed: {resp}").into())
}

/// Create a Stripe Checkout Session
pub async fn create_checkout_session(
    secret_key: &str,
    customer_id: &str,
    success_url: &str,
    cancel_url: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .post("https://api.stripe.com/v1/checkout/sessions")
        .basic_auth(secret_key, None::<&str>)
        .form(&[
            ("customer", customer_id),
            ("mode", "subscription"),
            ("success_url", success_url),
            ("cancel_url", cancel_url),
            // Let Stripe show the price table — prices configured in Dashboard
            ("allow_promotion_codes", "true"),
        ])
        .send()
        .await?
        .json()
        .await?;

    resp["url"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| format!("Stripe create_checkout failed: {resp}").into())
}

/// Verify Stripe webhook signature
///
/// See: https://stripe.com/docs/webhooks/signatures
pub fn verify_webhook_signature(
    payload: &[u8],
    sig_header: &str,
    secret: &str,
) -> Result<(), &'static str> {
    // Parse "t=...,v1=..." header
    let mut timestamp = "";
    let mut signature = "";
    for part in sig_header.split(',') {
        if let Some(t) = part.strip_prefix("t=") {
            timestamp = t;
        } else if let Some(v) = part.strip_prefix("v1=") {
            signature = v;
        }
    }

    if timestamp.is_empty() || signature.is_empty() {
        return Err("Invalid Stripe-Signature header");
    }

    // Compute expected signature
    let signed_payload = format!("{timestamp}.{}", std::str::from_utf8(payload).unwrap_or(""));
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| "HMAC key error")?;
    mac.update(signed_payload.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());

    if expected != signature {
        return Err("Webhook signature mismatch");
    }

    Ok(())
}

/// Parse relevant fields from Stripe webhook event JSON
pub struct CheckoutCompleted {
    pub customer_id: String,
    pub subscription_id: String,
    pub tenant_id: String,
}

pub fn parse_checkout_completed(event: &serde_json::Value) -> Option<CheckoutCompleted> {
    let obj = event.get("data")?.get("object")?;
    Some(CheckoutCompleted {
        customer_id: obj["customer"].as_str()?.to_string(),
        subscription_id: obj["subscription"].as_str()?.to_string(),
        tenant_id: obj["metadata"]["tenant_id"].as_str()
            .or_else(|| obj["customer_details"]["metadata"]["tenant_id"].as_str())?
            .to_string(),
    })
}
```

NOTE: `create_checkout_session` 需要在 Stripe Dashboard 配置 Pricing Table 或在代码中指定 `line_items`。实际实现时根据 Stripe Dashboard 配置的 Price ID 调整。

**Step 2: Add `mod stripe;` to main.rs**

**Step 3: Verify + Commit**

```bash
cargo check -p crab-cloud
git add crab-cloud/src/stripe/ crab-cloud/src/main.rs
git commit -m "feat(crab-cloud): add Stripe checkout + webhook verification module"
```

---

### Task 7: AppState + Config — add SES, Stripe fields

**Files:**
- Modify: `crab-cloud/src/config.rs`
- Modify: `crab-cloud/src/state.rs`

**Step 1: Extend Config**

Add to `Config` struct:
```rust
pub ses_from_email: String,       // SES verified sender
pub stripe_secret_key: String,    // Stripe API secret
pub stripe_webhook_secret: String, // Webhook signing secret
pub registration_success_url: String, // Redirect after Stripe checkout
pub registration_cancel_url: String,
```

Add to `Config::from_env()`:
```rust
ses_from_email: std::env::var("SES_FROM_EMAIL")
    .unwrap_or_else(|_| "noreply@crab.es".into()),
stripe_secret_key: std::env::var("STRIPE_SECRET_KEY")
    .unwrap_or_default(),
stripe_webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET")
    .unwrap_or_default(),
registration_success_url: std::env::var("REGISTRATION_SUCCESS_URL")
    .unwrap_or_else(|_| "https://crab.es/registration/success".into()),
registration_cancel_url: std::env::var("REGISTRATION_CANCEL_URL")
    .unwrap_or_else(|_| "https://crab.es/registration/cancel".into()),
```

**Step 2: Extend AppState**

Add to `AppState` struct:
```rust
pub ses: aws_sdk_sesv2::Client,
pub stripe_secret_key: String,
pub stripe_webhook_secret: String,
pub ses_from_email: String,
pub registration_success_url: String,
pub registration_cancel_url: String,
```

Update `AppState::new()` to initialize SES client from `aws_config` and pass Stripe config.

**Step 3: Verify + Commit**

```bash
cargo check -p crab-cloud
git add crab-cloud/src/config.rs crab-cloud/src/state.rs
git commit -m "feat(crab-cloud): extend AppState with SES and Stripe config"
```

---

### Task 8: API Handler — POST /api/register

**Files:**
- Create: `crab-cloud/src/api/register.rs`
- Modify: `crab-cloud/src/api/mod.rs`

**Step 1: Implement register handler**

`crab-cloud/src/api/register.rs`:

Three handlers:
1. `register` — create tenant (pending) + send verification code
2. `verify_email` — verify code + create Stripe Checkout → return checkout_url
3. `resend_code` — resend verification code

Key logic for `register`:
```rust
pub async fn register(State(state): State<AppState>, Json(req): Json<RegisterRequest>) -> Json<Value> {
    // 1. Validate email + password (>= 8 chars)
    // 2. Check email not taken (find_by_email)
    // 3. Hash password with argon2
    // 4. Generate tenant_id (uuid v4)
    // 5. INSERT tenant (status=pending)
    // 6. Generate 6-digit code, hash with argon2
    // 7. UPSERT email_verifications (5min TTL)
    // 8. Send code via SES
    // 9. Return success
}
```

Key logic for `verify_email`:
```rust
pub async fn verify_email(State(state): State<AppState>, Json(req): Json<VerifyRequest>) -> Json<Value> {
    // 1. Find email_verification record
    // 2. Check not expired (now < expires_at)
    // 3. Check attempts < 3
    // 4. Verify code with argon2
    // 5. On success: tenant → verified, delete verification record
    // 6. Create Stripe Customer
    // 7. Set stripe_customer_id on tenant
    // 8. Create Stripe Checkout Session
    // 9. Return { checkout_url }
}
```

**Step 2: Add routes to api/mod.rs**

```rust
use axum::routing::post;

// In create_router:
let registration = Router::new()
    .route("/api/register", post(register::register))
    .route("/api/verify-email", post(register::verify_email))
    .route("/api/resend-code", post(register::resend_code));

// Merge into main router (no auth middleware)
Router::new()
    .route("/health", get(health::health_check))
    .merge(registration)
    .merge(edge)
    .with_state(state)
```

**Step 3: Verify + Commit**

```bash
cargo check -p crab-cloud
git add crab-cloud/src/api/register.rs crab-cloud/src/api/mod.rs
git commit -m "feat(crab-cloud): add registration API (register + verify-email + resend-code)"
```

---

### Task 9: API Handler — POST /stripe/webhook

**Files:**
- Create: `crab-cloud/src/api/stripe_webhook.rs`
- Modify: `crab-cloud/src/api/mod.rs`

**Step 1: Implement webhook handler**

`crab-cloud/src/api/stripe_webhook.rs`:

```rust
pub async fn handle_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    // 1. Get Stripe-Signature header
    // 2. Verify signature (stripe::verify_webhook_signature)
    // 3. Parse JSON event
    // 4. Match event type:
    //    "checkout.session.completed" → create subscription + tenant active
    //    "customer.subscription.updated" → update subscription status/plan
    //    "customer.subscription.deleted" → subscription canceled + tenant canceled
    //    "invoice.payment_failed" → subscription past_due + tenant suspended
    // 5. Return 200 OK (Stripe requires 2xx to stop retrying)
}
```

For `checkout.session.completed`:
- Parse customer_id, subscription_id from event
- Find tenant by stripe_customer_id
- Determine plan from Stripe subscription (fetch subscription details or use metadata)
- INSERT subscription with correct quota (plan_quota mapping)
- UPDATE tenant status → active

**Step 2: Add webhook route**

```rust
// In api/mod.rs create_router:
.route("/stripe/webhook", post(stripe_webhook::handle_webhook))
```

Note: webhook route must NOT have JSON content-type parsing — it needs raw body for signature verification. Use `axum::body::Bytes` instead of `Json<>`.

**Step 3: Verify + Commit**

```bash
cargo check -p crab-cloud
git add crab-cloud/src/api/stripe_webhook.rs crab-cloud/src/api/mod.rs
git commit -m "feat(crab-cloud): add Stripe webhook handler (checkout + subscription events)"
```

---

### Task 10: Integration Verification

**Step 1: Run clippy**

```bash
cargo clippy -p crab-cloud -- -D warnings
```

**Step 2: Run full workspace check**

```bash
cargo clippy --workspace -- -D warnings
cd red_coral && npx tsc --noEmit
```

**Step 3: Verify crab-auth still reads tenants correctly**

Ensure crab-auth's `db/tenants.rs` query (`SELECT id, name, hashed_password, status FROM tenants WHERE id = $1`) is compatible with the new schema. The new tenants table has all required columns plus additional ones — fully backward compatible.

**Step 4: Final commit if any fixes needed**

```bash
git add -A
git commit -m "fix(crab-cloud): address clippy warnings from registration implementation"
```

---

## Summary of All Commits

| # | Commit Message | Files |
|---|---|---|
| 1 | `feat(crab-cloud): add tenants, subscriptions, email_verifications tables` | migrations |
| 2 | `feat(crab-cloud): add registration dependencies` | Cargo.toml |
| 3 | `feat(crab-cloud): add tenants DB layer` | db/tenants.rs |
| 4 | `feat(crab-cloud): add subscriptions and email_verifications DB layers` | db/*.rs |
| 5 | `feat(crab-cloud): add SES email verification code sender` | email/mod.rs |
| 6 | `feat(crab-cloud): add Stripe checkout + webhook verification module` | stripe/mod.rs |
| 7 | `feat(crab-cloud): extend AppState with SES and Stripe config` | config.rs, state.rs |
| 8 | `feat(crab-cloud): add registration API` | api/register.rs |
| 9 | `feat(crab-cloud): add Stripe webhook handler` | api/stripe_webhook.rs |
| 10 | Quality gate: clippy + workspace check | fixes if any |

## Testing Checklist

- [ ] `cargo check -p crab-cloud` passes
- [ ] `cargo clippy -p crab-cloud -- -D warnings` zero warnings
- [ ] Manual: POST /api/register → returns success
- [ ] Manual: POST /api/verify-email with correct code → returns checkout_url
- [ ] Manual: Stripe test webhook → tenant becomes active
- [ ] crab-auth: `POST /api/server/activate` works with new tenant
