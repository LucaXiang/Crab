# Security Fixes + AWS Architecture Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix 8 code security issues and add NLB + Secrets Manager to CloudFormation so edge-servers can reach the mTLS endpoint.

**Architecture:** Extract shared helpers to `util.rs`, fix security flaws in auth/webhook/config, add NLB (TCP 8443 pass-through) to CloudFormation, support PEM content via env vars for containerized mTLS.

**Tech Stack:** Rust (axum, sqlx, argon2, hmac), AWS CloudFormation (NLB, Secrets Manager, ECS)

---

### Task 1: Extract shared helpers to `util.rs`

**Files:**
- Create: `crab-cloud/src/util.rs`
- Modify: `crab-cloud/src/main.rs`
- Modify: `crab-cloud/src/api/register.rs`
- Modify: `crab-cloud/src/api/tenant.rs`

**Step 1: Create `crab-cloud/src/util.rs`**

```rust
//! Shared utility functions for crab-cloud

pub fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn generate_code() -> String {
    use rand::Rng;
    let code: u32 = rand::thread_rng().gen_range(100_000..1_000_000);
    code.to_string()
}

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    use argon2::password_hash::SaltString;
    use argon2::password_hash::rand_core::OsRng;
    use argon2::{Argon2, PasswordHasher};
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}
```

**Step 2: Add `mod util;` to `crab-cloud/src/main.rs`**

Add `mod util;` after `mod stripe;` (line 16).

**Step 3: Update `register.rs`**

- Delete: `fn now_millis()` (lines 40-42), `fn generate_code()` (lines 44-48), `fn hash_password()` (lines 50-57), `fn verify_password()` (lines 59-67)
- Replace all calls with `crate::util::now_millis()`, `crate::util::generate_code()`, `crate::util::hash_password()`, `crate::util::verify_password()`
- Keep `fn error_response()` — it's specific to register.rs return type

**Step 4: Update `tenant.rs`**

- Delete: `fn hash_password()` (lines 563-570), `fn generate_code()` (lines 572-576), `fn verify_password_hash()` (lines 578-586)
- Replace all calls: `hash_password(` → `crate::util::hash_password(`, `generate_code()` → `crate::util::generate_code()`, `verify_password_hash(` → `crate::util::verify_password(`
- In `login()` (line 43-51): replace inline argon2 verification with `crate::util::verify_password(&req.password, &tenant.hashed_password)`

**Step 5: Verify**

```bash
cargo check -p crab-cloud
cargo clippy -p crab-cloud -- -D warnings
```

**Step 6: Commit**

```bash
git add crab-cloud/src/util.rs crab-cloud/src/main.rs crab-cloud/src/api/register.rs crab-cloud/src/api/tenant.rs
git commit -m "refactor(crab-cloud): extract shared helpers to util.rs"
```

---

### Task 2: Fix X-Forwarded-For IP extraction

**Files:**
- Modify: `crab-cloud/src/auth/rate_limit.rs:74-94`

**Step 1: Fix `extract_ip`**

Replace the current implementation. ALB appends the real client IP as the **last** entry in X-Forwarded-For. Current code takes the first (attacker-controlled).

```rust
/// Extract client IP: X-Forwarded-For last entry (ALB appends real IP), then peer address.
fn extract_ip(request: &Request) -> String {
    if let Some(forwarded) = request.headers().get("x-forwarded-for")
        && let Ok(val) = forwarded.to_str()
    {
        // ALB appends real client IP as the last entry
        if let Some(last) = val.rsplit(',').next() {
            let ip = last.trim();
            if !ip.is_empty() {
                return ip.to_owned();
            }
        }
    }

    // Fallback: peer address from extensions (ConnectInfo)
    request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}
```

Key change: `val.split(',').next()` → `val.rsplit(',').next()` and comment updated.

**Step 2: Verify**

```bash
cargo check -p crab-cloud
```

**Step 3: Commit**

```bash
git add crab-cloud/src/auth/rate_limit.rs
git commit -m "fix(crab-cloud): extract real client IP from last X-Forwarded-For entry (ALB)"
```

---

