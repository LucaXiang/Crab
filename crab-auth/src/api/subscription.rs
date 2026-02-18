use crate::db::{p12, subscriptions, tenants};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use shared::activation::{PlanType, SubscriptionInfo, SubscriptionStatus};
use shared::error::ErrorCode;
use std::sync::Arc;

#[derive(serde::Deserialize)]
pub struct SubscriptionRequest {
    pub username: String,
    pub password: String,
}

pub async fn get_subscription_status(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SubscriptionRequest>,
) -> Json<serde_json::Value> {
    // 0. Authenticate tenant
    let tenant = match tenants::authenticate(&state.db, &payload.username, &payload.password).await
    {
        Ok(Some(t)) => t,
        Ok(None) => {
            return Json(serde_json::json!({
                "success": false,
                "error": "Invalid credentials",
                "error_code": ErrorCode::TenantCredentialsInvalid
            }));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error during authentication");
            return Json(serde_json::json!({
                "success": false,
                "error": "Internal error",
                "error_code": ErrorCode::InternalError
            }));
        }
    };

    let tenant_id = &tenant.id;

    // 1. Load Tenant CA
    let tenant_ca = match state.ca_store.load_tenant_ca(tenant_id).await {
        Ok(ca) => ca,
        Err(e) => {
            tracing::error!(error = %e, tenant_id = %tenant_id, "Tenant CA error");
            return Json(serde_json::json!({
                "success": false,
                "error": "Internal error",
                "error_code": ErrorCode::AuthServerError
            }));
        }
    };

    // 2. Query subscription from PG
    let sub = match subscriptions::get_active_subscription(&state.db, tenant_id).await {
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

    let status = parse_status(&sub.status);
    let plan = parse_plan(&sub.plan);
    let signature_valid_until = shared::util::now_millis() + 7 * 24 * 60 * 60 * 1000;

    let subscription = SubscriptionInfo {
        tenant_id: tenant_id.clone(),
        id: Some(sub.id),
        status,
        plan,
        starts_at: shared::util::now_millis(),
        expires_at: sub.current_period_end,
        features: sub.features,
        max_stores: sub.max_edge_servers as u32,
        max_clients: sub.max_clients as u32,
        signature_valid_until,
        signature: String::new(),
        last_checked_at: 0,
        p12: match p12::get_p12_info(&state.db, tenant_id).await {
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

fn parse_status(s: &str) -> SubscriptionStatus {
    match s {
        "active" => SubscriptionStatus::Active,
        "past_due" => SubscriptionStatus::PastDue,
        "canceled" => SubscriptionStatus::Canceled,
        "unpaid" => SubscriptionStatus::Unpaid,
        "expired" => SubscriptionStatus::Expired,
        _ => SubscriptionStatus::Inactive,
    }
}

fn parse_plan(s: &str) -> PlanType {
    match s {
        "pro" => PlanType::Pro,
        "enterprise" => PlanType::Enterprise,
        _ => PlanType::Basic,
    }
}
