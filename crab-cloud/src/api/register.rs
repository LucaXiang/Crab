//! Registration API handlers
//!
//! POST /api/register     — create tenant (pending) + send verification code
//! POST /api/verify-email — verify code → Stripe Checkout → return checkout_url
//! POST /api/resend-code  — resend verification code

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Deserialize;
use serde_json::{Value, json};
use shared::error::{AppError, ErrorCode};

use crate::db;
use crate::state::AppState;

use sqlx;

// ── Request / Response types ──

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    /// Selected plan: "basic" or "pro" (default: "basic")
    pub plan: Option<String>,
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

use crate::util::{generate_code, hash_password, verify_password};
use shared::util::now_millis;

// ── POST /api/register ──

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let email = req.email.trim().to_lowercase();

    // Validate
    if email.is_empty() || !email.contains('@') {
        return Err(AppError::new(ErrorCode::ValidationFailed));
    }
    if req.password.len() < 8 {
        return Err(AppError::new(ErrorCode::PasswordTooShort));
    }

    let plan = req.plan.as_deref().unwrap_or("basic");
    if !matches!(plan, "basic" | "pro") {
        return Err(AppError::new(ErrorCode::ValidationFailed));
    }

    // Check email — allow re-registration if tenant is still pending
    let existing = match db::tenants::find_by_email(&state.pool, &email).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(%e, "DB error checking email");
            return Err(AppError::new(ErrorCode::InternalError));
        }
    };

    if let Some(ref tenant) = existing {
        use shared::cloud::TenantStatus;
        match TenantStatus::from_db(&tenant.status) {
            Some(TenantStatus::Pending) => {
                // Allow re-registration: update password + resend code (handled below)
            }
            Some(TenantStatus::Verified) => {
                // Already verified — tell frontend to redirect to console login
                return Ok((
                    StatusCode::OK,
                    Json(json!({
                        "status": TenantStatus::Verified.as_db(),
                        "message": "Email already verified. Please log in to continue setup."
                    })),
                ));
            }
            _ => {
                // active, suspended, canceled — truly already exists
                return Err(AppError::new(ErrorCode::AlreadyExists));
            }
        }
    }

    // Hash password
    let hashed_password = match hash_password(&req.password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!(%e, "Password hash error");
            return Err(AppError::new(ErrorCode::InternalError));
        }
    };

    let now = now_millis();

    // Generate + hash verification code before transaction
    let code = generate_code();
    let code_hash = match hash_password(&code) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!(%e, "Code hash error");
            return Err(AppError::new(ErrorCode::InternalError));
        }
    };
    let expires_at = now + 5 * 60 * 1000; // 5 minutes

    // Use existing tenant_id or generate new one
    let tenant_id = match &existing {
        Some(t) => t.id.clone(),
        None => uuid::Uuid::new_v4().to_string(),
    };

    // Insert or update tenant + verification code in a single transaction
    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(%e, "Failed to begin transaction");
            return Err(AppError::new(ErrorCode::InternalError));
        }
    };

    if existing.is_some() {
        // Update existing pending tenant's password
        if let Err(e) = sqlx::query("UPDATE tenants SET hashed_password = $1 WHERE id = $2")
            .bind(&hashed_password)
            .bind(&tenant_id)
            .execute(&mut *tx)
            .await
        {
            tracing::error!(%e, "Failed to update pending tenant");
            return Err(AppError::new(ErrorCode::InternalError));
        }
    } else {
        // Insert new tenant
        if let Err(e) = sqlx::query(
            "INSERT INTO tenants (id, email, hashed_password, status, created_at)
             VALUES ($1, $2, $3, 'pending', $4)",
        )
        .bind(&tenant_id)
        .bind(&email)
        .bind(&hashed_password)
        .bind(now)
        .execute(&mut *tx)
        .await
        {
            tracing::error!(%e, "Failed to create tenant");
            return Err(AppError::new(ErrorCode::InternalError));
        }
    }

    if let Err(e) = sqlx::query(
        "INSERT INTO email_verifications (email, code, attempts, expires_at, created_at, purpose, metadata)
         VALUES ($1, $2, 0, $3, $4, $5, $6)
         ON CONFLICT (email, purpose) DO UPDATE SET
            code = $2, attempts = 0, expires_at = $3, created_at = $4, metadata = $6",
    )
    .bind(&email)
    .bind(&code_hash)
    .bind(expires_at)
    .bind(now)
    .bind("registration")
    .bind(Some(plan))
    .execute(&mut *tx)
    .await
    {
        tracing::error!(%e, "Failed to save verification code");
        return Err(AppError::new(ErrorCode::InternalError));
    }

    if let Err(e) = tx.commit().await {
        tracing::error!(%e, "Failed to commit registration transaction");
        return Err(AppError::new(ErrorCode::InternalError));
    }

    // Send email after commit — if this fails, user can resend
    if let Err(e) = state.email.send_verification_code(&email, &code).await {
        tracing::warn!(%e, "Failed to send verification email (user can resend)");
    }

    tracing::info!(tenant_id = %tenant_id, email = %email, "Tenant registered, verification code sent");

    Ok((
        StatusCode::OK,
        Json(json!({
            "tenant_id": tenant_id,
            "message": "Verification code sent to your email"
        })),
    ))
}

