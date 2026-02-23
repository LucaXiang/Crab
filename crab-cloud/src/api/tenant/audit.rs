//! Audit log endpoint

use axum::{
    Extension, Json,
    extract::{Query, State},
};
use serde::Deserialize;
use shared::error::{AppError, ErrorCode};

use crate::auth::tenant_auth::TenantIdentity;
use crate::state::AppState;

use super::ApiResult;

#[derive(Deserialize)]
pub struct AuditQuery {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

/// GET /api/tenant/audit-log
pub async fn audit_log(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Query(query): Query<AuditQuery>,
) -> ApiResult<Vec<crate::db::audit::AuditEntry>> {
    let per_page = query.per_page.unwrap_or(20).min(100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let entries = crate::db::audit::query(&state.pool, &identity.tenant_id, per_page, offset)
        .await
        .map_err(|e| {
            tracing::error!("Audit log query error: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    Ok(Json(entries))
}
