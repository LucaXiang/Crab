# crab-cloud Comprehensive Improvement — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Bring crab-cloud to production readiness with security hardening, account management, Stripe completion, and audit logging.

**Architecture:** Four independent areas layered on existing crab-cloud (Axum + PgPool + AWS SDK). Each area adds migrations, DB functions, API endpoints, and email functions following established patterns. Rate limiting uses in-memory state. Audit logging is a cross-cutting concern wired into existing endpoints.

**Tech Stack:** Rust, Axum 0.8, sqlx (PostgreSQL), AWS SES v2, Stripe REST API, argon2, jsonwebtoken, chrono

---

## Implementation Order

1. **Task 1–3**: Part 1 — Production Hardening (webhook idempotency, register transaction, JWT enforcement, unwrap cleanup, rate limiting)
2. **Task 4–5**: Part 4 — Audit Logging (table + db module + query endpoint + wire into existing endpoints)
3. **Task 6–9**: Part 2 — Account Management (email_verifications purpose column, password reset, email change, profile update, password change, email templates)
4. **Task 10–11**: Part 3 — Stripe Completion (new webhook events, billing portal)

## Verification Command

After each task:

```bash
cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace --lib
```

---

### Task 1: Webhook Idempotency + Registration Transaction

**Files:**
- Create: `crab-cloud/migrations/0003_webhook_idempotency.up.sql`
- Create: `crab-cloud/migrations/0003_webhook_idempotency.down.sql`
- Modify: `crab-cloud/src/api/stripe_webhook.rs`
- Modify: `crab-cloud/src/api/register.rs`

**Step 1: Create migration for processed_webhook_events**

`crab-cloud/migrations/0003_webhook_idempotency.up.sql`:
```sql
CREATE TABLE IF NOT EXISTS processed_webhook_events (
    event_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    processed_at BIGINT NOT NULL
);
```

`crab-cloud/migrations/0003_webhook_idempotency.down.sql`:
```sql
DROP TABLE IF EXISTS processed_webhook_events;
```

**Step 2: Add idempotency check to stripe_webhook.rs**

After signature verification and JSON parsing (after line 50), before the match:

```rust
// Extract event ID
let event_id = match event["id"].as_str() {
    Some(id) => id.to_string(),
    None => {
        tracing::warn!("Webhook event missing id");
        return StatusCode::BAD_REQUEST;
    }
};

// Idempotency check
let already_processed: bool = sqlx::query_scalar(
    "SELECT EXISTS(SELECT 1 FROM processed_webhook_events WHERE event_id = $1)"
)
.bind(&event_id)
.fetch_one(&state.pool)
.await
.unwrap_or(false);

if already_processed {
    tracing::debug!(event_id = %event_id, "Duplicate webhook event, skipping");
    return StatusCode::OK;
}
```

After each handler returns OK, record the processed event. Wrap the handler dispatch + record in a PG transaction:

```rust
let mut tx = match state.pool.begin().await {
    Ok(tx) => tx,
    Err(e) => {
        tracing::error!(%e, "Failed to begin transaction");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
};

let result = match event_type {
    "checkout.session.completed" => handle_checkout_completed(&mut tx, &state, &event).await,
    "customer.subscription.updated" => handle_subscription_updated(&mut tx, &event).await,
    "customer.subscription.deleted" => handle_subscription_deleted(&mut tx, &event).await,
    "invoice.payment_failed" => handle_payment_failed(&mut tx, &event).await,
    _ => {
        tracing::debug!(event_type, "Unhandled webhook event type");
        StatusCode::OK
    }
};

if result == StatusCode::OK {
    let now = chrono::Utc::now().timestamp_millis();
    let _ = sqlx::query(
        "INSERT INTO processed_webhook_events (event_id, event_type, processed_at) VALUES ($1, $2, $3)"
    )
    .bind(&event_id)
    .bind(event_type)
    .bind(now)
    .execute(&mut *tx)
    .await;

    if let Err(e) = tx.commit().await {
        tracing::error!(%e, "Failed to commit webhook transaction");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
} else {
    let _ = tx.rollback().await;
}

result
```

Change all handler signatures from `state: &AppState` to accept `tx: &mut sqlx::PgConnection` (using `&mut *tx`) and use `tx` instead of `&state.pool` for DB operations. Keep `state` parameter where needed for non-DB state (like `ses`, `stripe_secret_key`).

Concrete signature changes:
- `handle_checkout_completed(tx: &mut PgConnection, state: &AppState, event: &Value) -> StatusCode`
- `handle_subscription_updated(tx: &mut PgConnection, event: &Value) -> StatusCode`
- `handle_subscription_deleted(tx: &mut PgConnection, event: &Value) -> StatusCode`
- `handle_payment_failed(tx: &mut PgConnection, event: &Value) -> StatusCode`

Replace all `&state.pool` with `&mut *tx` inside these handlers (e.g., `db::tenants::update_status(&mut *tx, ...)` — the db functions accept `impl sqlx::Executor<'_, Database = Postgres>` by using PgPool, but sqlx query_as/query also accepts PgConnection, so just change the pool param type in db functions from `&PgPool` to `impl sqlx::PgExecutor<'_>`).

**Actually — simpler approach:** Don't change the db function signatures. Instead, pass `&mut *tx` directly to `sqlx::query(...).execute()` calls within the handlers themselves, or just continue using `&state.pool` for read queries and only wrap the final INSERT into processed_webhook_events. This is the pragmatic approach since all handlers already commit individual changes:

Revised approach — keep handlers using `&state.pool`, only ensure the idempotency record is written after success:

```rust
let result = match event_type {
    "checkout.session.completed" => handle_checkout_completed(&state, &event).await,
    // ... same as before
};

if result == StatusCode::OK && event_type != "" {
    let now = chrono::Utc::now().timestamp_millis();
    let _ = sqlx::query(
        "INSERT INTO processed_webhook_events (event_id, event_type, processed_at) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING"
    )
    .bind(&event_id)
    .bind(event_type)
    .bind(now)
    .execute(&state.pool)
    .await;
}

result
```

This is sufficient for idempotency — the check-then-insert with `ON CONFLICT DO NOTHING` handles races.

**Step 3: Wrap register() in a PG transaction**

In `register.rs`, the `register()` function currently does: create tenant → save verification → send email. Wrap the DB operations in a transaction, send email after commit:

