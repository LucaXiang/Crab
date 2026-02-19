use crate::db::{p12, subscriptions};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use shared::activation::{SignedBinding, SubscriptionInfo};
use shared::error::ErrorCode;

use super::activate::{parse_plan_type, parse_subscription_status};

/// Subscription request — authenticated via SignedBinding (no password stored on device)
#[derive(serde::Deserialize)]
pub struct SubscriptionRequest {
    pub binding: SignedBinding,
}

pub async fn get_subscription_status(
    State(state): State<AppState>,
    Json(payload): Json<SubscriptionRequest>,
) -> Json<serde_json::Value> {
    let tenant_id = &payload.binding.tenant_id;

    // Verify binding signature using Tenant CA
    let tenant_ca = match state.ca_store.load_tenant_ca(tenant_id).await {
        Ok(ca) => ca,
        Err(e) => {
            tracing::error!(error = %e, tenant_id = %tenant_id, "Tenant CA not found");
            return Json(serde_json::json!({
                "success": false,
                "error": "Invalid binding",
                "error_code": ErrorCode::TenantCredentialsInvalid
            }));
        }
    };

    if let Err(e) = payload.binding.verify_signature(tenant_ca.cert_pem()) {
        tracing::warn!(tenant_id = %tenant_id, error = %e, "Binding signature verification failed");
        return Json(serde_json::json!({
            "success": false,
            "error": "Invalid binding signature",
            "error_code": ErrorCode::TenantCredentialsInvalid
        }));
    }

    let sub = match subscriptions::get_active_subscription(&state.pool, tenant_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Json(serde_json::json!({
                "success": false,
                "error": "No subscription found",
                "error_code": ErrorCode::TenantNoSubscription
            }));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error fetching subscription");
            return Json(serde_json::json!({
                "success": false,
                "error": "Internal error",
                "error_code": ErrorCode::InternalError
            }));
        }
    };

    let status = parse_subscription_status(&sub.status);
    let plan = parse_plan_type(&sub.plan);
    let signature_valid_until = shared::util::now_millis() + 7 * 24 * 60 * 60 * 1000;

    let subscription = SubscriptionInfo {
        tenant_id: tenant_id.clone(),
        id: Some(sub.id),
        status,
        plan,
        starts_at: shared::util::now_millis(),
        expires_at: sub.current_period_end,
        features: sub.features,
        max_stores: plan.max_stores() as u32,
        max_clients: sub.max_clients as u32,
        signature_valid_until,
        signature: String::new(),
        last_checked_at: 0,
        p12: match p12::get_p12_info(&state.pool, tenant_id).await {
            Ok(info) => Some(info),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to query P12 info, defaulting to None");
                None
            }
        },
    };

    let signed = match subscription.sign(&tenant_ca.key_pem()) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "Failed to sign subscription");
            return Json(serde_json::json!({
                "success": false,
                "error": "Internal error",
                "error_code": ErrorCode::AuthServerError
            }));
        }
    };

    Json(serde_json::json!({
        "success": true,
        "subscription": signed
    }))
}
