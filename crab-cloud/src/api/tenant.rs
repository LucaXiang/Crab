//! Tenant management API endpoints

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use sqlx;

use crate::auth::tenant_auth::TenantIdentity;
use crate::db::{self, commands, tenant_queries};
use crate::email;
use crate::state::AppState;

type ApiResult<T> = Result<Json<T>, (http::StatusCode, Json<serde_json::Value>)>;

/// POST /api/tenant/login
#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(serde::Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub tenant_id: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<LoginResponse> {
    let tenant = crate::db::tenants::find_by_email(&state.pool, &req.email)
        .await
        .map_err(|e| {
            tracing::error!("DB error during login: {e}");
            internal_error("Internal error")
        })?
        .ok_or_else(|| error(401, "Invalid credentials"))?;

    if !verify_password(&req.password, &tenant.hashed_password) {
        return Err(error(401, "Invalid credentials"));
    }

    if tenant.status != "active" {
        return Err(error(
            403,
            &format!("Account is not active (status: {})", tenant.status),
        ));
    }

    let token =
        crate::auth::tenant_auth::create_token(&tenant.id, &tenant.email, &state.jwt_secret)
            .map_err(|e| {
                tracing::error!("JWT creation failed: {e}");
                internal_error("Internal error")
            })?;

    let now = shared::util::now_millis();
    let _ = crate::db::audit::log(&state.pool, &tenant.id, "login", None, None, now).await;

    Ok(Json(LoginResponse {
        token,
        tenant_id: tenant.id,
    }))
}

/// GET /api/tenant/profile
pub async fn get_profile(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
) -> ApiResult<serde_json::Value> {
    let profile = tenant_queries::get_profile(&state.pool, &identity.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Profile query error: {e}");
            internal_error("Internal error")
        })?
        .ok_or_else(|| error(404, "Tenant not found"))?;

    let subscription = tenant_queries::get_subscription(&state.pool, &identity.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Subscription query error: {e}");
            internal_error("Internal error")
        })?;

    Ok(Json(serde_json::json!({
        "profile": profile,
        "subscription": subscription,
    })))
}

/// GET /api/tenant/stores
pub async fn list_stores(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
) -> ApiResult<Vec<serde_json::Value>> {
    let stores = tenant_queries::list_stores(&state.pool, &identity.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Stores query error: {e}");
            internal_error("Internal error")
        })?;

    let mut result = Vec::new();
    for store in stores {
        let store_info = tenant_queries::get_store_info(&state.pool, store.id, &identity.tenant_id)
            .await
            .unwrap_or(None);

        result.push(serde_json::json!({
            "id": store.id,
            "entity_id": store.entity_id,
            "device_id": store.device_id,
            "last_sync_at": store.last_sync_at,
            "registered_at": store.registered_at,
            "store_info": store_info,
        }));
    }

    Ok(Json(result))
}

/// GET /api/tenant/stores/:id/orders
#[derive(Deserialize)]
pub struct OrdersQuery {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
    pub status: Option<String>,
}

pub async fn list_orders(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Query(query): Query<OrdersQuery>,
) -> ApiResult<Vec<tenant_queries::ArchivedOrderSummary>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let per_page = query.per_page.unwrap_or(20).min(100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let orders = tenant_queries::list_orders(
        &state.pool,
        store_id,
        &identity.tenant_id,
        query.status.as_deref(),
        per_page,
        offset,
    )
    .await
    .map_err(|e| {
        tracing::error!("Orders query error: {e}");
        internal_error("Internal error")
    })?;

    Ok(Json(orders))
}

/// GET /api/tenant/stores/:id/stats
#[derive(Deserialize)]
pub struct StatsQuery {
    pub from: Option<i64>,
    pub to: Option<i64>,
}

pub async fn get_stats(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Query(query): Query<StatsQuery>,
) -> ApiResult<Vec<tenant_queries::DailyReportEntry>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let reports = tenant_queries::list_daily_reports(
        &state.pool,
        store_id,
        &identity.tenant_id,
        query.from,
        query.to,
    )
    .await
    .map_err(|e| {
        tracing::error!("Stats query error: {e}");
        internal_error("Internal error")
    })?;

    Ok(Json(reports))
}

