//! System Issues API Handlers

use axum::{
    extract::{Extension, State},
    Json,
};

use crate::core::ServerState;
use crate::db::repository::system_issue::{ResolveSystemIssue, SystemIssueRow};
use crate::utils::AppResult;
use crate::CurrentUser;

/// GET /api/system-issues/pending — 获取所有待处理的系统问题
pub async fn pending(
    State(state): State<ServerState>,
) -> AppResult<Json<Vec<SystemIssueRow>>> {
    let repo = crate::db::repository::SystemIssueRepository::new(state.db.clone());
    let issues = repo.find_pending().await?;
    Ok(Json(issues))
}

/// POST /api/system-issues/resolve — 回应一个系统问题
pub async fn resolve(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<ResolveSystemIssue>,
) -> AppResult<Json<serde_json::Value>> {
    let repo = crate::db::repository::SystemIssueRepository::new(state.db.clone());
    let resolved = repo
        .resolve(&req.id, &req.response, Some(&current_user.id))
        .await?;

    // 写审计日志
    state
        .audit_service
        .log(
            crate::audit::AuditAction::ResolveSystemIssue,
            "system",
            &req.id,
            Some(current_user.id.clone()),
            Some(current_user.display_name.clone()),
            serde_json::json!({
                "kind": resolved.kind,
                "response": req.response,
                "source": resolved.source,
            }),
        )
        .await;

    Ok(Json(serde_json::json!({"ok": true})))
}