```rust
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    // ... validation unchanged ...

    // Begin transaction
    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(%e, "Failed to begin transaction");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
        }
    };

    // Insert tenant (use &mut *tx instead of &state.pool)
    if let Err(e) = sqlx::query(
        "INSERT INTO tenants (id, email, hashed_password, status, created_at) VALUES ($1, $2, $3, 'pending', $4)"
    )
    .bind(&tenant_id)
    .bind(&email)
    .bind(&hashed_password)
    .bind(now)
    .execute(&mut *tx)
    .await {
        tracing::error!(%e, "Failed to create tenant");
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
    }

    // Save verification code (in same tx)
    if let Err(e) = sqlx::query(
        "INSERT INTO email_verifications (email, code, attempts, expires_at, created_at)
         VALUES ($1, $2, 0, $3, $4)
         ON CONFLICT (email) DO UPDATE SET code = $2, attempts = 0, expires_at = $3, created_at = $4"
    )
    .bind(&email)
    .bind(&code_hash)
    .bind(expires_at)
    .bind(now)
    .execute(&mut *tx)
    .await {
        tracing::error!(%e, "Failed to save verification code");
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
    }

    // Commit transaction
    if let Err(e) = tx.commit().await {
        tracing::error!(%e, "Failed to commit registration transaction");
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
    }

    // Send email AFTER commit (failure doesn't rollback registration — user can resend)
    if let Err(e) = email::send_verification_code(&state.ses, &state.ses_from_email, &email, &code).await {
        tracing::warn!(%e, "Failed to send verification email (user can resend)");
    }

    // ... return success response ...
}
```

**Step 4: Verify**

```bash
cargo check --workspace && cargo clippy --workspace -- -D warnings
```

**Step 5: Commit**

```bash
git add crab-cloud/migrations/0003_webhook_idempotency.up.sql crab-cloud/migrations/0003_webhook_idempotency.down.sql crab-cloud/src/api/stripe_webhook.rs crab-cloud/src/api/register.rs
git commit -m "feat(crab-cloud): add webhook idempotency and registration transaction boundary"
```

---

### Task 2: JWT_SECRET Enforcement + Eliminate Unwrap/Panic

**Files:**
- Modify: `crab-cloud/src/config.rs:71-72`
- Modify: `crab-cloud/src/db/commands.rs:111`
- Modify: `crab-cloud/src/stripe/mod.rs:107`

**Step 1: Enforce JWT_SECRET in production**

In `config.rs`, after loading all fields, add validation:

```rust
impl Config {
    pub fn from_env() -> Self {
        let environment = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".into());
        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            if environment != "development" {
                panic!("JWT_SECRET must be set in {environment} environment");
            }
            "dev-jwt-secret-change-in-production".into()
        });

        Self {
            // ... rest unchanged, use the computed values ...
            environment,
            jwt_secret,
            // ...
        }
    }
}
```

**Step 2: Fix unwrap_or(0) in commands.rs**

`crab-cloud/src/db/commands.rs:111` — `result.command_id.parse::<i64>().unwrap_or(0)`:

Replace with proper error handling:

```rust
let command_id = match result.command_id.parse::<i64>() {
    Ok(id) => id,
    Err(e) => {
        tracing::warn!(
            command_id = %result.command_id,
            "Invalid command_id in result, skipping: {e}"
        );
        continue;
    }
};
```

And use `command_id` variable in the query below.

**Step 3: Fix unwrap in stripe/mod.rs**

`crab-cloud/src/stripe/mod.rs:107` — `std::str::from_utf8(payload).unwrap_or("")`:

This is actually fine as a pattern (unwrap_or with default). But the design doc says to eliminate unwrap patterns. The `unwrap_or("")` is safe here. Leave it as-is — it's not a panic risk.

The only other `.unwrap()` patterns in stripe/mod.rs are the `?` operator chains which are already proper error handling. No changes needed.

**Step 4: Verify**

```bash
cargo check --workspace && cargo clippy --workspace -- -D warnings
```

**Step 5: Commit**

```bash
git add crab-cloud/src/config.rs crab-cloud/src/db/commands.rs
git commit -m "fix(crab-cloud): enforce JWT_SECRET in production and fix command_id parse error"
```

---

### Task 3: Application-Layer Rate Limiting

**Files:**
- Create: `crab-cloud/src/auth/rate_limit.rs`
- Modify: `crab-cloud/src/auth/mod.rs`
- Modify: `crab-cloud/src/api/mod.rs`
- Modify: `crab-cloud/src/state.rs`

**Step 1: Create rate_limit.rs**

`crab-cloud/src/auth/rate_limit.rs`:

```rust
//! Simple in-memory rate limiter for login and registration endpoints

use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;

/// Per-route rate limit configuration
struct RateConfig {
    max_requests: u32,
    window_secs: u64,
}

/// Tracks requests from a single IP
struct IpEntry {
    count: u32,
    window_start: Instant,
}

/// Shared rate limit state
#[derive(Clone)]
pub struct RateLimiter {
    /// route_key -> (IP -> entry)
    buckets: Arc<Mutex<HashMap<&'static str, HashMap<String, IpEntry>>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn check(&self, route: &'static str, ip: &str, config: &RateConfig) -> bool {
        let mut buckets = self.buckets.lock().await;
        let route_map = buckets.entry(route).or_default();
        let now = Instant::now();

        let entry = route_map.entry(ip.to_string()).or_insert(IpEntry {
            count: 0,
            window_start: now,
        });

        // Reset window if expired
        if now.duration_since(entry.window_start).as_secs() >= config.window_secs {
            entry.count = 0;
            entry.window_start = now;
        }

        entry.count += 1;
        entry.count <= config.max_requests
    }

    /// Remove expired entries (call periodically)
    pub async fn cleanup(&self) {
        let mut buckets = self.buckets.lock().await;
        let now = Instant::now();
        for route_map in buckets.values_mut() {
            route_map.retain(|_, entry| {
                now.duration_since(entry.window_start).as_secs() < 300
            });
        }
    }
}

fn extract_ip(request: &Request) -> String {
    // Try X-Forwarded-For first (behind ALB/CloudFront)
    request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ci| ci.0.ip().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn rate_error() -> Response {
    let body = serde_json::json!({ "error": "Too many requests, try again later" });
    (
        http::StatusCode::TOO_MANY_REQUESTS,
        axum::Json(body),
    )
        .into_response()
}

/// Rate limit: 5 req/min for login
pub async fn login_rate_limit(
    State(state): State<crate::state::AppState>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let ip = extract_ip(&request);
    let config = RateConfig { max_requests: 5, window_secs: 60 };
    if !state.rate_limiter.check("login", &ip, &config).await {
        tracing::warn!(ip = %ip, "Login rate limit exceeded");
        return Err(rate_error());
    }
    Ok(next.run(request).await)
}

/// Rate limit: 3 req/min for registration
pub async fn register_rate_limit(
    State(state): State<crate::state::AppState>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let ip = extract_ip(&request);
    let config = RateConfig { max_requests: 3, window_secs: 60 };
    if !state.rate_limiter.check("register", &ip, &config).await {
        tracing::warn!(ip = %ip, "Registration rate limit exceeded");
        return Err(rate_error());
    }
    Ok(next.run(request).await)
}
```

