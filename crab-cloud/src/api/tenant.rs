//! Tenant management API endpoints

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use shared::error::{AppError, ErrorCode};
use sqlx;

use crate::auth::tenant_auth::TenantIdentity;
use crate::db::{self, commands, tenant_queries};
use crate::state::AppState;

type ApiResult<T> = Result<Json<T>, AppError>;

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
    pub status: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<LoginResponse> {
    let tenant = crate::db::tenants::find_by_email(&state.pool, &req.email)
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

    let token =
        crate::auth::tenant_auth::create_token(&tenant.id, &tenant.email, &state.jwt_secret)
            .map_err(|e| {
                tracing::error!("JWT creation failed: {e}");
                AppError::new(ErrorCode::InternalError)
            })?;

    let now = shared::util::now_millis();
    let _ = crate::db::audit::log(&state.pool, &tenant.id, "login", None, None, now).await;

    Ok(Json(LoginResponse {
        token,
        tenant_id: tenant.id.clone(),
        status: tenant.status,
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

/// GET /api/tenant/stores
pub async fn list_stores(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
) -> ApiResult<Vec<shared::cloud::StoreDetailResponse>> {
    let stores = tenant_queries::list_stores(&state.pool, &identity.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Stores query error: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    let mut result = Vec::new();
    for store in stores {
        let store_info = tenant_queries::get_store_info(&state.pool, store.id, &identity.tenant_id)
            .await
            .map_err(|e| {
                tracing::error!(store_id = store.id, "Failed to get store_info: {e}");
                AppError::new(ErrorCode::InternalError)
            })?;

        result.push(shared::cloud::StoreDetailResponse {
            id: store.id,
            entity_id: store.entity_id,
            device_id: store.device_id,
            last_sync_at: store.last_sync_at,
            registered_at: store.registered_at,
            store_info,
        });
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
        AppError::new(ErrorCode::InternalError)
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
        AppError::new(ErrorCode::InternalError)
    })?;

    Ok(Json(reports))
}

/// GET /api/tenant/overview?from=&to=
///
/// Tenant-wide overview: all stores combined.
#[derive(Deserialize)]
pub struct OverviewQuery {
    pub from: i64,
    pub to: i64,
}

pub async fn get_tenant_overview(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Query(query): Query<OverviewQuery>,
) -> ApiResult<tenant_queries::StoreOverview> {
    let overview =
        tenant_queries::get_tenant_overview(&state.pool, &identity.tenant_id, query.from, query.to)
            .await
            .map_err(|e| {
                tracing::error!("Tenant overview query error: {e}");
                AppError::new(ErrorCode::InternalError)
            })?;

    Ok(Json(overview))
}

/// GET /api/tenant/stores/:id/overview?from=&to=
///
/// Real-time statistics computed from cloud_archived_orders + desglose + details.
pub async fn get_store_overview(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Query(query): Query<OverviewQuery>,
) -> ApiResult<tenant_queries::StoreOverview> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let overview = tenant_queries::get_store_overview(
        &state.pool,
        store_id,
        &identity.tenant_id,
        query.from,
        query.to,
    )
    .await
    .map_err(|e| {
        tracing::error!("Overview query error: {e}");
        AppError::new(ErrorCode::InternalError)
    })?;

    Ok(Json(overview))
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
            AppError::new(ErrorCode::InternalError)
        })?;

    Ok(Json(products))
}

/// GET /api/tenant/stores/:id/orders/:order_key/detail
///
/// 30-day cache first, fallback to on-demand edge fetch if edge is online.
pub async fn get_order_detail(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, order_key)): Path<(i64, String)>,
) -> ApiResult<shared::cloud::OrderDetailResponse> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    // 1. Check 30-day cache
    if let Some(detail_json) =
        tenant_queries::get_order_detail(&state.pool, store_id, &identity.tenant_id, &order_key)
            .await
            .map_err(|e| {
                tracing::error!("Order detail query error: {e}");
                AppError::new(ErrorCode::InternalError)
            })?
    {
        let detail: shared::cloud::OrderDetailPayload = serde_json::from_value(detail_json)
            .map_err(|e| {
                tracing::error!("Failed to deserialize cached OrderDetailPayload: {e}");
                AppError::new(ErrorCode::InternalError)
            })?;

        let desglose = tenant_queries::get_order_desglose(
            &state.pool,
            store_id,
            &identity.tenant_id,
            &order_key,
        )
        .await
        .map_err(|e| {
            tracing::error!(order_key = %order_key, "Failed to query desglose: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

        return Ok(Json(shared::cloud::OrderDetailResponse {
            source: "cache".to_string(),
            detail,
            desglose,
        }));
    }

    // 2. Cache miss — fetch from edge via WS command (requires edge online)
    let Some(sender) = state.connected_edges.get(&store_id).map(|s| s.clone()) else {
        return Err(AppError::with_message(
            ErrorCode::NotFound,
            "Order detail expired and edge server is offline",
        ));
    };

    let now = shared::util::now_millis();
    let command_id = uuid::Uuid::new_v4().to_string();

    let (tx, rx) = tokio::sync::oneshot::channel();
    state.pending_requests.insert(command_id.clone(), (now, tx));

    let cloud_cmd = shared::cloud::CloudCommand {
        id: command_id.clone(),
        command_type: "get_order_detail".to_string(),
        payload: serde_json::json!({ "order_key": order_key }),
        created_at: now,
    };

    if sender.try_send(cloud_cmd).is_err() {
        state.pending_requests.remove(&command_id);
        return Err(AppError::with_message(
            ErrorCode::NotFound,
            "Edge server command queue full",
        ));
    }

    let result = match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
        Ok(Ok(r)) => r,
        Ok(Err(_)) => {
            state.pending_requests.remove(&command_id);
            return Err(AppError::with_message(
                ErrorCode::NotFound,
                "Edge server disconnected during fetch",
            ));
        }
        Err(_) => {
            state.pending_requests.remove(&command_id);
            return Err(AppError::with_message(
                ErrorCode::NotFound,
                "Order detail fetch timed out",
            ));
        }
    };

    if !result.success {
        return Err(AppError::with_message(
            ErrorCode::NotFound,
            result
                .error
                .unwrap_or_else(|| "Edge could not find order detail".to_string()),
        ));
    }

    let Some(data) = result.data else {
        return Err(AppError::with_message(
            ErrorCode::NotFound,
            "Edge returned empty order detail",
        ));
    };

    let detail_sync: shared::cloud::OrderDetailSync =
        serde_json::from_value(data).map_err(|e| {
            tracing::error!("Failed to deserialize on-demand OrderDetailSync: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    // 3. Write fetched detail back to cache (best-effort)
    let archived_order_id: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM cloud_archived_orders WHERE edge_server_id = $1 AND tenant_id = $2 AND order_key = $3",
    )
    .bind(store_id)
    .bind(&identity.tenant_id)
    .bind(&order_key)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    if let Some(order_id) = archived_order_id
        && let Ok(detail_json) = serde_json::to_value(&detail_sync.detail)
    {
        let _ = sqlx::query(
            r#"
                INSERT INTO cloud_order_details (archived_order_id, detail, synced_at)
                VALUES ($1, $2, $3)
                ON CONFLICT (archived_order_id)
                DO UPDATE SET detail = EXCLUDED.detail, synced_at = EXCLUDED.synced_at
                "#,
        )
        .bind(order_id)
        .bind(&detail_json)
        .bind(now)
        .execute(&state.pool)
        .await;
    }

    // Audit
    let fetch_detail = serde_json::json!({
        "order_key": order_key,
        "store_id": store_id,
    });
    let _ = crate::db::audit::log(
        &state.pool,
        &identity.tenant_id,
        "order_detail_fetched",
        Some(&fetch_detail),
        None,
        now,
    )
    .await;

    Ok(Json(shared::cloud::OrderDetailResponse {
        source: "edge".to_string(),
        detail: detail_sync.detail,
        desglose: detail_sync.desglose,
    }))
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
        AppError::new(ErrorCode::InternalError)
    })?;

    // Try real-time push via WebSocket if edge is connected.
    // If try_send succeeds, mark as 'delivered' immediately to prevent
    // get_pending on reconnect from picking it up again (double-delivery).
    let mut ws_pushed = false;
    if let Some(sender) = state.connected_edges.get(&store_id) {
        let cloud_cmd = shared::cloud::CloudCommand {
            id: command_id.to_string(),
            command_type: req.command_type.clone(),
            payload: req.payload.clone(),
            created_at: now,
        };
        if sender.try_send(cloud_cmd).is_ok() {
            ws_pushed = true;
            let _ = commands::mark_delivered(&state.pool, &[command_id]).await;
        }
    }

    let detail = serde_json::json!({
        "command_type": req.command_type,
        "command_id": command_id,
        "ws_pushed": ws_pushed,
    });
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
        "ws_queued": ws_pushed,
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
                AppError::new(ErrorCode::InternalError)
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
            AppError::new(ErrorCode::InternalError)
        })?;

    Ok(Json(entries))
}