### Task 3: Enforce secrets in production

**Files:**
- Modify: `crab-cloud/src/config.rs`

**Step 1: Add `require_secret` helper and use it**

Add a helper function at the top of `impl Config`:

```rust
impl Config {
    fn require_secret(name: &str, environment: &str) -> String {
        let val = std::env::var(name).unwrap_or_else(|_| {
            if environment != "development" {
                panic!("{name} must be set in {environment} environment");
            }
            format!("dev-{name}-not-for-production")
        });
        if val.is_empty() && environment != "development" {
            panic!("{name} must not be empty in {environment} environment");
        }
        val
    }

    pub fn from_env() -> Self {
        let environment = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".into());

        Self {
            // ... existing fields ...
            stripe_secret_key: Self::require_secret("STRIPE_SECRET_KEY", &environment),
            stripe_webhook_secret: Self::require_secret("STRIPE_WEBHOOK_SECRET", &environment),
            jwt_secret: Self::require_secret("JWT_SECRET", &environment),
            // ... rest ...
        }
    }
}
```

This replaces:
- `stripe_secret_key: std::env::var("STRIPE_SECRET_KEY").unwrap_or_default()` (line 69)
- `stripe_webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_default()` (line 70)
- The custom jwt_secret block (lines 43-48)

**Step 2: Verify**

```bash
cargo check -p crab-cloud
```

**Step 3: Commit**

```bash
git add crab-cloud/src/config.rs
git commit -m "fix(crab-cloud): enforce non-empty secrets in production environment"
```

---

### Task 4: Fix webhook idempotency (INSERT-first)

**Files:**
- Modify: `crab-cloud/src/api/stripe_webhook.rs:54-81`

**Step 1: Replace SELECT EXISTS + INSERT with INSERT-first**

Replace the current idempotency check block (lines 54-81) with:

```rust
    // 4. Idempotency: INSERT first, check rows_affected
    let event_id = match event["id"].as_str() {
        Some(id) => id,
        None => {
            tracing::warn!("Webhook event missing id");
            return StatusCode::BAD_REQUEST;
        }
    };

    let now = chrono::Utc::now().timestamp_millis();
    let insert_result = sqlx::query(
        "INSERT INTO processed_webhook_events (event_id, event_type, processed_at)
         VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(event_id)
    .bind(event_type)
    .bind(now)
    .execute(&state.pool)
    .await;

    match insert_result {
        Ok(r) if r.rows_affected() == 0 => {
            tracing::info!(event_id = event_id, "Duplicate webhook event, skipping");
            return StatusCode::OK;
        }
        Err(e) => {
            tracing::error!(%e, "DB error recording webhook event");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
        Ok(_) => {} // New event, proceed
    }
```

**Step 2: Remove the post-processing INSERT block** (lines 98-114)

Delete the "6. Record processed event (only on success)" block entirely, since we now INSERT before processing.

**Step 3: Verify**

```bash
cargo check -p crab-cloud
```

**Step 4: Commit**

```bash
git add crab-cloud/src/api/stripe_webhook.rs
git commit -m "fix(crab-cloud): webhook idempotency INSERT-first to eliminate TOCTOU race"
```

---

### Task 5: Handle increment_attempts errors

**Files:**
- Modify: `crab-cloud/src/api/register.rs` (verify_email function, ~line 231)
- Modify: `crab-cloud/src/api/tenant.rs` (confirm_email_change ~line 468, reset_password ~line 671)

**Step 1: Fix register.rs::verify_email**

Replace:
```rust
let _ = db::email_verifications::increment_attempts(&state.pool, &email, "registration").await;
```

With:
```rust
if let Err(e) = db::email_verifications::increment_attempts(&state.pool, &email, "registration").await {
    tracing::error!(%e, "Failed to increment verification attempts");
    return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
}
```

**Step 2: Fix tenant.rs::confirm_email_change**

Replace (line ~468):
```rust
let _ = db::email_verifications::increment_attempts(&state.pool, &new_email, "email_change").await;
```

