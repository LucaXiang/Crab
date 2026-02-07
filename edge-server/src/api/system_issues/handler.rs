//! System Issues API Handlers

use axum::{
    extract::{Extension, State},
    Json,
};

use crate::core::ServerState;
use crate::db::repository::system_issue;
use crate::utils::AppResult;
use crate::CurrentUser;
use shared::models::{SystemIssue, SystemIssueResolve};

/// GET /api/system-issues/pending
pub async fn pending(
    State(state): State<ServerState>,
) -> AppResult<Json<Vec<SystemIssue>>> {
    let issues = system_issue::find_pending(&state.pool).await?;
    Ok(Json(issues))
}

/// POST /api/system-issues/resolve
pub async fn resolve(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<SystemIssueResolve>,
) -> AppResult<Json<serde_json::Value>> {
    let resolved = system_issue::resolve(
        &state.pool,
        req.id,
        &req.response,
        Some(&current_user.id),
    )
    .await?;

    // 写审计日志（target 指向原始问题的 audit_log 条目序号）
    let audit_target = resolved.target.clone().map(|seq| format!("#{}", seq));
    state
        .audit_service
        .log_with_target(
            crate::audit::AuditAction::ResolveSystemIssue,
            "system_issue",
            &req.id.to_string(),
            Some(current_user.id.clone()),
            Some(current_user.display_name.clone()),
            serde_json::json!({
                "kind": resolved.kind,
                "response": req.response,
                "source": resolved.source,
            }),
            audit_target,
        )
        .await;

    Ok(Json(serde_json::json!({"ok": true})))
}