/// GET /api/tenant/stores/:id/products
pub async fn list_products(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<tenant_queries::ProductEntry>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let products = tenant_queries::list_products(&state.pool, store_id, &identity.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Products query error: {e}");
            internal_error("Internal error")
        })?;

    Ok(Json(products))
}

/// POST /api/tenant/stores/:id/commands
#[derive(Deserialize)]
pub struct CreateCommandRequest {
    pub command_type: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

pub async fn create_command(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(req): Json<CreateCommandRequest>,
) -> ApiResult<serde_json::Value> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let now = shared::util::now_millis();
    let command_id = commands::create_command(
        &state.pool,
        store_id,
        &identity.tenant_id,
        &req.command_type,
        &req.payload,
        now,
    )
    .await
    .map_err(|e| {
        tracing::error!("Create command error: {e}");
        internal_error("Internal error")
    })?;

    let detail = serde_json::json!({ "command_type": req.command_type, "command_id": command_id });
    let _ = crate::db::audit::log(
        &state.pool,
        &identity.tenant_id,
        "command_created",
        Some(&detail),
        None,
        now,
    )
    .await;

    Ok(Json(serde_json::json!({
        "command_id": command_id,
        "status": "pending",
    })))
}

/// GET /api/tenant/stores/:id/commands
#[derive(Deserialize)]
pub struct CommandsQuery {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

pub async fn list_commands(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Query(query): Query<CommandsQuery>,
) -> ApiResult<Vec<commands::CommandRecord>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let per_page = query.per_page.unwrap_or(20).min(100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let commands =
        commands::get_command_history(&state.pool, store_id, &identity.tenant_id, per_page, offset)
            .await
            .map_err(|e| {
                tracing::error!("Commands query error: {e}");
                internal_error("Internal error")
            })?;

    Ok(Json(commands))
}

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

async fn verify_store(
    state: &AppState,
    store_id: i64,
    tenant_id: &str,
) -> Result<(), (http::StatusCode, Json<serde_json::Value>)> {
    tenant_queries::verify_store_ownership(&state.pool, store_id, tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Store verification error: {e}");
            internal_error("Internal error")
        })?
        .ok_or_else(|| error(404, "Store not found"))?;
    Ok(())
}

fn error(status: u16, msg: &str) -> (http::StatusCode, Json<serde_json::Value>) {
    (
        http::StatusCode::from_u16(status).unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR),
        Json(serde_json::json!({ "error": msg })),
    )
}

fn internal_error(msg: &str) -> (http::StatusCode, Json<serde_json::Value>) {
    error(500, msg)
}