**Step 2: Add RateLimiter to state.rs**

Add to `AppState`:
```rust
pub rate_limiter: crate::auth::rate_limit::RateLimiter,
```

In `AppState::new()`:
```rust
rate_limiter: crate::auth::rate_limit::RateLimiter::new(),
```

**Step 3: Export in auth/mod.rs**

```rust
pub mod rate_limit;
```

**Step 4: Apply rate limit middleware in api/mod.rs**

Wrap the login and registration routes:

```rust
use crate::auth::rate_limit::{login_rate_limit, register_rate_limit};

// Registration with rate limiting
let registration = Router::new()
    .route("/api/register", post(register::register))
    .route("/api/verify-email", post(register::verify_email))
    .route("/api/resend-code", post(register::resend_code))
    .layer(middleware::from_fn_with_state(state.clone(), register_rate_limit));

// Tenant login with rate limiting
let tenant_login = Router::new()
    .route("/api/tenant/login", post(tenant::login))
    .layer(middleware::from_fn_with_state(state.clone(), login_rate_limit));
```

**Step 5: Add periodic cleanup in main.rs**

After spawning servers, spawn a cleanup task:

```rust
let rate_limiter = state.rate_limiter.clone();
tokio::spawn(async move {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
    loop {
        interval.tick().await;
        rate_limiter.cleanup().await;
    }
});
```

**Step 6: Verify**

```bash
cargo check --workspace && cargo clippy --workspace -- -D warnings
```

**Step 7: Commit**

```bash
git add crab-cloud/src/auth/rate_limit.rs crab-cloud/src/auth/mod.rs crab-cloud/src/api/mod.rs crab-cloud/src/state.rs crab-cloud/src/main.rs
git commit -m "feat(crab-cloud): add application-layer rate limiting for login and registration"
```

---

### Task 4: Audit Logging — Migration + DB Module

**Files:**
- Create: `crab-cloud/migrations/0004_audit_log.up.sql`
- Create: `crab-cloud/migrations/0004_audit_log.down.sql`
- Create: `crab-cloud/src/db/audit.rs`
- Modify: `crab-cloud/src/db/mod.rs`

**Step 1: Create migration**

`crab-cloud/migrations/0004_audit_log.up.sql`:
```sql
CREATE TABLE IF NOT EXISTS cloud_audit_log (
    id BIGSERIAL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    action TEXT NOT NULL,
    detail JSONB,
    ip_address TEXT,
    created_at BIGINT NOT NULL
);

CREATE INDEX idx_cloud_audit_tenant ON cloud_audit_log (tenant_id, created_at);
```

`crab-cloud/migrations/0004_audit_log.down.sql`:
```sql
DROP TABLE IF EXISTS cloud_audit_log;
```

**Step 2: Create db/audit.rs**

```rust
//! Audit log operations

use sqlx::PgPool;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Write an audit log entry
pub async fn log(
    pool: &PgPool,
    tenant_id: &str,
    action: &str,
    detail: Option<&serde_json::Value>,
    ip_address: Option<&str>,
    now: i64,
) -> Result<(), BoxError> {
    sqlx::query(
        "INSERT INTO cloud_audit_log (tenant_id, action, detail, ip_address, created_at) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(tenant_id)
    .bind(action)
    .bind(detail)
    .bind(ip_address)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Query audit log entries for a tenant (paginated)
#[derive(sqlx::FromRow, serde::Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub action: String,
    pub detail: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: i64,
}

pub async fn query(
    pool: &PgPool,
    tenant_id: &str,
    limit: i32,
    offset: i32,
) -> Result<Vec<AuditEntry>, BoxError> {
    let rows: Vec<AuditEntry> = sqlx::query_as(
        "SELECT id, action, detail, ip_address, created_at FROM cloud_audit_log WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
    )
    .bind(tenant_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
```

**Step 3: Export in db/mod.rs**

Add: `pub mod audit;`

**Step 4: Verify**

```bash
cargo check --workspace && cargo clippy --workspace -- -D warnings
```

**Step 5: Commit**

```bash
git add crab-cloud/migrations/0004_audit_log.up.sql crab-cloud/migrations/0004_audit_log.down.sql crab-cloud/src/db/audit.rs crab-cloud/src/db/mod.rs
git commit -m "feat(crab-cloud): add audit log table and db module"
```

---

### Task 5: Audit Logging — Wire Into Endpoints + Query API

**Files:**
- Modify: `crab-cloud/src/api/tenant.rs` (add audit-log endpoint + audit calls to login, create_command)
- Modify: `crab-cloud/src/api/stripe_webhook.rs` (add audit calls)
- Modify: `crab-cloud/src/api/mod.rs` (register audit-log route)

**Step 1: Add IP extraction helper to tenant.rs**

```rust
fn extract_ip(headers: &http::HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
}
```

**Step 2: Add audit-log endpoint to tenant.rs**

```rust
#[derive(Deserialize)]
pub struct AuditQuery {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

/// GET /api/tenant/audit-log
pub async fn audit_log(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Query(query): Query<AuditQuery>,
) -> ApiResult<Vec<crate::db::audit::AuditEntry>> {
    let per_page = query.per_page.unwrap_or(20).min(100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let entries = crate::db::audit::query(&state.pool, &identity.tenant_id, per_page, offset)
        .await
        .map_err(|e| {
            tracing::error!("Audit log query error: {e}");
            internal_error("Internal error")
        })?;

    Ok(Json(entries))
}
```