async fn verify_store(state: &AppState, store_id: i64, tenant_id: &str) -> Result<(), AppError> {
    tenant_queries::verify_store_ownership(&state.pool, store_id, tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Store verification error: {e}");
            AppError::new(ErrorCode::InternalError)
        })?
        .ok_or_else(|| AppError::new(ErrorCode::NotFound))?;
    Ok(())
}

/// POST /api/tenant/billing-portal — get Stripe Customer Portal URL
pub async fn billing_portal(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
) -> ApiResult<serde_json::Value> {
    let tenant = db::tenants::find_by_id(&state.pool, &identity.tenant_id)
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?
        .ok_or_else(|| AppError::new(ErrorCode::TenantNotFound))?;

    let customer_id = tenant
        .stripe_customer_id
        .as_deref()
        .ok_or_else(|| AppError::new(ErrorCode::ValidationFailed))?;

    let return_url = state.console_base_url.clone();

    let url = crate::stripe::create_billing_portal_session(
        &state.stripe_secret_key,
        customer_id,
        &return_url,
    )
    .await
    .map_err(|e| {
        tracing::error!("Billing portal error: {e}");
        AppError::new(ErrorCode::InternalError)
    })?;

    Ok(Json(serde_json::json!({ "url": url })))
}

/// POST /api/tenant/create-checkout — create Stripe checkout session (for verified tenants)
#[derive(Deserialize)]
pub struct CreateCheckoutRequest {
    pub plan: String,
}