With:
```rust
db::email_verifications::increment_attempts(&state.pool, &new_email, "email_change")
    .await
    .map_err(|e| {
        tracing::error!("Failed to increment attempts: {e}");
        internal_error("Internal error")
    })?;
```

**Step 3: Fix tenant.rs::reset_password**

Replace (line ~671):
```rust
let _ = db::email_verifications::increment_attempts(&state.pool, &email_addr, "password_reset").await;
```

With:
```rust
db::email_verifications::increment_attempts(&state.pool, &email_addr, "password_reset")
    .await
    .map_err(|e| {
        tracing::error!("Failed to increment attempts: {e}");
        internal_error("Internal error")
    })?;
```

**Step 4: Verify**

```bash
cargo check -p crab-cloud
```

**Step 5: Commit**

```bash
git add crab-cloud/src/api/register.rs crab-cloud/src/api/tenant.rs
git commit -m "fix(crab-cloud): handle increment_attempts errors to prevent brute force bypass"
```

---

### Task 6: Bind tenant_id in email change verification

**Files:**
- Modify: `crab-cloud/src/db/email_verifications.rs`
- Create: `crab-cloud/migrations/0006_email_verification_metadata.up.sql`
- Create: `crab-cloud/migrations/0006_email_verification_metadata.down.sql`
- Modify: `crab-cloud/src/api/tenant.rs` (change_email + confirm_email_change)

**Step 1: Create migration**

`0006_email_verification_metadata.up.sql`:
```sql
ALTER TABLE email_verifications ADD COLUMN metadata TEXT;
```

`0006_email_verification_metadata.down.sql`:
```sql
ALTER TABLE email_verifications DROP COLUMN metadata;
```

**Step 2: Update `EmailVerification` struct**

In `crab-cloud/src/db/email_verifications.rs`, add field:
```rust
pub struct EmailVerification {
    pub email: String,
    pub code: String,
    pub attempts: i32,
    pub expires_at: i64,
    pub created_at: i64,
    pub purpose: String,
    pub metadata: Option<String>,
}
```

**Step 3: Update `upsert` to accept optional metadata**

```rust
pub async fn upsert(
    pool: &PgPool,
    email: &str,
    code_hash: &str,
    expires_at: i64,
    now: i64,
    purpose: &str,
    metadata: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO email_verifications (email, code, attempts, expires_at, created_at, purpose, metadata)
         VALUES ($1, $2, 0, $3, $4, $5, $6)
         ON CONFLICT (email, purpose) DO UPDATE SET
            code = $2, attempts = 0, expires_at = $3, created_at = $4, metadata = $6",
    )
    .bind(email)
    .bind(code_hash)
    .bind(expires_at)
    .bind(now)
    .bind(purpose)
    .bind(metadata)
    .execute(pool)
    .await?;
    Ok(())
}
```

**Step 4: Update all `upsert` call sites to pass `None` for metadata**

- `register.rs` (inline SQL in transaction): add `, metadata` to INSERT column list and bind `None::<&str>`
- `register.rs::resend_code`: add `None` parameter to `upsert()` call
- `tenant.rs::forgot_password`: add `None` parameter to `upsert()` call

**Step 5: Update `tenant.rs::change_email` to store tenant_id in metadata**

```rust
let metadata = serde_json::json!({
    "tenant_id": identity.tenant_id,
    "old_email": identity.email,
}).to_string();

db::email_verifications::upsert(
    &state.pool,
    &new_email,
    &code_hash,
    expires_at,
    now,
    "email_change",
    Some(&metadata),
)
.await
.map_err(|_| internal_error("Internal error"))?;
```

**Step 6: Update `tenant.rs::confirm_email_change` to verify tenant_id**

After finding the verification record, add:
```rust
// Verify this email change belongs to the current tenant
if let Some(ref meta_str) = record.metadata {
    if let Ok(meta) = serde_json::from_str::<serde_json::Value>(meta_str) {
        if meta["tenant_id"].as_str() != Some(&identity.tenant_id) {
            return Err(error(403, "Email change does not belong to this account"));
        }
    }
} else {
    return Err(error(400, "Invalid email change request"));
}
```

**Step 7: Verify**

```bash
cargo check -p crab-cloud
```