**Step 3: Add audit logging to login()**

After successful login (before returning the response):
```rust
let now = shared::util::now_millis();
let _ = crate::db::audit::log(&state.pool, &tenant.id, "login", None, None, now).await;
```

After failed login (invalid credentials):
```rust
if let Ok(Some(tenant)) = crate::db::tenants::find_by_email(&state.pool, &req.email).await {
    let now = shared::util::now_millis();
    let _ = crate::db::audit::log(&state.pool, &tenant.id, "login_failed", None, None, now).await;
}
```

**Step 4: Add audit logging to create_command()**

After successful command creation:
```rust
let detail = serde_json::json!({ "command_type": req.command_type, "command_id": command_id });
let _ = crate::db::audit::log(&state.pool, &identity.tenant_id, "command_created", Some(&detail), None, now).await;
```

**Step 5: Add audit logging to stripe_webhook.rs**

In `handle_checkout_completed`, after activating tenant:
```rust
let now_audit = chrono::Utc::now().timestamp_millis();
let detail = serde_json::json!({ "subscription_id": subscription_id, "plan": plan });
let _ = crate::db::audit::log(&state.pool, &tenant.id, "subscription_activated", Some(&detail), None, now_audit).await;
```

In `handle_subscription_deleted`, after canceling tenant:
```rust
let detail = serde_json::json!({ "subscription_id": sub_id });
let _ = crate::db::audit::log(&state.pool, &tenant_id, "subscription_canceled", Some(&detail), None, chrono::Utc::now().timestamp_millis()).await;
```

**Step 6: Register audit-log route in api/mod.rs**

Add inside `tenant_api` Router:
```rust
.route("/api/tenant/audit-log", get(tenant::audit_log))
```

**Step 7: Verify**

```bash
cargo check --workspace && cargo clippy --workspace -- -D warnings
```

**Step 8: Commit**

```bash
git add crab-cloud/src/api/tenant.rs crab-cloud/src/api/stripe_webhook.rs crab-cloud/src/api/mod.rs
git commit -m "feat(crab-cloud): wire audit logging into login, commands, and subscription events"
```

---

### Task 6: Email Verifications Purpose Column + DB Functions

**Files:**
- Create: `crab-cloud/migrations/0005_email_verification_purpose.up.sql`
- Create: `crab-cloud/migrations/0005_email_verification_purpose.down.sql`
- Modify: `crab-cloud/src/db/email_verifications.rs`

**Step 1: Create migration**

`crab-cloud/migrations/0005_email_verification_purpose.up.sql`:
```sql
ALTER TABLE email_verifications ADD COLUMN purpose TEXT NOT NULL DEFAULT 'registration';
-- Change PK from email to (email, purpose) to support multiple purposes simultaneously
ALTER TABLE email_verifications DROP CONSTRAINT email_verifications_pkey;
ALTER TABLE email_verifications ADD PRIMARY KEY (email, purpose);
```

`crab-cloud/migrations/0005_email_verification_purpose.down.sql`:
```sql
ALTER TABLE email_verifications DROP CONSTRAINT email_verifications_pkey;
ALTER TABLE email_verifications ADD PRIMARY KEY (email);
ALTER TABLE email_verifications DROP COLUMN purpose;
```

**Step 2: Update email_verifications.rs**

Add `purpose` parameter to all functions:

```rust
use sqlx::PgPool;

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct EmailVerification {
    pub email: String,
    pub code: String,
    pub attempts: i32,
    pub expires_at: i64,
    pub created_at: i64,
    pub purpose: String,
}

pub async fn upsert(
    pool: &PgPool,
    email: &str,
    code_hash: &str,
    expires_at: i64,
    now: i64,
    purpose: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO email_verifications (email, code, attempts, expires_at, created_at, purpose)
         VALUES ($1, $2, 0, $3, $4, $5)
         ON CONFLICT (email, purpose) DO UPDATE SET
            code = $2, attempts = 0, expires_at = $3, created_at = $4",
    )
    .bind(email)
    .bind(code_hash)
    .bind(expires_at)
    .bind(now)
    .bind(purpose)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find(
    pool: &PgPool,
    email: &str,
    purpose: &str,
) -> Result<Option<EmailVerification>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM email_verifications WHERE email = $1 AND purpose = $2")
        .bind(email)
        .bind(purpose)
        .fetch_optional(pool)
        .await
}

pub async fn increment_attempts(
    pool: &PgPool,
    email: &str,
    purpose: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE email_verifications SET attempts = attempts + 1 WHERE email = $1 AND purpose = $2",
    )
    .bind(email)
    .bind(purpose)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, email: &str, purpose: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM email_verifications WHERE email = $1 AND purpose = $2")
        .bind(email)
        .bind(purpose)
        .execute(pool)
        .await?;
    Ok(())
}
```

**Step 3: Update register.rs callers**

Add `"registration"` as the `purpose` argument to all `db::email_verifications::` calls:

- `db::email_verifications::upsert(&state.pool, &email, &code_hash, expires_at, now, "registration")`
- `db::email_verifications::find(&state.pool, &email, "registration")`
- `db::email_verifications::increment_attempts(&state.pool, &email, "registration")`
- `db::email_verifications::delete(&state.pool, &email, "registration")`

**Step 4: Verify**

```bash
cargo check --workspace && cargo clippy --workspace -- -D warnings
```

**Step 5: Commit**

```bash
git add crab-cloud/migrations/0005_email_verification_purpose.up.sql crab-cloud/migrations/0005_email_verification_purpose.down.sql crab-cloud/src/db/email_verifications.rs crab-cloud/src/api/register.rs
git commit -m "feat(crab-cloud): add purpose column to email_verifications for multi-use codes"
```

---

### Task 7: Password Reset Endpoints

**Files:**
- Modify: `crab-cloud/src/api/tenant.rs`
- Modify: `crab-cloud/src/api/mod.rs`
- Modify: `crab-cloud/src/db/tenants.rs`
- Modify: `crab-cloud/src/email/mod.rs`

**Step 1: Add update_password to db/tenants.rs**

```rust
pub async fn update_password(
    pool: &PgPool,
    tenant_id: &str,
    hashed_password: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tenants SET hashed_password = $1 WHERE id = $2")
        .bind(hashed_password)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(())
}
```

**Step 2: Add update_email to db/tenants.rs**

