use crate::auth::tenant_auth;
use crate::db::{refresh_tokens, tenants};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use shared::activation::{TokenRefreshRequest, TokenRefreshResponse};
use shared::error::ErrorCode;

pub async fn refresh_token(
    State(state): State<AppState>,
    Json(req): Json<TokenRefreshRequest>,
) -> Json<TokenRefreshResponse> {
    let (tenant_id, _device_id, new_refresh_token) =
        match refresh_tokens::rotate(&state.pool, &req.refresh_token).await {
            Ok(Some(result)) => result,
            Ok(None) => {
                return Json(TokenRefreshResponse {
                    success: false,
                    error: Some("Invalid or expired refresh token".to_string()),
                    error_code: Some(ErrorCode::TokenExpired),
                    token: None,
                    refresh_token: None,
                });
            }
            Err(e) => {
                tracing::error!(error = %e, "Database error during token refresh");
                return Json(TokenRefreshResponse {
                    success: false,
                    error: Some("Internal error".to_string()),
                    error_code: Some(ErrorCode::InternalError),
                    token: None,
                    refresh_token: None,
                });
            }
        };

    let tenant = match tenants::find_by_id(&state.pool, &tenant_id).await {
        Ok(Some(t)) => t,
        _ => {
            return Json(TokenRefreshResponse {
                success: false,
                error: Some("Tenant not found".to_string()),
                error_code: Some(ErrorCode::TenantNotFound),
                token: None,
                refresh_token: None,
            });
        }
    };

    let token = match tenant_auth::create_token(&tenant.id, &tenant.email, &state.jwt_secret) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "Failed to create JWT token");
            return Json(TokenRefreshResponse {
                success: false,
                error: Some("Internal error".to_string()),
                error_code: Some(ErrorCode::InternalError),
                token: None,
                refresh_token: None,
            });
        }
    };

    tracing::debug!(tenant_id = %tenant.id, "Token refreshed");

    Json(TokenRefreshResponse {
        success: true,
        error: None,
        error_code: None,
        token: Some(token),
        refresh_token: Some(new_refresh_token),
    })
}