**Step 8: Commit**

```bash
git add crab-cloud/migrations/0006_email_verification_metadata.up.sql \
       crab-cloud/migrations/0006_email_verification_metadata.down.sql \
       crab-cloud/src/db/email_verifications.rs \
       crab-cloud/src/api/tenant.rs \
       crab-cloud/src/api/register.rs
git commit -m "fix(crab-cloud): bind tenant_id in email change verification to prevent cross-tenant attacks"
```

---

### Task 7: JWT endpoints use find_by_id instead of find_by_email

**Files:**
- Modify: `crab-cloud/src/api/tenant.rs`

**Step 1: Fix `billing_portal`** (line 354)

Replace:
```rust
let tenant = db::tenants::find_by_email(&state.pool, &identity.email)
```
With:
```rust
let tenant = db::tenants::find_by_id(&state.pool, &identity.tenant_id)
```

**Step 2: Fix `change_email`** (line 404)

Replace:
```rust
let tenant = db::tenants::find_by_email(&state.pool, &identity.email)
```
With:
```rust
let tenant = db::tenants::find_by_id(&state.pool, &identity.tenant_id)
```

**Step 3: Fix `change_password`** (line 511)

Replace:
```rust
let tenant = db::tenants::find_by_email(&state.pool, &identity.email)
```
With:
```rust
let tenant = db::tenants::find_by_id(&state.pool, &identity.tenant_id)
```

**Step 4: Verify**

```bash
cargo check -p crab-cloud
```

**Step 5: Commit**

```bash
git add crab-cloud/src/api/tenant.rs
git commit -m "fix(crab-cloud): use find_by_id for JWT-authenticated endpoints"
```

---

### Task 8: Webhook signature timestamp validation

**Files:**
- Modify: `crab-cloud/src/stripe/mod.rs:110-140`

**Step 1: Add timestamp tolerance check**

Update `verify_webhook_signature` to validate timestamp:

```rust
pub fn verify_webhook_signature(
    payload: &[u8],
    sig_header: &str,
    secret: &str,
) -> Result<(), &'static str> {
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

    // Check timestamp tolerance (5 minutes)
    let ts: i64 = timestamp.parse().map_err(|_| "Invalid timestamp")?;
    let now = chrono::Utc::now().timestamp();
    if (now - ts).abs() > 300 {
        return Err("Webhook timestamp too old or too far in the future");
    }

    let signed_payload = format!("{timestamp}.{}", std::str::from_utf8(payload).unwrap_or(""));
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|_| "HMAC key error")?;
    mac.update(signed_payload.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());

    if expected != signature {
        return Err("Webhook signature mismatch");
    }

    Ok(())
}
```

Key addition: parse `t=` as `i64`, check `|now - t| <= 300` seconds.

**Step 2: Verify**

```bash
cargo check -p crab-cloud
```

**Step 3: Commit**

```bash
git add crab-cloud/src/stripe/mod.rs
git commit -m "fix(crab-cloud): validate webhook signature timestamp to prevent replay attacks"
```

---

### Task 9: CloudFormation — NLB for mTLS + JWT Secret

**Files:**
- Modify: `deploy/cloudformation.yml`
- Modify: `deploy/setup-secrets.sh`

**Step 1: Add JwtSecret to Secrets Manager section** (after StripeWebhookSecretSecret, ~line 79)

```yaml
  JwtSecret:
    Type: AWS::SecretsManager::Secret
    Properties:
      Name: !Sub crab/${Environment}/jwt-secret
      Description: JWT signing secret for tenant authentication
      Tags:
        - Key: Project
          Value: crab
        - Key: Environment
          Value: !Ref Environment
```

**Step 2: Add NLB resources** (after WAFAssociation, ~line 491)