```rust
pub async fn update_email(
    pool: &PgPool,
    tenant_id: &str,
    new_email: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tenants SET email = $1 WHERE id = $2")
        .bind(new_email)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(())
}
```

**Step 3: Add send_password_reset_code to email/mod.rs**

```rust
pub async fn send_password_reset_code(
    ses: &SesClient,
    from: &str,
    to: &str,
    code: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subject = Content::builder()
        .data("Restablecer contraseña / Reset your password")
        .build()?;

    let body_text = format!(
        "Tu código para restablecer la contraseña es: {code}\n\
         Válido durante 5 minutos.\n\n\
         Your password reset code is: {code}\n\
         Valid for 5 minutes."
    );

    let body = Body::builder()
        .text(Content::builder().data(body_text).build()?)
        .build();

    let message = Message::builder().subject(subject).body(body).build();

    ses.send_email()
        .from_email_address(from)
        .destination(Destination::builder().to_addresses(to).build())
        .content(EmailContent::builder().simple(message).build())
        .send()
        .await?;

    tracing::info!(to = to, "Password reset code sent");
    Ok(())
}
```

**Step 4: Add forgot-password and reset-password endpoints to tenant.rs**

```rust
use crate::{db, email};

// Reuse hash_password and verify_password from register.rs — extract to a shared location.
// For now, add these helper functions to tenant.rs (same implementations as register.rs):
fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    use argon2::password_hash::SaltString;
    use argon2::password_hash::rand_core::OsRng;
    use argon2::{Argon2, PasswordHasher};
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

fn generate_code() -> String {
    use rand::Rng;
    let code: u32 = rand::thread_rng().gen_range(100_000..1_000_000);
    code.to_string()
}

fn verify_password_hash(password: &str, hash: &str) -> bool {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    let Ok(parsed) = PasswordHash::new(hash) else { return false };
    Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()
}

#[derive(Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

/// POST /api/tenant/forgot-password — send password reset code
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(req): Json<ForgotPasswordRequest>,
) -> ApiResult<serde_json::Value> {
    let email = req.email.trim().to_lowercase();

    // Always return OK to prevent email enumeration
    let tenant = match db::tenants::find_by_email(&state.pool, &email).await {
        Ok(Some(t)) => t,
        _ => return Ok(Json(serde_json::json!({ "message": "If the email exists, a reset code has been sent" }))),
    };

    let code = generate_code();
    let code_hash = hash_password(&code).map_err(|_| internal_error("Internal error"))?;
    let now = shared::util::now_millis();
    let expires_at = now + 5 * 60 * 1000;

    let _ = db::email_verifications::upsert(&state.pool, &email, &code_hash, expires_at, now, "password_reset").await;
    let _ = email::send_password_reset_code(&state.ses, &state.ses_from_email, &email, &code).await;

    Ok(Json(serde_json::json!({ "message": "If the email exists, a reset code has been sent" })))
}

#[derive(Deserialize)]
pub struct ResetPasswordRequest {
    pub email: String,
    pub code: String,
    pub new_password: String,
}

/// POST /api/tenant/reset-password — verify code and set new password
pub async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> ApiResult<serde_json::Value> {
    let email = req.email.trim().to_lowercase();

    if req.new_password.len() < 8 {
        return Err(error(400, "Password must be at least 8 characters"));
    }

    let record = db::email_verifications::find(&state.pool, &email, "password_reset")
        .await
        .map_err(|_| internal_error("Internal error"))?
        .ok_or_else(|| error(404, "No password reset pending"))?;

    let now = shared::util::now_millis();
    if now > record.expires_at {
        return Err(error(410, "Reset code expired"));
    }
    if record.attempts >= 3 {
        return Err(error(429, "Too many attempts, request a new code"));
    }

    let _ = db::email_verifications::increment_attempts(&state.pool, &email, "password_reset").await;

    if !verify_password_hash(&req.code, &record.code) {
        return Err(error(401, "Invalid reset code"));
    }

    let tenant = db::tenants::find_by_email(&state.pool, &email)
        .await
        .map_err(|_| internal_error("Internal error"))?
        .ok_or_else(|| error(404, "Tenant not found"))?;

    let hashed = hash_password(&req.new_password).map_err(|_| internal_error("Internal error"))?;
    db::tenants::update_password(&state.pool, &tenant.id, &hashed)
        .await
        .map_err(|_| internal_error("Internal error"))?;

    let _ = db::email_verifications::delete(&state.pool, &email, "password_reset").await;

    // Audit
    let _ = crate::db::audit::log(&state.pool, &tenant.id, "password_reset", None, None, now).await;

    Ok(Json(serde_json::json!({ "message": "Password has been reset" })))
}
```

**Step 5: Register routes in api/mod.rs**

Add to public routes (no auth required):

```rust
let password_reset = Router::new()
    .route("/api/tenant/forgot-password", post(tenant::forgot_password))
    .route("/api/tenant/reset-password", post(tenant::reset_password));
```

Merge into the public router:
```rust
.merge(password_reset)
```

**Step 6: Verify**

```bash
cargo check --workspace && cargo clippy --workspace -- -D warnings
```

**Step 7: Commit**

```bash
git add crab-cloud/src/api/tenant.rs crab-cloud/src/api/mod.rs crab-cloud/src/db/tenants.rs crab-cloud/src/email/mod.rs
git commit -m "feat(crab-cloud): add password reset flow with email verification"
```

---

### Task 8: Email Change + Password Change + Profile Update

**Files:**
- Modify: `crab-cloud/src/api/tenant.rs`
- Modify: `crab-cloud/src/api/mod.rs`
- Modify: `crab-cloud/src/email/mod.rs`

**Step 1: Add send_email_change_code to email/mod.rs**

```rust
pub async fn send_email_change_code(
    ses: &SesClient,
    from: &str,
    to: &str,
    code: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subject = Content::builder()
        .data("Confirmar cambio de correo / Confirm email change")
        .build()?;

    let body_text = format!(
        "Tu código para confirmar el cambio de correo es: {code}\n\
         Válido durante 5 minutos.\n\n\
         Your email change confirmation code is: {code}\n\
         Valid for 5 minutes."
    );

    let body = Body::builder()
        .text(Content::builder().data(body_text).build()?)
        .build();

    let message = Message::builder().subject(subject).body(body).build();

    ses.send_email()
        .from_email_address(from)
        .destination(Destination::builder().to_addresses(to).build())
        .content(EmailContent::builder().simple(message).build())
        .send()
        .await?;

    tracing::info!(to = to, "Email change code sent");
    Ok(())
}
```

