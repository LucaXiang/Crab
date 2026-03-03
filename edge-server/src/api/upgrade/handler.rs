//! Upgrade API Handlers

use crate::archiving::UpgradeService;
use crate::archiving::upgrade::{CreateUpgradeRequest, UpgradeResponse};
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::utils::{AppError, AppResult};
use axum::{
    Json,
    extract::{Path, State},
};

/// Helper: construct UpgradeService from state
fn upgrade_service(state: &ServerState) -> Result<UpgradeService, AppError> {
    let archive_service = state
        .orders_manager
        .archive_service()
        .ok_or_else(|| AppError::internal("Archive service not initialized"))?;

    Ok(UpgradeService::new(
        state.pool.clone(),
        archive_service.hash_chain_lock().clone(),
    ))
}

/// POST /api/invoices/upgrade - 创建升级
pub async fn create(
    State(state): State<ServerState>,
    current_user: CurrentUser,
    Json(request): Json<CreateUpgradeRequest>,
) -> AppResult<Json<UpgradeResponse>> {
    let service = upgrade_service(&state)?;
    let response = service
        .create_upgrade(&request, current_user.id, &current_user.name)
        .await?;

    // Notify cloud worker to sync
    state.archive_notify.notify_one();

    Ok(Json(response))
}

/// GET /api/invoices/upgrade/eligibility/:order_pk - 检查是否可以升级
pub async fn check_eligibility(
    State(state): State<ServerState>,
    Path(order_pk): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let service = upgrade_service(&state)?;
    match service.check_upgrade_eligibility(order_pk).await {
        Ok(()) => Ok(Json(serde_json::json!({ "eligible": true }))),
        Err(e) => Ok(Json(serde_json::json!({
            "eligible": false,
            "reason": e.to_string()
        }))),
    }
}
