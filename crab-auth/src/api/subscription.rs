use crate::db::subscriptions;
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use shared::activation::{PlanType, SubscriptionInfo, SubscriptionStatus};
use std::sync::Arc;

#[derive(serde::Deserialize)]
pub struct SubscriptionRequest {
    pub tenant_id: String,
}

pub async fn get_subscription_status(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SubscriptionRequest>,
) -> Json<serde_json::Value> {
    // 1. Load Tenant CA
    let tenant_ca = match state.ca_store.load_tenant_ca(&payload.tenant_id).await {
        Ok(ca) => ca,
        Err(e) => {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Tenant not found or CA error: {}", e)
            }));
        }
    };

    // 2. Query subscription from PG
    let sub = match subscriptions::get_active_subscription(&state.db, &payload.tenant_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Json(serde_json::json!({
                "success": false,
                "error": "No subscription found"
            }));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error fetching subscription");
            return Json(serde_json::json!({
                "success": false,
                "error": "Internal error"
            }));
        }
    };

    let status = parse_status(&sub.status);
    let plan = parse_plan(&sub.plan);
    let signature_valid_until = shared::util::now_millis() + 7 * 24 * 60 * 60 * 1000;

    let subscription = SubscriptionInfo {
        tenant_id: payload.tenant_id.clone(),
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
    };

    let signed = match subscription.sign(&tenant_ca.key_pem()) {
        Ok(s) => s,
        Err(e) => {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to sign subscription: {}", e)
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
