use crate::auth::tenant_auth;
use crate::db::{activations, tenants};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use shared::activation::DeactivateResponse;
use shared::error::ErrorCode;

#[derive(serde::Deserialize)]
pub struct DeactivateRequest {
    /// JWT session token
    pub token: String,
    pub device_id: String,
    pub entity_id: String,
}

pub async fn deactivate_server(
    State(state): State<AppState>,
    Json(req): Json<DeactivateRequest>,
) -> Json<DeactivateResponse> {
    let tenant_id = match tenant_auth::verify_token(&req.token, &state.jwt_secret) {
        Ok(claims) => claims.sub,
        Err(_) => {
            return Json(DeactivateResponse {
                success: false,
                error: Some("Invalid or expired token".to_string()),
                error_code: Some(ErrorCode::TokenExpired),
            });
        }
    };

    let tenant = match tenants::find_by_id(&state.pool, &tenant_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return Json(DeactivateResponse {
                success: false,
                error: Some("Tenant not found".to_string()),
                error_code: Some(ErrorCode::TenantCredentialsInvalid),
            });
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding tenant");
            return Json(DeactivateResponse {
                success: false,
                error: Some("Internal error".to_string()),
                error_code: Some(ErrorCode::InternalError),
            });
        }
    };

    let activation = match activations::find_by_entity(&state.pool, &req.entity_id).await {
        Ok(Some(a)) if a.tenant_id == tenant.id && a.device_id == req.device_id => a,
        Ok(Some(_)) => {
            return Json(DeactivateResponse {
                success: false,
                error: Some("Entity does not belong to this tenant/device".to_string()),
                error_code: Some(ErrorCode::PermissionDenied),
            });
        }
        Ok(None) => {
            return Json(DeactivateResponse {
                success: false,
                error: Some("Activation not found".to_string()),
                error_code: Some(ErrorCode::NotFound),
            });
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding activation");
            return Json(DeactivateResponse {
                success: false,
                error: Some("Internal error".to_string()),
                error_code: Some(ErrorCode::InternalError),
            });
        }
    };

    if activation.status != "active" {
        return Json(DeactivateResponse {
            success: false,
            error: Some(format!("Activation is already {}", activation.status)),
            error_code: Some(ErrorCode::ValidationFailed),
        });
    }

    if let Err(e) = activations::deactivate(&state.pool, &req.entity_id).await {
        tracing::error!(error = %e, "Failed to deactivate server");
        return Json(DeactivateResponse {
            success: false,
            error: Some("Failed to deactivate".to_string()),
            error_code: Some(ErrorCode::InternalError),
        });
    }

    tracing::info!(
        entity_id = %req.entity_id,
        tenant_id = %tenant.id,
        "Server deactivated"
    );

    Json(DeactivateResponse {
        success: true,
        error: None,
        error_code: None,
    })
}
