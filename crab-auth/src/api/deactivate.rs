use crate::db::{activations, tenants};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use shared::activation::DeactivateResponse;
use shared::error::ErrorCode;
use std::sync::Arc;

#[derive(serde::Deserialize)]
pub struct DeactivateRequest {
    pub username: String,
    pub password: String,
    pub device_id: String,
    pub entity_id: String,
}

/// POST /api/server/deactivate — 注销 Server 证书，释放配额
pub async fn deactivate_server(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DeactivateRequest>,
) -> Json<DeactivateResponse> {
    // 1. Authenticate
    let tenant = match tenants::authenticate(&state.db, &req.username, &req.password).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return Json(DeactivateResponse {
                success: false,
                error: Some("Invalid credentials".to_string()),
                error_code: Some(ErrorCode::TenantCredentialsInvalid),
            });
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error during authentication");
            return Json(DeactivateResponse {
                success: false,
                error: Some("Internal error".to_string()),
                error_code: Some(ErrorCode::InternalError),
            });
        }
    };

    // 2. Find activation and verify ownership
    let activation = match activations::find_by_entity(&state.db, &req.entity_id).await {
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

    // 3. Deactivate
    if let Err(e) = activations::deactivate(&state.db, &req.entity_id).await {
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
