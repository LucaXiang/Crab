//! Audit Log API Handlers

use axum::{
    extract::{Query, State},
    Json,
};

use crate::audit::{AuditListResponse, AuditQuery};
use crate::core::ServerState;
use crate::utils::AppResult;

/// GET /api/audit-log — 查询审计日志
pub async fn list(
    State(state): State<ServerState>,
    Query(query): Query<AuditQuery>,
) -> AppResult<Json<AuditListResponse>> {
    let (items, total) = state.audit_service.query(&query).await?;
    Ok(Json(AuditListResponse { items, total }))
}