pub async fn create_checkout(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Json(req): Json<CreateCheckoutRequest>,
) -> ApiResult<serde_json::Value> {
    let plan = req.plan.as_str();
    if !matches!(plan, "basic" | "pro" | "basic_yearly" | "pro_yearly") {
        return Err(AppError::new(ErrorCode::ValidationFailed));
    }

    let tenant = db::tenants::find_by_id(&state.pool, &identity.tenant_id)
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?
        .ok_or_else(|| AppError::new(ErrorCode::TenantNotFound))?;

    // Only verified (unpaid) tenants can create checkout
    let status = shared::cloud::TenantStatus::from_db(&tenant.status);
    if status != Some(shared::cloud::TenantStatus::Verified) {
        return Err(AppError::new(ErrorCode::ValidationFailed));
    }

    // P12 certificate must be uploaded before payment (Verifactu compliance)
    let p12 = db::p12::find_by_tenant(&state.pool, &identity.tenant_id)
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?;
    if p12.is_none() {
        return Err(AppError::new(ErrorCode::P12Required));
    }

    // Create or reuse Stripe customer
    let customer_id = if let Some(ref cid) = tenant.stripe_customer_id {
        cid.clone()
    } else {
        let cid =
            crate::stripe::create_customer(&state.stripe_secret_key, &tenant.email, &tenant.id)
                .await
                .map_err(|e| {
                    tracing::error!(%e, "Failed to create Stripe customer");
                    AppError::new(ErrorCode::PaymentSetupFailed)
                })?;
        db::tenants::set_stripe_customer(&state.pool, &tenant.id, &cid)
            .await
            .map_err(|_| AppError::new(ErrorCode::InternalError))?;
        cid
    };

    let price_id = match plan {
        "pro" => &state.stripe_pro_price_id,
        "basic_yearly" => &state.stripe_basic_yearly_price_id,
        "pro_yearly" => &state.stripe_pro_yearly_price_id,
        _ => &state.stripe_basic_price_id,
    };

    let checkout_url = crate::stripe::create_checkout_session(
        &state.stripe_secret_key,
        &customer_id,
        price_id,
        plan,
        &state.console_base_url,
        &state.console_base_url,
    )
    .await
    .map_err(|e| {
        tracing::error!(%e, "Failed to create Stripe checkout");
        AppError::new(ErrorCode::PaymentSetupFailed)
    })?;

    let now = shared::util::now_millis();
    let detail = serde_json::json!({ "plan": plan });
    let _ = crate::db::audit::log(
        &state.pool,
        &identity.tenant_id,
        "checkout_created",
        Some(&detail),
        None,
        now,
    )
    .await;

    Ok(Json(serde_json::json!({
        "checkout_url": checkout_url,
    })))
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
            .map_err(|_| AppError::new(ErrorCode::InternalError))?;
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
    let code_hash = hash_password(&code).map_err(|_| AppError::new(ErrorCode::InternalError))?;
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
    db::tenants::update_password(&state.pool, &tenant.id, &hashed)
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?;

    let _ = db::email_verifications::delete(&state.pool, &email_addr, "password_reset").await;

    // Audit
    let _ = crate::db::audit::log(&state.pool, &tenant.id, "password_reset", None, None, now).await;

    Ok(Json(
        serde_json::json!({ "message": "Password has been reset" }),
    ))
}
