//! Audit Log API Handlers

use axum::{
    extract::{Extension, Query, State},
    Json,
};

use crate::audit::{
    AcknowledgeStartupRequest, AuditChainVerification, AuditListResponse, AuditQuery, StartupIssue,
};
use crate::core::ServerState;
use crate::utils::AppResult;
use crate::CurrentUser;

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

/// GET /api/audit-log/pending-startup — 获取待确认的启动异常
///
/// 前端启动时调用此接口，如果返回非空列表，
/// 必须弹窗要求用户输入原因后才能继续使用。
pub async fn pending_startup(
    State(state): State<ServerState>,
) -> AppResult<Json<Vec<StartupIssue>>> {
    let issues = state.audit_service.get_pending_startup().await;
    Ok(Json(issues))
}

/// POST /api/audit-log/acknowledge-startup — 确认启动异常
///
/// 用户在前端 dialog 输入原因后调用此接口。
/// 创建审计记录并清除待确认状态。
pub async fn acknowledge_startup(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<AcknowledgeStartupRequest>,
) -> AppResult<Json<serde_json::Value>> {
    state
        .audit_service
        .acknowledge_startup(
            req.reason,
            Some(current_user.id.clone()),
            Some(current_user.display_name.clone()),
        )
        .await?;

    Ok(Json(serde_json::json!({"ok": true})))
}
