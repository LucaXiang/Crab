//! Registration API handlers
//!
//! POST /api/register     — create tenant (pending) + send verification code
//! POST /api/verify-email — verify code → Stripe Checkout → return checkout_url
//! POST /api/resend-code  — resend verification code

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::state::AppState;
use crate::{db, email, stripe};

// ── Request / Response types ──

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub email: String,
    pub code: String,
}

#[derive(Deserialize)]
pub struct ResendRequest {
    pub email: String,
}

// ── Helpers ──

fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn generate_code() -> String {
    use rand::Rng;
    let code: u32 = rand::thread_rng().gen_range(100_000..1_000_000);
    code.to_string()
}

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    use argon2::password_hash::SaltString;
    use argon2::password_hash::rand_core::OsRng;
    use argon2::{Argon2, PasswordHasher};
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

fn error_response(status: StatusCode, msg: &str) -> (StatusCode, Json<Value>) {
    (status, Json(json!({ "error": msg })))
}

// ── POST /api/register ──

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    let email = req.email.trim().to_lowercase();

    // Validate
    if email.is_empty() || !email.contains('@') {
        return error_response(StatusCode::BAD_REQUEST, "Invalid email");
    }
    if req.password.len() < 8 {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Password must be at least 8 characters",
        );
    }

    // Check email not taken
    match db::tenants::find_by_email(&state.pool, &email).await {
        Ok(Some(_)) => {
            return error_response(StatusCode::CONFLICT, "Email already registered");
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!(%e, "DB error checking email");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
        }
    }

    // Hash password
    let hashed_password = match hash_password(&req.password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!(%e, "Password hash error");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
        }
    };

    // Generate tenant_id
    let tenant_id = uuid::Uuid::new_v4().to_string();
    let now = now_millis();

    // Insert tenant
    if let Err(e) =
        db::tenants::create(&state.pool, &tenant_id, &email, &hashed_password, now).await
    {
        tracing::error!(%e, "Failed to create tenant");
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
    }

    // Generate + send verification code
    let code = generate_code();
    let code_hash = match hash_password(&code) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!(%e, "Code hash error");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
        }
    };

    let expires_at = now + 5 * 60 * 1000; // 5 minutes
    if let Err(e) =
        db::email_verifications::upsert(&state.pool, &email, &code_hash, expires_at, now).await
    {
        tracing::error!(%e, "Failed to save verification code");
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
    }

    if let Err(e) =
        email::send_verification_code(&state.ses, &state.ses_from_email, &email, &code).await
    {
        tracing::error!(%e, "Failed to send verification email");
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to send email");
    }

    tracing::info!(tenant_id = %tenant_id, email = %email, "Tenant registered, verification code sent");

    (
        StatusCode::OK,
        Json(json!({
            "tenant_id": tenant_id,
            "message": "Verification code sent to your email"
        })),
    )
}

// ── POST /api/verify-email ──

pub async fn verify_email(
    State(state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> impl IntoResponse {
    let email = req.email.trim().to_lowercase();
    let now = now_millis();

    // Find verification record
    let record = match db::email_verifications::find(&state.pool, &email).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                "No verification pending for this email",
            );
        }
        Err(e) => {
            tracing::error!(%e, "DB error finding verification");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
        }
    };

    // Check expiry
    if now > record.expires_at {
        return error_response(StatusCode::GONE, "Verification code expired");
    }

    // Check attempts
    if record.attempts >= 3 {
        return error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many attempts, request a new code",
        );
    }

    // Increment attempts
    let _ = db::email_verifications::increment_attempts(&state.pool, &email).await;

    // Verify code
    if !verify_password(&req.code, &record.code) {
        return error_response(StatusCode::UNAUTHORIZED, "Invalid verification code");
    }

    // Find tenant
    let tenant = match db::tenants::find_by_email(&state.pool, &email).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return error_response(StatusCode::NOT_FOUND, "Tenant not found");
        }
        Err(e) => {
            tracing::error!(%e, "DB error finding tenant");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
        }
    };

    // Mark tenant as verified
    if let Err(e) = db::tenants::set_verified(&state.pool, &tenant.id, now).await {
        tracing::error!(%e, "Failed to verify tenant");
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
    }

    // Delete verification record
    let _ = db::email_verifications::delete(&state.pool, &email).await;

    // Create Stripe Customer
    let customer_id =
        match stripe::create_customer(&state.stripe_secret_key, &email, &tenant.id).await {
            Ok(id) => id,
            Err(e) => {
                tracing::error!(%e, "Failed to create Stripe customer");
                return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Payment setup failed");
            }
        };

    // Save stripe_customer_id
    if let Err(e) = db::tenants::set_stripe_customer(&state.pool, &tenant.id, &customer_id).await {
        tracing::error!(%e, "Failed to save Stripe customer ID");
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
    }

    // Create Stripe Checkout Session
    let checkout_url = match stripe::create_checkout_session(
        &state.stripe_secret_key,
        &customer_id,
        &state.registration_success_url,
        &state.registration_cancel_url,
    )
    .await
    {
        Ok(url) => url,
        Err(e) => {
            tracing::error!(%e, "Failed to create Stripe checkout session");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Payment setup failed");
        }
    };

    tracing::info!(tenant_id = %tenant.id, "Email verified, Stripe checkout created");

    (
        StatusCode::OK,
        Json(json!({
            "checkout_url": checkout_url,
            "message": "Email verified successfully"
        })),
    )
}

// ── POST /api/resend-code ──

pub async fn resend_code(
    State(state): State<AppState>,
    Json(req): Json<ResendRequest>,
) -> impl IntoResponse {
    let email = req.email.trim().to_lowercase();
    let now = now_millis();

    // Find tenant
    match db::tenants::find_by_email(&state.pool, &email).await {
        Ok(Some(t)) if t.status == "pending" => {}
        Ok(Some(_)) => {
            return error_response(StatusCode::CONFLICT, "Email already verified");
        }
        Ok(None) => {
            return error_response(StatusCode::NOT_FOUND, "Email not registered");
        }
        Err(e) => {
            tracing::error!(%e, "DB error finding tenant");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
        }
    }

    // Generate new code
    let code = generate_code();
    let code_hash = match hash_password(&code) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!(%e, "Code hash error");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
        }
    };

    let expires_at = now + 5 * 60 * 1000;
    if let Err(e) =
        db::email_verifications::upsert(&state.pool, &email, &code_hash, expires_at, now).await
    {
        tracing::error!(%e, "Failed to save verification code");
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
    }

    if let Err(e) =
        email::send_verification_code(&state.ses, &state.ses_from_email, &email, &code).await
    {
        tracing::error!(%e, "Failed to send verification email");
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to send email");
    }

    tracing::info!(email = %email, "Verification code resent");

    (
        StatusCode::OK,
        Json(json!({ "message": "Verification code resent" })),
    )
}
