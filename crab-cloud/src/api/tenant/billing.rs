//! Billing endpoints: Stripe portal, checkout session

use axum::{Extension, Json, extract::State};
use serde::Deserialize;
use shared::error::{AppError, ErrorCode};

use crate::auth::tenant_auth::TenantIdentity;
use crate::db;
use crate::state::AppState;

use super::ApiResult;

/// POST /api/tenant/billing-portal
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
        &state.stripe.secret_key,
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
            crate::stripe::create_customer(&state.stripe.secret_key, &tenant.email, &tenant.id)
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
        "pro" => &state.stripe.pro_price_id,
        "basic_yearly" => &state.stripe.basic_yearly_price_id,
        "pro_yearly" => &state.stripe.pro_yearly_price_id,
        _ => &state.stripe.basic_price_id,
    };

    let checkout_url = crate::stripe::create_checkout_session(
        &state.stripe.secret_key,
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
