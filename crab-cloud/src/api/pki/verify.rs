use crate::auth::tenant_auth;
use crate::db::{activations, client_connections, refresh_tokens, subscriptions, tenants};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use shared::activation::{PlanType, SubscriptionStatus, TenantVerifyData, TenantVerifyResponse};
use shared::error::ErrorCode;

use super::activate::{parse_plan_type, parse_subscription_status};

#[derive(serde::Deserialize)]
pub struct VerifyRequest {
    pub username: String,
    pub password: String,
    pub device_id: String,
}

pub async fn verify_tenant(
    State(state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> Json<TenantVerifyResponse> {
    let tenant = match tenants::authenticate(&state.pool, &req.username, &req.password).await {
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

    // 无订阅时仍允许登录，前端根据 subscription_status 决定后续流程
    let sub = match subscriptions::get_active_subscription(&state.pool, &tenant.id).await {
        Ok(s) => s, // Some or None
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

    let active_servers = activations::count_active(&state.pool, &tenant.id)
        .await
        .unwrap_or(0);
    let active_clients = client_connections::count_active(&state.pool, &tenant.id)
        .await
        .unwrap_or(0);

    let (server_slots_remaining, client_slots_remaining) = match &sub {
        Some(s) => {
            let sr = if s.max_edge_servers > 0 {
                (s.max_edge_servers as i64 - active_servers).max(0) as i32
            } else {
                -1
            };
            let cr = if s.max_clients > 0 {
                (s.max_clients as i64 - active_clients).max(0) as i32
            } else {
                -1
            };
            (sr, cr)
        }
        None => (0, 0),
    };

    let has_active_server = activations::find_by_device(&state.pool, &tenant.id, &req.device_id)
        .await
        .ok()
        .flatten()
        .map(|a| a.status == "active")
        .unwrap_or(false);

    let has_active_client =
        client_connections::find_by_device(&state.pool, &tenant.id, &req.device_id)
            .await
            .ok()
            .flatten()
            .map(|c| c.status == "active")
            .unwrap_or(false);

    tracing::info!(
        tenant_id = %tenant.id,
        has_subscription = sub.is_some(),
        "Tenant verified"
    );

    let (subscription_status, plan) = match &sub {
        Some(s) => (
            parse_subscription_status(&s.status),
            parse_plan_type(&s.plan),
        ),
        None => (SubscriptionStatus::Inactive, PlanType::Basic),
    };

    // Generate JWT session token for subsequent operations (activate, deactivate)
    let token = match tenant_auth::create_token(&tenant.id, &tenant.email, &state.jwt_secret) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "Failed to create JWT token");
            return Json(TenantVerifyResponse {
                success: false,
                error: Some("Internal error".to_string()),
                error_code: Some(ErrorCode::InternalError),
                data: None,
            });
        }
    };

    let refresh_token = match refresh_tokens::create(&state.pool, &tenant.id, &req.device_id).await
    {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "Failed to create refresh token");
            return Json(TenantVerifyResponse {
                success: false,
                error: Some("Internal error".to_string()),
                error_code: Some(ErrorCode::InternalError),
                data: None,
            });
        }
    };

    Json(TenantVerifyResponse {
        success: true,
        error: None,
        error_code: None,
        data: Some(TenantVerifyData {
            tenant_id: tenant.id,
            token,
            refresh_token,
            subscription_status,
            plan,
            server_slots_remaining,
            client_slots_remaining,
            has_active_server,
            has_active_client,
        }),
    })
}
