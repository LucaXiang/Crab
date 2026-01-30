//! Audit Log API Handlers

use axum::{
    extract::{Query, State},
    Json,
};

use crate::audit::{AuditChainVerification, AuditListResponse, AuditQuery};
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

/// 审计链验证查询参数
#[derive(Debug, serde::Deserialize)]
pub struct VerifyQuery {
    pub from: Option<i64>,
    pub to: Option<i64>,
}

/// GET /api/audit-log/verify — 验证审计链完整性
pub async fn verify_chain(
    State(state): State<ServerState>,
    Query(query): Query<VerifyQuery>,
) -> AppResult<Json<AuditChainVerification>> {
    let verification = state.audit_service.verify_chain(query.from, query.to).await?;
    Ok(Json(verification))
}
