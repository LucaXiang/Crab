//! Authentication endpoints: login, forgot-password, reset-password

use axum::Json;
use axum::extract::State;
use http::HeaderMap;
use serde::Deserialize;
use shared::error::{AppError, ErrorCode};

use crate::db;
use crate::state::AppState;
use crate::util::{generate_code, hash_password, verify_password};

use super::ApiResult;

pub fn extract_client_info(headers: &HeaderMap) -> (String, String) {
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    // Caddy sets X-Real-IP; fallback to X-Forwarded-For
    let ip = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            headers
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.rsplit(',').next())
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_default();
    (ua, ip)
}

/// POST /api/tenant/login
#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(serde::Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub refresh_token: String,
    pub tenant_id: i64,
    pub status: String,
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> ApiResult<LoginResponse> {
    let email = req.email.trim().to_lowercase();
    let tenant = db::tenants::find_by_email(&state.pool, &email)
        .await
        .map_err(|e| {
            tracing::error!("DB error during login: {e}");
            AppError::new(ErrorCode::InternalError)
        })?
        .ok_or_else(|| AppError::new(ErrorCode::InvalidCredentials))?;

    if !verify_password(&req.password, &tenant.hashed_password) {
        return Err(AppError::new(ErrorCode::InvalidCredentials));
    }

    let status = shared::cloud::TenantStatus::from_db(&tenant.status);
    if !status.is_some_and(|s| s.can_login()) {
        return Err(AppError::new(ErrorCode::AccountDisabled));
    }

    let token = crate::auth::tenant_auth::create_token(tenant.id, &tenant.email, &state.jwt_secret)
        .map_err(|e| {
            tracing::error!("JWT creation failed: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    let device_id = format!("console-{}", uuid::Uuid::new_v4());
    let (user_agent, ip_address) = extract_client_info(&headers);
    let refresh_token =
        db::refresh_tokens::create(&state.pool, tenant.id, &device_id, &user_agent, &ip_address)
            .await
            .map_err(|e| {
                tracing::error!("Refresh token creation failed: {e}");
                AppError::new(ErrorCode::InternalError)
            })?;

    let now = shared::util::now_millis();
    let _ = db::audit::log(&state.pool, tenant.id, "login", None, None, now).await;

    Ok(Json(LoginResponse {
        token,
        refresh_token,
        tenant_id: tenant.id,
        status: tenant.status,
    }))
}

// ── Password reset endpoints ──

#[derive(Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

/// POST /api/tenant/forgot-password
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(req): Json<ForgotPasswordRequest>,
) -> ApiResult<serde_json::Value> {
    let email_addr = req.email.trim().to_lowercase();

    // Always return OK to prevent email enumeration
    let _tenant = match db::tenants::find_by_email(&state.pool, &email_addr).await {
        Ok(Some(t)) => t,
        _ => {
            return Ok(Json(serde_json::json!({
                "message": "If the email exists, a reset code has been sent"
            })));
        }
    };

    let code = generate_code();
    let code_hash = hash_password(&code).map_err(|_| AppError::new(ErrorCode::InternalError))?;
    let now = shared::util::now_millis();
    let expires_at = now + 5 * 60 * 1000;

    if let Err(e) = db::email_verifications::upsert(
        &state.pool,
        &email_addr,
        &code_hash,
        expires_at,
        now,
        "password_reset",
        None,
    )
    .await
    {
        tracing::error!(error = %e, "Failed to upsert password reset code, skipping email send");
        // Return OK to prevent email enumeration — do not send email if code wasn't stored
        return Ok(Json(serde_json::json!({
            "message": "If the email exists, a reset code has been sent"
        })));
    }

    let _ = state
        .email
        .send_password_reset_code(&email_addr, &code)
        .await;

    Ok(Json(serde_json::json!({
        "message": "If the email exists, a reset code has been sent"
    })))
}

#[derive(Deserialize)]
pub struct ResetPasswordRequest {
    pub email: String,
    pub code: String,
    pub new_password: String,
}

/// POST /api/tenant/reset-password
pub async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> ApiResult<serde_json::Value> {
    let email_addr = req.email.trim().to_lowercase();

    if req.new_password.len() < 8 {
        return Err(AppError::new(ErrorCode::PasswordTooShort));
    }

    let record = db::email_verifications::find(&state.pool, &email_addr, "password_reset")
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?
        .ok_or_else(|| AppError::new(ErrorCode::NotFound))?;

    let now = shared::util::now_millis();
    if now > record.expires_at {
        return Err(AppError::new(ErrorCode::VerificationCodeExpired));
    }
    if record.attempts >= 3 {
        return Err(AppError::new(ErrorCode::TooManyAttempts));
    }

    db::email_verifications::increment_attempts(&state.pool, &email_addr, "password_reset")
        .await
        .map_err(|e| {
            tracing::error!("Failed to increment attempts: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    if !verify_password(&req.code, &record.code) {
        return Err(AppError::new(ErrorCode::VerificationCodeInvalid));
    }

    let tenant = db::tenants::find_by_email(&state.pool, &email_addr)
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?
        .ok_or_else(|| AppError::new(ErrorCode::TenantNotFound))?;

    let hashed =
        hash_password(&req.new_password).map_err(|_| AppError::new(ErrorCode::InternalError))?;
    db::tenants::update_password(&state.pool, tenant.id, &hashed)
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?;

    let _ = db::email_verifications::delete(&state.pool, &email_addr, "password_reset").await;

    let _ = db::audit::log(&state.pool, tenant.id, "password_reset", None, None, now).await;

    Ok(Json(
        serde_json::json!({ "message": "Password has been reset" }),
    ))
}