// ── POST /api/verify-email ──

pub async fn verify_email(
    State(state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let email = req.email.trim().to_lowercase();
    let now = now_millis();

    // Find verification record
    let record = match db::email_verifications::find(&state.pool, &email, "registration").await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Err(AppError::new(ErrorCode::ValidationFailed));
        }
        Err(e) => {
            tracing::error!(%e, "DB error finding verification");
            return Err(AppError::new(ErrorCode::InternalError));
        }
    };

    // Check expiry
    if now > record.expires_at {
        return Err(AppError::new(ErrorCode::VerificationCodeExpired));
    }

    // Check attempts
    if record.attempts >= 3 {
        return Err(AppError::new(ErrorCode::TooManyAttempts));
    }

    // Increment attempts (must not be silently ignored — enables brute force bypass)
    if let Err(e) =
        db::email_verifications::increment_attempts(&state.pool, &email, "registration").await
    {
        tracing::error!(%e, "Failed to increment verification attempts");
        return Err(AppError::new(ErrorCode::InternalError));
    }

    // Verify code
    if !verify_password(&req.code, &record.code) {
        return Err(AppError::new(ErrorCode::VerificationCodeInvalid));
    }

    // Find tenant
    let tenant = match db::tenants::find_by_email(&state.pool, &email).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return Err(AppError::new(ErrorCode::TenantNotFound));
        }
        Err(e) => {
            tracing::error!(%e, "DB error finding tenant");
            return Err(AppError::new(ErrorCode::InternalError));
        }
    };

    // Mark tenant as verified
    if let Err(e) = db::tenants::set_verified(&state.pool, &tenant.id, now).await {
        tracing::error!(%e, "Failed to verify tenant");
        return Err(AppError::new(ErrorCode::InternalError));
    }

    // Delete verification record
    let _ = db::email_verifications::delete(&state.pool, &email, "registration").await;

    tracing::info!(tenant_id = %tenant.id, "Email verified successfully");

    Ok((
        StatusCode::OK,
        Json(json!({
            "message": "Email verified successfully"
        })),
    ))
}

// ── POST /api/resend-code ──

pub async fn resend_code(
    State(state): State<AppState>,
    Json(req): Json<ResendRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let email = req.email.trim().to_lowercase();
    let now = now_millis();

    // Find tenant — return identical response for all non-pending states to prevent email enumeration
    match db::tenants::find_by_email(&state.pool, &email).await {
        Ok(Some(t)) if t.status == shared::cloud::TenantStatus::Pending.as_db() => {}
        Ok(Some(_)) | Ok(None) => {
            return Ok((
                StatusCode::OK,
                Json(
                    json!({ "message": "If this email is registered and pending verification, a new code has been sent" }),
                ),
            ));
        }
        Err(e) => {
            tracing::error!(%e, "DB error finding tenant");
            return Err(AppError::new(ErrorCode::InternalError));
        }
    }

    // Generate new code
    let code = generate_code();
    let code_hash = match hash_password(&code) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!(%e, "Code hash error");
            return Err(AppError::new(ErrorCode::InternalError));
        }
    };

    let expires_at = now + 5 * 60 * 1000;
    if let Err(e) = db::email_verifications::upsert(
        &state.pool,
        &email,
        &code_hash,
        expires_at,
        now,
        "registration",
        None,
    )
    .await
    {
        tracing::error!(%e, "Failed to save verification code");
        return Err(AppError::new(ErrorCode::InternalError));
    }

    if let Err(e) = state.email.send_verification_code(&email, &code).await {
        tracing::error!(%e, "Failed to send verification email");
        return Err(AppError::new(ErrorCode::InternalError));
    }

    tracing::info!(email = %email, "Verification code resent");

    Ok((
        StatusCode::OK,
        Json(json!({ "message": "Verification code resent" })),
    ))
}