```yaml
  # ════════════════════════════════════════════
  # NLB — TCP pass-through for mTLS (edge-server sync)
  # ════════════════════════════════════════════

  NLB:
    Type: AWS::ElasticLoadBalancingV2::LoadBalancer
    Properties:
      Name: !Sub crab-mtls-${Environment}
      Scheme: internet-facing
      Type: network
      Subnets:
        - !Ref PublicSubnet1
        - !Ref PublicSubnet2
      Tags:
        - Key: Project
          Value: crab

  NLBTargetGroup:
    Type: AWS::ElasticLoadBalancingV2::TargetGroup
    Properties:
      VpcId: !Ref VPC
      Port: 8443
      Protocol: TCP
      TargetType: ip
      HealthCheckProtocol: TCP
      HealthCheckPort: '8443'
      HealthyThresholdCount: 3
      UnhealthyThresholdCount: 3
      HealthCheckIntervalSeconds: 30
      Tags:
        - Key: Project
          Value: crab

  NLBListener:
    Type: AWS::ElasticLoadBalancingV2::Listener
    Properties:
      LoadBalancerArn: !Ref NLB
      Port: 8443
      Protocol: TCP
      DefaultActions:
        - Type: forward
          TargetGroupArn: !Ref NLBTargetGroup
```

**Step 3: Update ECSSecurityGroup** (~line 281)

