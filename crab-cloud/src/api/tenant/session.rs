//! Session management: list active sessions, revoke a session

use axum::Json;
use axum::extract::State;
use http::request::Parts;
use serde::Serialize;
use shared::error::{AppError, ErrorCode};

use crate::db::refresh_tokens;
use crate::state::AppState;

use super::ApiResult;

#[derive(Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub device_id: String,
    pub user_agent: String,
    pub ip_address: String,
    pub created_at: i64,
    pub is_current: bool,
}

/// GET /api/tenant/sessions
pub async fn list_sessions(
    State(state): State<AppState>,
    parts: Parts,
) -> ApiResult<Vec<SessionInfo>> {
    let tenant_id = parts
        .extensions
        .get::<crate::auth::tenant_auth::TenantIdentity>()
        .ok_or_else(|| AppError::new(ErrorCode::TokenExpired))?
        .tenant_id;

    // Get current token from Authorization header to mark current session
    let current_token = parts
        .headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");

    let rows = refresh_tokens::list_active(&state.pool, tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list sessions: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    // We can't directly know which refresh token corresponds to the current JWT,
    // but we can use the most recently created token as a heuristic
    let _ = current_token; // JWT doesn't map to refresh token directly

    let sessions = rows
        .into_iter()
        .map(|r| SessionInfo {
            id: r.id,
            device_id: r.device_id,
            user_agent: r.user_agent,
            ip_address: r.ip_address,
            created_at: r.created_at,
            is_current: false, // Frontend can match by comparing with stored refresh token
        })
        .collect();

    Ok(Json(sessions))
}

#[derive(serde::Deserialize)]
pub struct RevokeRequest {
    pub session_id: String,
}

/// POST /api/tenant/sessions/revoke
pub async fn revoke_session(
    State(state): State<AppState>,
    parts: Parts,
    Json(req): Json<RevokeRequest>,
) -> ApiResult<serde_json::Value> {
    let tenant_id = parts
        .extensions
        .get::<crate::auth::tenant_auth::TenantIdentity>()
        .ok_or_else(|| AppError::new(ErrorCode::TokenExpired))?
        .tenant_id;

    let revoked = refresh_tokens::revoke_session(&state.pool, tenant_id, &req.session_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to revoke session: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    if !revoked {
        return Err(AppError::new(ErrorCode::NotFound));
    }

    Ok(Json(serde_json::json!({ "revoked": true })))
}
