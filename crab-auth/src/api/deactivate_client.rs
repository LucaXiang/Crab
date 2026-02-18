use crate::db::{client_connections, tenants};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use shared::activation::DeactivateResponse;
use shared::error::ErrorCode;
use std::sync::Arc;

#[derive(serde::Deserialize)]
pub struct DeactivateClientRequest {
    pub username: String,
    pub password: String,
    pub device_id: String,
    pub entity_id: String,
}

/// POST /api/client/deactivate — 注销 Client 证书，释放配额
pub async fn deactivate_client(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DeactivateClientRequest>,
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

    // 2. Find client connection and verify ownership
    let connection = match client_connections::find_by_entity(&state.db, &req.entity_id).await {
        Ok(Some(c)) if c.tenant_id == tenant.id && c.device_id == req.device_id => c,
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
                error: Some("Client connection not found".to_string()),
                error_code: Some(ErrorCode::NotFound),
            });
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding client connection");
            return Json(DeactivateResponse {
                success: false,
                error: Some("Internal error".to_string()),
                error_code: Some(ErrorCode::InternalError),
            });
        }
    };

    if connection.status != "active" {
        return Json(DeactivateResponse {
            success: false,
            error: Some(format!("Client is already {}", connection.status)),
            error_code: Some(ErrorCode::ValidationFailed),
        });
    }

    // 3. Deactivate
    if let Err(e) = client_connections::deactivate(&state.db, &req.entity_id).await {
        tracing::error!(error = %e, "Failed to deactivate client");
        return Json(DeactivateResponse {
            success: false,
            error: Some("Failed to deactivate".to_string()),
            error_code: Some(ErrorCode::InternalError),
        });
    }

    tracing::info!(
        entity_id = %req.entity_id,
        tenant_id = %tenant.id,
        "Client deactivated"
    );

    Json(DeactivateResponse {
        success: true,
        error: None,
        error_code: None,
    })
}
