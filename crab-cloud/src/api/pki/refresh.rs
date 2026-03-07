use axum::Json;
use axum::extract::State;
use http::HeaderMap;
use shared::activation::{TokenRefreshRequest, TokenRefreshResponse};
use shared::error::{AppError, ErrorCode};

use crate::api::tenant::extract_client_info;
use crate::auth::tenant_auth;
use crate::db::{refresh_tokens, subscriptions, tenants};
use crate::state::AppState;

pub async fn refresh_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<TokenRefreshRequest>,
) -> Result<Json<TokenRefreshResponse>, AppError> {
    let (user_agent, ip_address) = extract_client_info(&headers);

    let (tenant_id, _device_id, new_refresh_token) =
        refresh_tokens::rotate(&state.pool, &req.refresh_token, &user_agent, &ip_address)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Database error during token refresh");
                AppError::new(ErrorCode::InternalError)
            })?
            .ok_or_else(|| AppError::new(ErrorCode::TokenExpired))?;

    let tenant = tenants::find_by_id(&state.pool, tenant_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find tenant during refresh");
            AppError::new(ErrorCode::InternalError)
        })?
        .ok_or_else(|| AppError::new(ErrorCode::TenantNotFound))?;

    // Sync subscription state from Stripe (catches missed webhooks)
    sync_subscription_from_stripe(&state, tenant_id).await;

    let token =
        tenant_auth::create_token(tenant.id, &tenant.email, &state.jwt_secret).map_err(|e| {
            tracing::error!(error = %e, "Failed to create JWT token");
            AppError::new(ErrorCode::InternalError)
        })?;

    tracing::debug!(tenant_id = tenant.id, "Token refreshed");

    Ok(Json(TokenRefreshResponse {
        token,
        refresh_token: new_refresh_token,
    }))
}

/// Sync subscription fields from Stripe API during token refresh.
/// Best-effort: errors are logged but do not block the refresh.
async fn sync_subscription_from_stripe(state: &AppState, tenant_id: i64) {
    let sub = match subscriptions::get_latest_subscription(&state.pool, tenant_id).await {
        Ok(Some(s)) => s,
        _ => return,
    };

    let stripe_sub = match crate::stripe::get_subscription(&state.stripe.secret_key, &sub.id).await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch subscription from Stripe during refresh");
            return;
        }
    };

    let status = stripe_sub["status"].as_str().unwrap_or(&sub.status);
    let cancel_at_period_end = stripe_sub["cancel_at_period_end"].as_bool();
    let current_period_end = stripe_sub["current_period_end"].as_i64().map(|s| s * 1000);
    let billing_interval = stripe_sub
        .get("plan")
        .and_then(|p| p["interval"].as_str())
        .map(|s| s.to_string());

    // Sync plan from Stripe price_id
    if let Some((plan_type, interval)) = stripe_sub["items"]["data"][0]["price"]["id"]
        .as_str()
        .and_then(|pid| state.stripe.resolve_plan(pid))
    {
        let plan_changed = plan_type.as_str() != sub.plan
            || interval.as_str() != sub.billing_interval.as_deref().unwrap_or("");
        if plan_changed
            && let Err(e) = subscriptions::update_plan(
                &state.pool,
                &sub.id,
                plan_type.as_str(),
                plan_type.max_stores_i32(),
                interval.as_str(),
            )
            .await
        {
            tracing::warn!(error = %e, "Failed to sync plan from Stripe");
        }
    }

    if let Err(e) = subscriptions::update_subscription_fields(
        &state.pool,
        &sub.id,
        status,
        current_period_end,
        cancel_at_period_end,
        billing_interval.as_deref(),
    )
    .await
    {
        tracing::warn!(error = %e, "Failed to sync subscription fields from Stripe");
    }

    // Sync tenant status
    let tenant_status = match status {
        "active" | "trialing" => shared::cloud::TenantStatus::Active,
        "past_due" | "incomplete" | "paused" => shared::cloud::TenantStatus::Suspended,
        "canceled" | "unpaid" | "incomplete_expired" => shared::cloud::TenantStatus::Canceled,
        _ => return,
    };

    let current_status = shared::cloud::TenantStatus::from_db(
        &tenants::find_by_id(&state.pool, tenant_id)
            .await
            .ok()
            .flatten()
            .map(|t| t.status)
            .unwrap_or_default(),
    );

    if current_status != Some(tenant_status.clone())
        && let Err(e) = tenants::update_status(&state.pool, tenant_id, tenant_status.as_db()).await
    {
        tracing::warn!(error = %e, "Failed to sync tenant status during refresh");
    }
}