Add ingress rule for 8443 from anywhere (NLB doesn't use security groups, it preserves source IP):

```yaml
  ECSSecurityGroup:
    Type: AWS::EC2::SecurityGroup
    Properties:
      GroupDescription: ECS tasks - allow from ALB (8080) and NLB/internet (8443 mTLS)
      VpcId: !Ref VPC
      SecurityGroupIngress:
        - IpProtocol: tcp
          FromPort: 8080
          ToPort: 8080
          SourceSecurityGroupId: !Ref ALBSecurityGroup
        - IpProtocol: tcp
          FromPort: 8443
          ToPort: 8443
          CidrIp: 0.0.0.0/0
      Tags:
        - Key: Project
          Value: crab
```

**Step 4: Update CrabCloudTaskDef** (~line 871)

Add port 8443 mapping and JWT_SECRET secret:

In `PortMappings`:
```yaml
          PortMappings:
            - ContainerPort: 8080
              Protocol: tcp
            - ContainerPort: 8443
              Protocol: tcp
```

In `Secrets` section:
```yaml
          Secrets:
            - Name: DATABASE_URL
              ValueFrom: !Ref DatabaseUrlSecret
            - Name: STRIPE_SECRET_KEY
              ValueFrom: !Ref StripeSecretKeySecret
            - Name: STRIPE_WEBHOOK_SECRET
              ValueFrom: !Ref StripeWebhookSecretSecret
            - Name: JWT_SECRET
              ValueFrom: !Ref JwtSecret
```

**Step 5: Update CrabCloudExecutionRole** (~line 867)

Add JwtSecret to ReadSecrets policy:

```yaml
              Resource:
                - !Ref DatabaseUrlSecret
                - !Ref StripeSecretKeySecret
                - !Ref StripeWebhookSecretSecret
                - !Ref JwtSecret
```

**Step 6: Update CrabCloudService** to register with NLB target group too (~line 1015)

```yaml
      LoadBalancers:
        - ContainerName: crab-cloud
          ContainerPort: 8080
          TargetGroupArn: !Ref ALBTargetGroup
        - ContainerName: crab-cloud
          ContainerPort: 8443
          TargetGroupArn: !Ref NLBTargetGroup
```

**Step 7: Add Outputs** (at end of Outputs section)

```yaml
  NLBDnsName:
    Description: NLB DNS for mTLS (CNAME mtls.redcoral.app here)
    Value: !GetAtt NLB.DNSName

  JwtSecretArn:
    Description: Secrets Manager ARN for JWT_SECRET
    Value: !Ref JwtSecret
```

**Step 8: Update setup-secrets.sh**

Add after the stripe-webhook-secret line:
```bash
set_secret "jwt-secret" "JWT signing secret for tenant authentication (random 64+ char string)"
```

**Step 9: Commit**

```bash
git add deploy/cloudformation.yml deploy/setup-secrets.sh
git commit -m "infra: add NLB for mTLS edge sync + JWT secret to Secrets Manager"
```

---

### Task 10: Support PEM content from environment variables

**Files:**
- Modify: `crab-cloud/src/config.rs`
- Modify: `crab-cloud/src/main.rs` (build_mtls_config function)

**Step 1: Add PEM env vars to Config**

Add 3 new fields to Config struct:
```rust
pub struct Config {
    // ... existing fields ...
    /// Root CA PEM content (from env, overrides root_ca_path)
    pub root_ca_pem: Option<String>,
    /// Server cert PEM content (from env, overrides server_cert_path)
    pub server_cert_pem: Option<String>,
    /// Server key PEM content (from env, overrides server_key_path)
    pub server_key_pem: Option<String>,
}
```

In `from_env()`:
```rust
root_ca_pem: std::env::var("ROOT_CA_PEM").ok().filter(|s| !s.is_empty()),
server_cert_pem: std::env::var("SERVER_CERT_PEM").ok().filter(|s| !s.is_empty()),
server_key_pem: std::env::var("SERVER_KEY_PEM").ok().filter(|s| !s.is_empty()),
```

**Step 2: Update `build_mtls_config` in `main.rs`**

Replace the file-reading section to check env vars first:

```rust
fn build_mtls_config(config: &Config) -> Result<axum_server::tls_rustls::RustlsConfig, BoxError> {
    // Read cert/key/CA from env vars (PEM content) or files
    let cert_pem = match &config.server_cert_pem {
        Some(pem) => pem.as_bytes().to_vec(),
        None => {
            let path = std::path::PathBuf::from(&config.server_cert_path);
            if !path.exists() {
                return Err(format!("Server cert not found: {}", config.server_cert_path).into());
            }
            std::fs::read(&path)?
        }
    };

    let key_pem = match &config.server_key_pem {
        Some(pem) => pem.as_bytes().to_vec(),
        None => {
            let path = std::path::PathBuf::from(&config.server_key_path);
            if !path.exists() {
                return Err(format!("Server key not found: {}", config.server_key_path).into());
            }
            std::fs::read(&path)?
        }
    };

    let ca_pem = match &config.root_ca_pem {
        Some(pem) => pem.as_bytes().to_vec(),
        None => {
            let path = std::path::PathBuf::from(&config.root_ca_path);
            if !path.exists() {
                return Err(format!("Root CA not found: {}", config.root_ca_path).into());
            }
            std::fs::read(&path)?
        }
    };

    // Parse server certs
    let certs: Vec<rustls_pki_types::CertificateDer<'static>> =
        rustls_pemfile::certs(&mut &cert_pem[..]).collect::<Result<Vec<_>, _>>()?;

    // Parse server key
    let key = rustls_pemfile::private_key(&mut &key_pem[..])?
        .ok_or("No private key found in server key PEM")?;

    // Parse Root CA for client verification
    let mut root_store = rustls::RootCertStore::empty();
    let ca_certs: Vec<rustls_pki_types::CertificateDer<'static>> =
        rustls_pemfile::certs(&mut &ca_pem[..]).collect::<Result<Vec<_>, _>>()?;
    for cert in ca_certs {
        root_store.add(cert)?;
    }

    // Build client cert verifier (mandatory)
    let client_verifier =
        rustls::server::WebPkiClientVerifier::builder(std::sync::Arc::new(root_store)).build()?;

    let mut tls_config = rustls::ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(certs, key)?;

    tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(axum_server::tls_rustls::RustlsConfig::from_config(
        std::sync::Arc::new(tls_config),
    ))
}
```

**Step 3: Verify**

```bash
cargo check -p crab-cloud
cargo clippy -p crab-cloud -- -D warnings
```

**Step 4: Commit**

```bash
git add crab-cloud/src/config.rs crab-cloud/src/main.rs
git commit -m "feat(crab-cloud): support mTLS PEM content from env vars for containerized deployment"
```

---

### Task 11: Final verification

**Step 1: Run full workspace checks**

```bash
cargo clippy --workspace -- -D warnings
cargo test --workspace --lib
cd red_coral && npx tsc --noEmit
```

All must pass with zero warnings and zero errors.

**Step 2: Verify git log**

```bash
git log --oneline -12
```

Should show ~10 clean commits from this implementation.
