use axum::Json;
use axum::extract::State;
use shared::activation::{TokenRefreshRequest, TokenRefreshResponse};
use shared::error::{AppError, ErrorCode};

use crate::auth::tenant_auth;
use crate::db::{refresh_tokens, tenants};
use crate::state::AppState;

pub async fn refresh_token(
    State(state): State<AppState>,
    Json(req): Json<TokenRefreshRequest>,
) -> Result<Json<TokenRefreshResponse>, AppError> {
    let (tenant_id, _device_id, new_refresh_token) =
        refresh_tokens::rotate(&state.pool, &req.refresh_token)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Database error during token refresh");
                AppError::new(ErrorCode::InternalError)
            })?
            .ok_or_else(|| AppError::new(ErrorCode::TokenExpired))?;

    let tenant = tenants::find_by_id(&state.pool, &tenant_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find tenant during refresh");
            AppError::new(ErrorCode::InternalError)
        })?
        .ok_or_else(|| AppError::new(ErrorCode::TenantNotFound))?;

    let token =
        tenant_auth::create_token(&tenant.id, &tenant.email, &state.jwt_secret).map_err(|e| {
            tracing::error!(error = %e, "Failed to create JWT token");
            AppError::new(ErrorCode::InternalError)
        })?;

    tracing::debug!(tenant_id = %tenant.id, "Token refreshed");

    Ok(Json(TokenRefreshResponse {
        token,
        refresh_token: new_refresh_token,
    }))
}
