//! Tenant management API endpoints

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::auth::tenant_auth::TenantIdentity;
use crate::db::{self, commands, tenant_queries};
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

    // Verify password with Argon2
    let parsed_hash = argon2::PasswordHash::new(&tenant.hashed_password)
        .map_err(|_| error(500, "Internal error"))?;

    argon2::PasswordVerifier::verify_password(
        &argon2::Argon2::default(),
        req.password.as_bytes(),
        &parsed_hash,
    )
    .map_err(|_| error(401, "Invalid credentials"))?;

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

// ── Password reset helpers ──

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
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
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

    let _ = db::email_verifications::increment_attempts(&state.pool, &email_addr, "password_reset")
        .await;

    if !verify_password_hash(&req.code, &record.code) {
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
