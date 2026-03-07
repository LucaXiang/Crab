//! Billing endpoints: Stripe portal, checkout, cancel, resume, change plan

use axum::{Extension, Json, extract::State};
use serde::Deserialize;
use shared::activation::{BillingInterval, PlanType};
use shared::error::{AppError, ErrorCode};

use crate::auth::tenant_auth::TenantIdentity;
use crate::db;
use crate::state::AppState;
use crate::stripe;

use super::ApiResult;

/// POST /api/tenant/billing-portal
pub async fn billing_portal(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
) -> ApiResult<serde_json::Value> {
    let tenant = db::tenants::find_by_id(&state.pool, identity.tenant_id)
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?
        .ok_or_else(|| AppError::new(ErrorCode::TenantNotFound))?;

    let customer_id = tenant
        .stripe_customer_id
        .as_deref()
        .ok_or_else(|| AppError::new(ErrorCode::ValidationFailed))?;

    let return_url = state.console_base_url.clone();

    let url =
        stripe::create_billing_portal_session(&state.stripe.secret_key, customer_id, &return_url)
            .await
            .map_err(|e| {
                tracing::error!("Billing portal error: {e}");
                AppError::new(ErrorCode::InternalError)
            })?;

    Ok(Json(serde_json::json!({ "url": url })))
}

/// Parse "basic", "pro", "basic_yearly", "pro_yearly" → (PlanType, BillingInterval)
fn parse_plan_interval(s: &str) -> Option<(PlanType, BillingInterval)> {
    match s {
        "basic" => Some((PlanType::Basic, BillingInterval::Month)),
        "pro" => Some((PlanType::Pro, BillingInterval::Month)),
        "basic_yearly" => Some((PlanType::Basic, BillingInterval::Year)),
        "pro_yearly" => Some((PlanType::Pro, BillingInterval::Year)),
        _ => None,
    }
}

/// POST /api/tenant/create-checkout
#[derive(Deserialize)]
pub struct CreateCheckoutRequest {
    pub plan: String,
}

