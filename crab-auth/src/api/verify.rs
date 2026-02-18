use crate::db::{activations, client_connections, subscriptions, tenants};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use shared::activation::{PlanType, SubscriptionStatus, TenantVerifyData, TenantVerifyResponse};
use shared::error::ErrorCode;
use std::sync::Arc;

#[derive(serde::Deserialize)]
pub struct VerifyRequest {
    pub username: String,
    pub password: String,
    pub device_id: String,
}

/// POST /api/tenant/verify — 只验证身份，不签发证书
pub async fn verify_tenant(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyRequest>,
) -> Json<TenantVerifyResponse> {
    // 1. Authenticate
    let tenant = match tenants::authenticate(&state.db, &req.username, &req.password).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return Json(TenantVerifyResponse {
                success: false,
                error: Some("Invalid credentials".to_string()),
                error_code: Some(ErrorCode::TenantCredentialsInvalid),
                data: None,
            });
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error during authentication");
            return Json(TenantVerifyResponse {
                success: false,
                error: Some("Internal error".to_string()),
                error_code: Some(ErrorCode::InternalError),
                data: None,
            });
        }
    };

    // 2. Check subscription
    let sub = match subscriptions::get_active_subscription(&state.db, &tenant.id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Json(TenantVerifyResponse {
                success: false,
                error: Some("No active subscription".to_string()),
                error_code: Some(ErrorCode::TenantNoSubscription),
                data: None,
            });
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error fetching subscription");
            return Json(TenantVerifyResponse {
                success: false,
                error: Some("Internal error".to_string()),
                error_code: Some(ErrorCode::InternalError),
                data: None,
            });
        }
    };

    // 3. Count active servers and clients
    let active_servers = activations::count_active(&state.db, &tenant.id)
        .await
        .unwrap_or(0);
    let active_clients = client_connections::count_active(&state.db, &tenant.id)
        .await
        .unwrap_or(0);

    let server_slots_remaining = if sub.max_edge_servers > 0 {
        (sub.max_edge_servers as i64 - active_servers).max(0) as i32
    } else {
        -1 // unlimited
    };

    let client_slots_remaining = if sub.max_clients > 0 {
        (sub.max_clients as i64 - active_clients).max(0) as i32
    } else {
        -1 // unlimited
    };

    // 4. Check if current device has active server/client
    let has_active_server = activations::find_by_device(&state.db, &tenant.id, &req.device_id)
        .await
        .ok()
        .flatten()
        .map(|a| a.status == "active")
        .unwrap_or(false);

    let has_active_client =
        client_connections::find_by_device(&state.db, &tenant.id, &req.device_id)
            .await
            .ok()
            .flatten()
            .map(|c| c.status == "active")
            .unwrap_or(false);

    tracing::info!(
        tenant_id = %tenant.id,
        "Tenant verified"
    );

    let subscription_status = match sub.status.as_str() {
        "active" => SubscriptionStatus::Active,
        "past_due" => SubscriptionStatus::PastDue,
        "expired" => SubscriptionStatus::Expired,
        "canceled" => SubscriptionStatus::Canceled,
        "unpaid" => SubscriptionStatus::Unpaid,
        _ => SubscriptionStatus::Inactive,
    };
    let plan = match sub.plan.as_str() {
        "pro" => PlanType::Pro,
        "enterprise" => PlanType::Enterprise,
        _ => PlanType::Basic,
    };

    Json(TenantVerifyResponse {
        success: true,
        error: None,
        error_code: None,
        data: Some(TenantVerifyData {
            tenant_id: tenant.id,
            subscription_status,
            plan,
            server_slots_remaining,
            client_slots_remaining,
            has_active_server,
            has_active_client,
        }),
    })
}