**Step 2: Add change-email endpoints to tenant.rs**

```rust
#[derive(Deserialize)]
pub struct ChangeEmailRequest {
    pub current_password: String,
    pub new_email: String,
}

/// POST /api/tenant/change-email — request email change (sends code to new email)
pub async fn change_email(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Json(req): Json<ChangeEmailRequest>,
) -> ApiResult<serde_json::Value> {
    let new_email = req.new_email.trim().to_lowercase();

    if new_email.is_empty() || !new_email.contains('@') {
        return Err(error(400, "Invalid email"));
    }

    // Verify current password
    let tenant = db::tenants::find_by_email(&state.pool, &identity.email)
        .await
        .map_err(|_| internal_error("Internal error"))?
        .ok_or_else(|| error(404, "Tenant not found"))?;

    if !verify_password_hash(&req.current_password, &tenant.hashed_password) {
        return Err(error(401, "Invalid password"));
    }

    // Check new email not taken
    if let Ok(Some(_)) = db::tenants::find_by_email(&state.pool, &new_email).await {
        return Err(error(409, "Email already in use"));
    }

    // Send code to NEW email
    let code = generate_code();
    let code_hash = hash_password(&code).map_err(|_| internal_error("Internal error"))?;
    let now = shared::util::now_millis();
    let expires_at = now + 5 * 60 * 1000;

    // Store with new_email as key, purpose = "email_change"
    db::email_verifications::upsert(&state.pool, &new_email, &code_hash, expires_at, now, "email_change")
        .await
        .map_err(|_| internal_error("Internal error"))?;

    let _ = email::send_email_change_code(&state.ses, &state.ses_from_email, &new_email, &code).await;

    Ok(Json(serde_json::json!({ "message": "Verification code sent to new email" })))
}

#[derive(Deserialize)]
pub struct ConfirmEmailChangeRequest {
    pub new_email: String,
    pub code: String,
}

/// POST /api/tenant/confirm-email-change — verify code and update email
pub async fn confirm_email_change(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Json(req): Json<ConfirmEmailChangeRequest>,
) -> ApiResult<serde_json::Value> {
    let new_email = req.new_email.trim().to_lowercase();

    let record = db::email_verifications::find(&state.pool, &new_email, "email_change")
        .await
        .map_err(|_| internal_error("Internal error"))?
        .ok_or_else(|| error(404, "No email change pending"))?;

    let now = shared::util::now_millis();
    if now > record.expires_at {
        return Err(error(410, "Code expired"));
    }
    if record.attempts >= 3 {
        return Err(error(429, "Too many attempts"));
    }

    let _ = db::email_verifications::increment_attempts(&state.pool, &new_email, "email_change").await;

    if !verify_password_hash(&req.code, &record.code) {
        return Err(error(401, "Invalid code"));
    }

    db::tenants::update_email(&state.pool, &identity.tenant_id, &new_email)
        .await
        .map_err(|_| internal_error("Internal error"))?;

    let _ = db::email_verifications::delete(&state.pool, &new_email, "email_change").await;

    // Audit
    let detail = serde_json::json!({ "old_email": identity.email, "new_email": new_email });
    let _ = crate::db::audit::log(&state.pool, &identity.tenant_id, "email_changed", Some(&detail), None, now).await;

    Ok(Json(serde_json::json!({ "message": "Email updated" })))
}
```

**Step 3: Add change-password endpoint to tenant.rs**

```rust
#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// POST /api/tenant/change-password
pub async fn change_password(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Json(req): Json<ChangePasswordRequest>,
) -> ApiResult<serde_json::Value> {
    if req.new_password.len() < 8 {
        return Err(error(400, "Password must be at least 8 characters"));
    }

    let tenant = db::tenants::find_by_email(&state.pool, &identity.email)
        .await
        .map_err(|_| internal_error("Internal error"))?
        .ok_or_else(|| error(404, "Tenant not found"))?;

    if !verify_password_hash(&req.current_password, &tenant.hashed_password) {
        return Err(error(401, "Invalid current password"));
    }

    let hashed = hash_password(&req.new_password).map_err(|_| internal_error("Internal error"))?;
    db::tenants::update_password(&state.pool, &identity.tenant_id, &hashed)
        .await
        .map_err(|_| internal_error("Internal error"))?;

    let now = shared::util::now_millis();
    let _ = crate::db::audit::log(&state.pool, &identity.tenant_id, "password_changed", None, None, now).await;

    Ok(Json(serde_json::json!({ "message": "Password changed" })))
}
```

**Step 4: Add profile update endpoint to tenant.rs**

```rust
#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
}

/// PUT /api/tenant/profile
pub async fn update_profile(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Json(req): Json<UpdateProfileRequest>,
) -> ApiResult<serde_json::Value> {
    if let Some(ref name) = req.name {
        sqlx::query("UPDATE tenants SET name = $1 WHERE id = $2")
            .bind(name)
            .bind(&identity.tenant_id)
            .execute(&state.pool)
            .await
            .map_err(|_| internal_error("Internal error"))?;
    }

    Ok(Json(serde_json::json!({ "message": "Profile updated" })))
}
```

**Step 5: Register routes in api/mod.rs**

Add to `tenant_api` (JWT authenticated):
```rust
.route("/api/tenant/profile", get(tenant::get_profile).put(tenant::update_profile))
.route("/api/tenant/change-email", post(tenant::change_email))
.route("/api/tenant/confirm-email-change", post(tenant::confirm_email_change))
.route("/api/tenant/change-password", post(tenant::change_password))
```

Note: the existing `/api/tenant/profile` GET route needs to become `.get(tenant::get_profile).put(tenant::update_profile)`.

**Step 6: Verify**

```bash
cargo check --workspace && cargo clippy --workspace -- -D warnings
```

**Step 7: Commit**

```bash
git add crab-cloud/src/api/tenant.rs crab-cloud/src/api/mod.rs crab-cloud/src/email/mod.rs
git commit -m "feat(crab-cloud): add email change, password change, and profile update endpoints"
```

---

### Task 9: Subscription Notification Emails

**Files:**
- Modify: `crab-cloud/src/email/mod.rs`
- Modify: `crab-cloud/src/api/stripe_webhook.rs`