/// POST /api/tenant/billing-portal — get Stripe Customer Portal URL
pub async fn billing_portal(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
) -> ApiResult<serde_json::Value> {
    let tenant = db::tenants::find_by_id(&state.pool, &identity.tenant_id)
        .await
        .map_err(|_| internal_error("Internal error"))?
        .ok_or_else(|| error(404, "Tenant not found"))?;

    let customer_id = tenant
        .stripe_customer_id
        .as_deref()
        .ok_or_else(|| error(400, "No Stripe customer linked"))?;

    let return_url = format!(
        "{}/dashboard",
        state
            .registration_success_url
            .trim_end_matches("/registration/success")
    );

    let url = crate::stripe::create_billing_portal_session(
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

// ── Account management endpoints ──

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
        return Err(error(400, "Invalid email"));
    }

    let tenant = db::tenants::find_by_id(&state.pool, &identity.tenant_id)
        .await
        .map_err(|_| internal_error("Internal error"))?
        .ok_or_else(|| error(404, "Tenant not found"))?;

    if !verify_password(&req.current_password, &tenant.hashed_password) {
        return Err(error(401, "Invalid password"));
    }

    if let Ok(Some(_)) = db::tenants::find_by_email(&state.pool, &new_email).await {
        return Err(error(409, "Email already in use"));
    }

    let code = generate_code();
    let code_hash = hash_password(&code).map_err(|_| internal_error("Internal error"))?;
    let now = shared::util::now_millis();
    let expires_at = now + 5 * 60 * 1000;

    // Store tenant_id in metadata so confirm_email_change can verify ownership
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
    .map_err(|_| internal_error("Internal error"))?;

    let _ =
        email::send_email_change_code(&state.ses, &state.ses_from_email, &new_email, &code).await;

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
        .map_err(|_| internal_error("Internal error"))?
        .ok_or_else(|| error(404, "No email change pending"))?;

    // Verify tenant_id from metadata to prevent cross-tenant attacks
    if let Some(ref meta) = record.metadata {
        let meta: serde_json::Value =
            serde_json::from_str(meta).map_err(|_| internal_error("Internal error"))?;
        if meta.get("tenant_id").and_then(|v| v.as_str()) != Some(&identity.tenant_id) {
            return Err(error(403, "Not authorized"));
        }
    } else {
        return Err(error(403, "Not authorized"));
    }

    let now = shared::util::now_millis();
    if now > record.expires_at {
        return Err(error(410, "Code expired"));
    }
    if record.attempts >= 3 {
        return Err(error(429, "Too many attempts"));
    }

    db::email_verifications::increment_attempts(&state.pool, &new_email, "email_change")
        .await
        .map_err(|e| {
            tracing::error!("Failed to increment attempts: {e}");
            internal_error("Internal error")
        })?;

    if !verify_password(&req.code, &record.code) {
        return Err(error(401, "Invalid code"));
    }

    db::tenants::update_email(&state.pool, &identity.tenant_id, &new_email)
        .await
        .map_err(|_| internal_error("Internal error"))?;

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
    let _ = crate::db::audit::log(
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
        return Err(error(400, "Password must be at least 8 characters"));
    }

    let tenant = db::tenants::find_by_id(&state.pool, &identity.tenant_id)
        .await
        .map_err(|_| internal_error("Internal error"))?
        .ok_or_else(|| error(404, "Tenant not found"))?;

    if !verify_password(&req.current_password, &tenant.hashed_password) {
        return Err(error(401, "Invalid current password"));
    }

    let hashed = hash_password(&req.new_password).map_err(|_| internal_error("Internal error"))?;
    db::tenants::update_password(&state.pool, &identity.tenant_id, &hashed)
        .await
        .map_err(|_| internal_error("Internal error"))?;

    let now = shared::util::now_millis();
    let _ = crate::db::audit::log(
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
            .map_err(|_| internal_error("Internal error"))?;
    }
    Ok(Json(serde_json::json!({ "message": "Profile updated" })))
}

use crate::util::{generate_code, hash_password, verify_password};

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
    let code_hash = hash_password(&code).map_err(|_| internal_error("Internal error"))?;
    let now = shared::util::now_millis();
    let expires_at = now + 5 * 60 * 1000;

    let _ = db::email_verifications::upsert(
        &state.pool,
        &email_addr,
        &code_hash,
        expires_at,
        now,
        "password_reset",
        None,
    )
    .await;

    let _ = crate::email::send_password_reset_code(
        &state.ses,
        &state.ses_from_email,
        &email_addr,
        &code,
    )
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
        return Err(error(400, "Password must be at least 8 characters"));
    }

    let record = db::email_verifications::find(&state.pool, &email_addr, "password_reset")
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

    db::email_verifications::increment_attempts(&state.pool, &email_addr, "password_reset")
        .await
        .map_err(|e| {
            tracing::error!("Failed to increment attempts: {e}");
            internal_error("Internal error")
        })?;

    if !verify_password(&req.code, &record.code) {
        return Err(error(401, "Invalid reset code"));
    }

    let tenant = db::tenants::find_by_email(&state.pool, &email_addr)
        .await
        .map_err(|_| internal_error("Internal error"))?
        .ok_or_else(|| error(404, "Tenant not found"))?;

    let hashed = hash_password(&req.new_password).map_err(|_| internal_error("Internal error"))?;
    db::tenants::update_password(&state.pool, &tenant.id, &hashed)
        .await
        .map_err(|_| internal_error("Internal error"))?;

    let _ = db::email_verifications::delete(&state.pool, &email_addr, "password_reset").await;

    // Audit
    let _ = crate::db::audit::log(&state.pool, &tenant.id, "password_reset", None, None, now).await;

    Ok(Json(
        serde_json::json!({ "message": "Password has been reset" }),
    ))
}