pub async fn create_checkout(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Json(req): Json<CreateCheckoutRequest>,
) -> ApiResult<serde_json::Value> {
    let (plan_type, interval) =
        parse_plan_interval(&req.plan).ok_or_else(|| AppError::new(ErrorCode::ValidationFailed))?;

    let tenant = db::tenants::find_by_id(&state.pool, identity.tenant_id)
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?
        .ok_or_else(|| AppError::new(ErrorCode::TenantNotFound))?;

    // Only verified (unpaid) tenants can create checkout
    let status = shared::cloud::TenantStatus::from_db(&tenant.status);
    if status != Some(shared::cloud::TenantStatus::Verified) {
        return Err(AppError::new(ErrorCode::ValidationFailed));
    }

    // P12 certificate must be uploaded before payment (Verifactu compliance)
    let p12 = db::p12::find_by_tenant(&state.pool, identity.tenant_id)
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?;
    if p12.is_none() {
        return Err(AppError::new(ErrorCode::P12Required));
    }

    // Create or reuse Stripe customer
    let customer_id = if let Some(ref cid) = tenant.stripe_customer_id {
        cid.clone()
    } else {
        let cid = stripe::create_customer(
            &state.stripe.secret_key,
            &tenant.email,
            &tenant.id.to_string(),
        )
        .await
        .map_err(|e| {
            tracing::error!(%e, "Failed to create Stripe customer");
            AppError::new(ErrorCode::PaymentSetupFailed)
        })?;
        db::tenants::set_stripe_customer(&state.pool, tenant.id, &cid)
            .await
            .map_err(|_| AppError::new(ErrorCode::InternalError))?;
        cid
    };

    let price_id = state.stripe.price_for(plan_type, interval);

    let checkout_url = stripe::create_checkout_session(
        &state.stripe.secret_key,
        &customer_id,
        price_id,
        plan_type.as_str(),
        &state.console_base_url,
        &state.console_base_url,
    )
    .await
    .map_err(|e| {
        tracing::error!(%e, "Failed to create Stripe checkout");
        AppError::new(ErrorCode::PaymentSetupFailed)
    })?;

    let now = shared::util::now_millis();
    let detail = serde_json::json!({ "plan": plan_type.as_str(), "interval": interval.as_str() });
    let _ = crate::db::audit::log(
        &state.pool,
        identity.tenant_id,
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

/// POST /api/tenant/cancel-subscription
pub async fn cancel_subscription(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
) -> ApiResult<serde_json::Value> {
    let sub = db::subscriptions::get_latest_subscription(&state.pool, identity.tenant_id)
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?
        .ok_or_else(|| AppError::new(ErrorCode::NotFound))?;

    if !matches!(sub.status.as_str(), "active" | "trialing") {
        return Err(AppError::new(ErrorCode::ValidationFailed));
    }

    stripe::cancel_subscription(&state.stripe.secret_key, &sub.id)
        .await
        .map_err(|e| {
            tracing::error!(%e, "Failed to cancel subscription on Stripe");
            AppError::new(ErrorCode::InternalError)
        })?;

    db::subscriptions::update_subscription_fields(
        &state.pool,
        &sub.id,
        &sub.status,
        None,
        Some(true),
        None,
    )
    .await
    .map_err(|_| AppError::new(ErrorCode::InternalError))?;

    Ok(Json(serde_json::json!({ "message": "ok" })))
}

/// POST /api/tenant/resume-subscription
pub async fn resume_subscription(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
) -> ApiResult<serde_json::Value> {
    let sub = db::subscriptions::get_latest_subscription(&state.pool, identity.tenant_id)
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?
        .ok_or_else(|| AppError::new(ErrorCode::NotFound))?;

    if !matches!(sub.status.as_str(), "active" | "trialing") || !sub.cancel_at_period_end {
        return Err(AppError::new(ErrorCode::ValidationFailed));
    }

    stripe::resume_subscription(&state.stripe.secret_key, &sub.id)
        .await
        .map_err(|e| {
            tracing::error!(%e, "Failed to resume subscription on Stripe");
            AppError::new(ErrorCode::InternalError)
        })?;

    db::subscriptions::update_subscription_fields(
        &state.pool,
        &sub.id,
        &sub.status,
        None,
        Some(false),
        None,
    )
    .await
    .map_err(|_| AppError::new(ErrorCode::InternalError))?;

    Ok(Json(serde_json::json!({ "message": "ok" })))
}

/// POST /api/tenant/change-plan
#[derive(Deserialize)]
pub struct ChangePlanRequest {
    pub plan: String,
}

pub async fn change_plan(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Json(req): Json<ChangePlanRequest>,
) -> ApiResult<serde_json::Value> {
    let (plan_type, interval) =
        parse_plan_interval(&req.plan).ok_or_else(|| AppError::new(ErrorCode::ValidationFailed))?;

    let sub = db::subscriptions::get_latest_subscription(&state.pool, identity.tenant_id)
        .await
        .map_err(|_| AppError::new(ErrorCode::InternalError))?
        .ok_or_else(|| AppError::new(ErrorCode::NotFound))?;

    if !matches!(sub.status.as_str(), "active" | "trialing") {
        return Err(AppError::new(ErrorCode::ValidationFailed));
    }

    // Get subscription from Stripe to find item ID
    let stripe_sub = stripe::get_subscription(&state.stripe.secret_key, &sub.id)
        .await
        .map_err(|e| {
            tracing::error!(%e, "Failed to get subscription from Stripe");
            AppError::new(ErrorCode::InternalError)
        })?;

    let item_id = stripe_sub["items"]["data"][0]["id"]
        .as_str()
        .ok_or_else(|| AppError::new(ErrorCode::InternalError))?;

    let price_id = state.stripe.price_for(plan_type, interval);

    stripe::update_subscription_price(&state.stripe.secret_key, &sub.id, item_id, price_id)
        .await
        .map_err(|e| {
            tracing::error!(%e, "Failed to change plan on Stripe");
            AppError::new(ErrorCode::InternalError)
        })?;

    db::subscriptions::update_plan(
        &state.pool,
        &sub.id,
        plan_type.as_str(),
        plan_type.max_stores_i32(),
        interval.as_str(),
    )
    .await
    .map_err(|_| AppError::new(ErrorCode::InternalError))?;

    Ok(Json(serde_json::json!({ "message": "ok" })))
}