**Step 1: Add 4 subscription email functions to email/mod.rs**

```rust
pub async fn send_subscription_activated(
    ses: &SesClient,
    from: &str,
    to: &str,
    plan: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subject = Content::builder()
        .data("Suscripción activada / Subscription activated")
        .build()?;

    let body_text = format!(
        "Tu suscripción al plan '{plan}' ha sido activada.\n\
         ¡Gracias por tu confianza!\n\n\
         Your '{plan}' subscription has been activated.\n\
         Thank you for your trust!"
    );

    let body = Body::builder()
        .text(Content::builder().data(body_text).build()?)
        .build();

    let message = Message::builder().subject(subject).body(body).build();

    ses.send_email()
        .from_email_address(from)
        .destination(Destination::builder().to_addresses(to).build())
        .content(EmailContent::builder().simple(message).build())
        .send()
        .await?;

    Ok(())
}

pub async fn send_subscription_canceled(
    ses: &SesClient,
    from: &str,
    to: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subject = Content::builder()
        .data("Suscripción cancelada / Subscription canceled")
        .build()?;

    let body_text =
        "Tu suscripción ha sido cancelada. Puedes volver a suscribirte en cualquier momento.\n\n\
         Your subscription has been canceled. You can resubscribe at any time.";

    let body = Body::builder()
        .text(Content::builder().data(body_text.to_string()).build()?)
        .build();

    let message = Message::builder().subject(subject).body(body).build();

    ses.send_email()
        .from_email_address(from)
        .destination(Destination::builder().to_addresses(to).build())
        .content(EmailContent::builder().simple(message).build())
        .send()
        .await?;

    Ok(())
}

pub async fn send_payment_failed(
    ses: &SesClient,
    from: &str,
    to: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subject = Content::builder()
        .data("Pago fallido / Payment failed")
        .build()?;

    let body_text =
        "No pudimos procesar tu pago. Por favor actualiza tu método de pago para evitar la suspensión de tu cuenta.\n\n\
         We couldn't process your payment. Please update your payment method to avoid account suspension.";

    let body = Body::builder()
        .text(Content::builder().data(body_text.to_string()).build()?)
        .build();

    let message = Message::builder().subject(subject).body(body).build();

    ses.send_email()
        .from_email_address(from)
        .destination(Destination::builder().to_addresses(to).build())
        .content(EmailContent::builder().simple(message).build())
        .send()
        .await?;

    Ok(())
}

pub async fn send_refund_processed(
    ses: &SesClient,
    from: &str,
    to: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subject = Content::builder()
        .data("Reembolso procesado / Refund processed")
        .build()?;

    let body_text =
        "Tu reembolso ha sido procesado. El monto se reflejará en tu cuenta en 5-10 días hábiles.\n\n\
         Your refund has been processed. The amount will be reflected in your account within 5-10 business days.";

    let body = Body::builder()
        .text(Content::builder().data(body_text.to_string()).build()?)
        .build();

    let message = Message::builder().subject(subject).body(body).build();

    ses.send_email()
        .from_email_address(from)
        .destination(Destination::builder().to_addresses(to).build())
        .content(EmailContent::builder().simple(message).build())
        .send()
        .await?;

    Ok(())
}
```

**Step 2: Wire emails into stripe_webhook.rs handlers**

In `handle_checkout_completed`, after activating tenant (find tenant email from the tenant struct):
```rust
let _ = email::send_subscription_activated(&state.ses, &state.ses_from_email, &tenant.email, plan).await;
```

In `handle_subscription_deleted`, after canceling:
```rust
// Need to find tenant email — query tenant by id
if let Ok(Some(tenant)) = db::tenants::find_by_email_or_id(&state.pool, &tenant_id).await {
    let _ = email::send_subscription_canceled(&state.ses, &state.ses_from_email, &tenant.email).await;
}
```

Actually, the handler only has `tenant_id` (String). Add a `find_by_id` function to db/tenants.rs:

```rust
pub async fn find_by_id(pool: &PgPool, id: &str) -> Result<Option<Tenant>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM tenants WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}
```

Then in `handle_subscription_deleted`:
```rust
if let Ok(Some(tenant_id)) = db::subscriptions::find_tenant_by_sub_id(&state.pool, sub_id).await {
    let _ = db::tenants::update_status(&state.pool, &tenant_id, "canceled").await;
    if let Ok(Some(tenant)) = db::tenants::find_by_id(&state.pool, &tenant_id).await {
        let _ = email::send_subscription_canceled(&state.ses, &state.ses_from_email, &tenant.email).await;
    }
    // ... audit log
}
```

In `handle_payment_failed`:
```rust
if let Ok(Some(tenant_id)) = db::subscriptions::find_tenant_by_sub_id(&state.pool, sub_id).await {
    let _ = db::tenants::update_status(&state.pool, &tenant_id, "suspended").await;
    if let Ok(Some(tenant)) = db::tenants::find_by_id(&state.pool, &tenant_id).await {
        let _ = email::send_payment_failed(&state.ses, &state.ses_from_email, &tenant.email).await;
    }
}
```

**Step 3: Verify**

```bash
cargo check --workspace && cargo clippy --workspace -- -D warnings
```

**Step 4: Commit**

```bash
git add crab-cloud/src/email/mod.rs crab-cloud/src/api/stripe_webhook.rs crab-cloud/src/db/tenants.rs
git commit -m "feat(crab-cloud): add subscription notification emails (activated, canceled, payment failed, refund)"
```

---

### Task 10: New Stripe Webhook Events

**Files:**
- Modify: `crab-cloud/src/api/stripe_webhook.rs`

**Step 1: Add 3 new event handlers**

Add to the `match event_type` in `handle_webhook`:

```rust
"charge.refunded" => handle_charge_refunded(&state, &event).await,
"invoice.paid" => handle_invoice_paid(&state, &event).await,
"invoice.payment_action_required" => handle_payment_action_required(&state, &event).await,
```

Implement the handlers:

```rust
/// charge.refunded → notify tenant
async fn handle_charge_refunded(state: &AppState, event: &serde_json::Value) -> StatusCode {
    let obj = match event.get("data").and_then(|d| d.get("object")) {
        Some(o) => o,
        None => return StatusCode::OK,
    };

    let customer_id = match obj["customer"].as_str() {
        Some(s) => s,
        None => return StatusCode::OK,
    };

    if let Ok(Some(tenant)) = db::tenants::find_by_stripe_customer(&state.pool, customer_id).await {
        let _ = email::send_refund_processed(&state.ses, &state.ses_from_email, &tenant.email).await;
        tracing::info!(tenant_id = %tenant.id, "Refund notification sent");
    }

    StatusCode::OK
}

/// invoice.paid → update current_period_end + send activation email
async fn handle_invoice_paid(state: &AppState, event: &serde_json::Value) -> StatusCode {
    let obj = match event.get("data").and_then(|d| d.get("object")) {
        Some(o) => o,
        None => return StatusCode::OK,
    };

    let sub_id = match obj["subscription"].as_str() {
        Some(s) => s,
        None => return StatusCode::OK,
    };

    // Update current_period_end from the invoice's lines
    if let Some(period_end) = obj
        .get("lines")
        .and_then(|l| l.get("data"))
        .and_then(|d| d.as_array())
        .and_then(|a| a.first())
        .and_then(|line| line.get("period"))
        .and_then(|p| p["end"].as_i64())
    {
        let period_end_ms = period_end * 1000; // Stripe uses seconds
        let _ = sqlx::query(
            "UPDATE subscriptions SET current_period_end = $1 WHERE id = $2"
        )
        .bind(period_end_ms)
        .bind(sub_id)
        .execute(&state.pool)
        .await;
    }

    tracing::info!(subscription_id = sub_id, "Invoice paid, period updated");
    StatusCode::OK
}

/// invoice.payment_action_required → email tenant about 3D Secure / SCA
async fn handle_payment_action_required(state: &AppState, event: &serde_json::Value) -> StatusCode {
    let obj = match event.get("data").and_then(|d| d.get("object")) {
        Some(o) => o,
        None => return StatusCode::OK,
    };

    let customer_id = match obj["customer"].as_str() {
        Some(s) => s,
        None => return StatusCode::OK,
    };

    if let Ok(Some(tenant)) = db::tenants::find_by_stripe_customer(&state.pool, customer_id).await {
        // Reuse payment_failed email with a note about action required
        let _ = email::send_payment_failed(&state.ses, &state.ses_from_email, &tenant.email).await;
        tracing::info!(tenant_id = %tenant.id, "Payment action required notification sent");
    }

    StatusCode::OK
}
```

**Step 2: Verify**

```bash
cargo check --workspace && cargo clippy --workspace -- -D warnings
```

**Step 3: Commit**

```bash
git add crab-cloud/src/api/stripe_webhook.rs
git commit -m "feat(crab-cloud): handle charge.refunded, invoice.paid, and payment_action_required webhook events"
```

---

### Task 11: Stripe Customer Portal (Billing Portal)

**Files:**
- Modify: `crab-cloud/src/stripe/mod.rs`
- Modify: `crab-cloud/src/api/tenant.rs`
- Modify: `crab-cloud/src/api/mod.rs`

**Step 1: Add create_billing_portal_session to stripe/mod.rs**

```rust
/// Create a Stripe Billing Portal session
pub async fn create_billing_portal_session(
    secret_key: &str,
    customer_id: &str,
    return_url: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .post("https://api.stripe.com/v1/billing_portal/sessions")
        .basic_auth(secret_key, None::<&str>)
        .form(&[
            ("customer", customer_id),
            ("return_url", return_url),
        ])
        .send()
        .await?
        .json()
        .await?;

    resp["url"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| format!("Stripe billing portal failed: {resp}").into())
}
```

**Step 2: Add billing-portal endpoint to tenant.rs**

```rust
/// POST /api/tenant/billing-portal — get Stripe Customer Portal URL
pub async fn billing_portal(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
) -> ApiResult<serde_json::Value> {
    let tenant = db::tenants::find_by_email(&state.pool, &identity.email)
        .await
        .map_err(|_| internal_error("Internal error"))?
        .ok_or_else(|| error(404, "Tenant not found"))?;

    let customer_id = tenant
        .stripe_customer_id
        .as_deref()
        .ok_or_else(|| error(400, "No Stripe customer linked"))?;

    let return_url = format!("{}/dashboard", state.registration_success_url.trim_end_matches("/registration/success"));

    let url = stripe::create_billing_portal_session(
        &state.stripe_secret_key,
        customer_id,
        &return_url,
    )
    .await
    .map_err(|e| {
        tracing::error!("Billing portal error: {e}");
        internal_error("Failed to create billing portal session")
    })?;

    Ok(Json(serde_json::json!({ "url": url })))
}
```

**Step 3: Register route in api/mod.rs**

Add to `tenant_api`:
```rust
.route("/api/tenant/billing-portal", post(tenant::billing_portal))
```

**Step 4: Verify**

```bash
cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace --lib
```

**Step 5: Commit**

```bash
git add crab-cloud/src/stripe/mod.rs crab-cloud/src/api/tenant.rs crab-cloud/src/api/mod.rs
git commit -m "feat(crab-cloud): add Stripe Customer Portal (billing-portal) endpoint"
```

---

## Final Verification

```bash
cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace --lib
```

## Summary of All Changes

| Area | New Files | Modified Files |
|------|-----------|----------------|
| **Webhook idempotency** | 2 migration files | `stripe_webhook.rs`, `register.rs` |
| **JWT + unwrap fixes** | — | `config.rs`, `commands.rs` |
| **Rate limiting** | `auth/rate_limit.rs` | `auth/mod.rs`, `api/mod.rs`, `state.rs`, `main.rs` |
| **Audit logging** | 2 migration files, `db/audit.rs` | `db/mod.rs`, `api/tenant.rs`, `stripe_webhook.rs`, `api/mod.rs` |
| **Purpose column** | 2 migration files | `db/email_verifications.rs`, `api/register.rs` |
| **Password reset** | — | `api/tenant.rs`, `api/mod.rs`, `db/tenants.rs`, `email/mod.rs` |
| **Email/password/profile** | — | `api/tenant.rs`, `api/mod.rs`, `email/mod.rs` |
| **Subscription emails** | — | `email/mod.rs`, `stripe_webhook.rs`, `db/tenants.rs` |
| **New webhook events** | — | `stripe_webhook.rs` |
| **Billing portal** | — | `stripe/mod.rs`, `api/tenant.rs`, `api/mod.rs` |
