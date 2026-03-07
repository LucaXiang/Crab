//! Anulación API Handlers

use crate::archiving::AnulacionService;
use crate::archiving::service::ArchiveError;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::utils::{AppError, AppResult};
use axum::{
    Json,
    extract::{Path, State},
};

use crate::archiving::anulacion::{AnulacionResponse, CreateAnulacionRequest};

/// Helper: construct AnulacionService from state
fn anulacion_service(state: &ServerState) -> Result<AnulacionService, AppError> {
    let archive_service = state
        .orders_manager
        .archive_service()
        .ok_or_else(|| AppError::internal("Archive service not initialized"))?;

    Ok(AnulacionService::new(
        state.pool.clone(),
        archive_service.hash_chain_lock().clone(),
    ))
}

/// POST /api/anulacion - 创建作废
pub async fn create(
    State(state): State<ServerState>,
    current_user: CurrentUser,
    Json(request): Json<CreateAnulacionRequest>,
) -> AppResult<Json<AnulacionResponse>> {
    let service = anulacion_service(&state)?;
    let response = service
        .create_anulacion(&request, current_user.id, &current_user.name)
        .await?;

    // Notify cloud worker to sync
    state.archive_notify.notify_one();

    Ok(Json(response))
}

/// GET /api/anulacion/eligibility/:order_pk - 检查是否可以作废
pub async fn check_eligibility(
    State(state): State<ServerState>,
    Path(order_pk): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let service = anulacion_service(&state)?;
    match service.check_anulacion_eligibility(order_pk).await {
        Ok(()) => Ok(Json(serde_json::json!({ "eligible": true }))),
        Err(ArchiveError::BusinessRule(code, msg)) => Ok(Json(serde_json::json!({
            "eligible": false,
            "code": code as u16,
            "reason": msg
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "eligible": false,
            "reason": e.to_string()
        }))),
    }
}

/// GET /api/anulacion/by-order/:order_pk - 获取订单的作废记录
pub async fn get_by_order(
    State(state): State<ServerState>,
    Path(order_pk): Path<i64>,
) -> AppResult<Json<Option<shared::models::invoice::InvoiceAnulacion>>> {
    let anulacion = crate::db::repository::anulacion::find_by_order(&state.pool, order_pk)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(anulacion))
}
