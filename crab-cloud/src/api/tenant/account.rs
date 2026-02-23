//! Account management endpoints: profile, email change, password change

use axum::{Extension, Json, extract::State};
use serde::Deserialize;
use shared::error::{AppError, ErrorCode};

use crate::auth::tenant_auth::TenantIdentity;
use crate::db::{self, tenant_queries};
use crate::state::AppState;
use crate::util::{generate_code, hash_password, verify_password};

use super::ApiResult;

/// GET /api/tenant/profile
pub async fn get_profile(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
) -> ApiResult<serde_json::Value> {
    let profile = tenant_queries::get_profile(&state.pool, &identity.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Profile query error: {e}");
            AppError::new(ErrorCode::InternalError)
        })?
        .ok_or_else(|| AppError::new(ErrorCode::TenantNotFound))?;

    let subscription = tenant_queries::get_subscription(&state.pool, &identity.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Subscription query error: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    let p12 = db::p12::get_p12_info(&state.pool, &identity.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("P12 query error: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    Ok(Json(serde_json::json!({
        "profile": profile,
        "subscription": subscription,
        "p12": p12,
    })))
}

/// PUT /api/tenant/profile
#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
}

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
            .map_err(|_| AppError::new(ErrorCode::InternalError))?;
    }
    Ok(Json(serde_json::json!({ "message": "Profile updated" })))
}

/// POST /api/tenant/change-email
#[derive(Deserialize)]
pub struct ChangeEmailRequest {
    pub current_password: String,
    pub new_email: String,
}

pub async fn change_email(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Json(req): Json<ChangeEmailRequest>,
) -> ApiResult<serde_json::Value> {
    let new_email = req.new_email.trim().to_lowercase();
    if new_email.is_empty() || !new_email.contains('@') {
        return Err(AppError::new(ErrorCode::ValidationFailed));
    }

    let tenant = db::tenants::find_by_id(&state.pool, &identity.tenant_id)
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?
        .ok_or_else(|| AppError::new(ErrorCode::TenantNotFound))?;

    if !verify_password(&req.current_password, &tenant.hashed_password) {
        return Err(AppError::new(ErrorCode::InvalidCredentials));
    }

    if let Ok(Some(_)) = db::tenants::find_by_email(&state.pool, &new_email).await {
        return Err(AppError::new(ErrorCode::AlreadyExists));
    }

    let code = generate_code();
    let code_hash = hash_password(&code).map_err(|_| AppError::new(ErrorCode::InternalError))?;
    let now = shared::util::now_millis();
    let expires_at = now + 5 * 60 * 1000;

    let metadata = serde_json::json!({
        "tenant_id": identity.tenant_id,
        "old_email": tenant.email,
    })
    .to_string();

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
    .map_err(|_| AppError::new(ErrorCode::InternalError))?;

    let _ = state.email.send_email_change_code(&new_email, &code).await;

    Ok(Json(
        serde_json::json!({ "message": "Verification code sent to new email" }),
    ))
}

/// POST /api/tenant/confirm-email-change
#[derive(Deserialize)]
pub struct ConfirmEmailChangeRequest {
    pub new_email: String,
    pub code: String,
}

pub async fn confirm_email_change(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Json(req): Json<ConfirmEmailChangeRequest>,
) -> ApiResult<serde_json::Value> {
    let new_email = req.new_email.trim().to_lowercase();

    let record = db::email_verifications::find(&state.pool, &new_email, "email_change")
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?
        .ok_or_else(|| AppError::new(ErrorCode::NotFound))?;

    // Verify tenant_id from metadata to prevent cross-tenant attacks
    if let Some(ref meta) = record.metadata {
        let meta: serde_json::Value =
            serde_json::from_str(meta).map_err(|_| AppError::new(ErrorCode::InternalError))?;
        if meta.get("tenant_id").and_then(|v| v.as_str()) != Some(&identity.tenant_id) {
            return Err(AppError::new(ErrorCode::PermissionDenied));
        }
    } else {
        return Err(AppError::new(ErrorCode::PermissionDenied));
    }

    let now = shared::util::now_millis();
    if now > record.expires_at {
        return Err(AppError::new(ErrorCode::VerificationCodeExpired));
    }
    if record.attempts >= 3 {
        return Err(AppError::new(ErrorCode::TooManyAttempts));
    }

    db::email_verifications::increment_attempts(&state.pool, &new_email, "email_change")
        .await
        .map_err(|e| {
            tracing::error!("Failed to increment attempts: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    if !verify_password(&req.code, &record.code) {
        return Err(AppError::new(ErrorCode::VerificationCodeInvalid));
    }

    db::tenants::update_email(&state.pool, &identity.tenant_id, &new_email)
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?;

    let _ = db::email_verifications::delete(&state.pool, &new_email, "email_change").await;

    let old_email = record
        .metadata
        .as_deref()
        .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
        .and_then(|v| {
            v.get("old_email")
                .and_then(|e| e.as_str())
                .map(String::from)
        })
        .unwrap_or_else(|| identity.email.clone());
    let detail = serde_json::json!({ "old_email": old_email, "new_email": new_email });
    let _ = db::audit::log(
        &state.pool,
        &identity.tenant_id,
        "email_changed",
        Some(&detail),
        None,
        now,
    )
    .await;

    Ok(Json(serde_json::json!({ "message": "Email updated" })))
}

/// POST /api/tenant/change-password
#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

pub async fn change_password(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Json(req): Json<ChangePasswordRequest>,
) -> ApiResult<serde_json::Value> {
    if req.new_password.len() < 8 {
        return Err(AppError::new(ErrorCode::PasswordTooShort));
    }

    let tenant = db::tenants::find_by_id(&state.pool, &identity.tenant_id)
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?
        .ok_or_else(|| AppError::new(ErrorCode::TenantNotFound))?;

    if !verify_password(&req.current_password, &tenant.hashed_password) {
        return Err(AppError::new(ErrorCode::InvalidCredentials));
    }

    let hashed =
        hash_password(&req.new_password).map_err(|_| AppError::new(ErrorCode::InternalError))?;
    db::tenants::update_password(&state.pool, &identity.tenant_id, &hashed)
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?;

    let now = shared::util::now_millis();
    let _ = db::audit::log(
        &state.pool,
        &identity.tenant_id,
        "password_changed",
        None,
        None,
        now,
    )
    .await;

    Ok(Json(serde_json::json!({ "message": "Password changed" })))
}
