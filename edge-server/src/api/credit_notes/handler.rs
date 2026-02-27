//! Credit Notes API Handlers

use crate::archiving::CreditNoteService;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::utils::{AppError, AppResult};
use axum::{
    Json,
    extract::{Path, State},
};
use shared::models::{CreateCreditNoteRequest, CreditNote, CreditNoteDetail, RefundableInfo};

/// Helper: construct CreditNoteService from state
fn credit_note_service(state: &ServerState) -> Result<CreditNoteService, AppError> {
    let archive_service = state
        .orders_manager
        .archive_service()
        .ok_or_else(|| AppError::internal("Archive service not initialized"))?;

    Ok(CreditNoteService::new(
        state.pool.clone(),
        state.config.timezone,
        archive_service.hash_chain_lock().clone(),
    ))
}

/// POST /api/credit-notes - 创建退款凭证
pub async fn create(
    State(state): State<ServerState>,
    current_user: CurrentUser,
    Json(request): Json<CreateCreditNoteRequest>,
) -> AppResult<Json<CreditNoteDetail>> {
    // Get current shift_id (optional)
    let shift_id = crate::db::repository::shift::find_any_open(&state.pool)
        .await
        .ok()
        .flatten()
        .map(|s| s.id);

    let service = credit_note_service(&state)?;
    let detail = service
        .create_credit_note(&request, current_user.id, &current_user.name, shift_id)
        .await?;

    // Notify cloud worker to sync
    state.archive_notify.notify_one();

    Ok(Json(detail))
}

/// GET /api/credit-notes/:id - 获取退款凭证详情
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<CreditNoteDetail>> {
    let service = credit_note_service(&state)?;
    let detail = service
        .get_detail(id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Credit note {} not found", id)))?;
    Ok(Json(detail))
}

/// GET /api/credit-notes/by-order/:order_pk - 获取订单的所有退款记录
pub async fn list_by_order(
    State(state): State<ServerState>,
    Path(order_pk): Path<i64>,
) -> AppResult<Json<Vec<CreditNote>>> {
    let service = credit_note_service(&state)?;
    let notes = service.list_by_order(order_pk).await?;
    Ok(Json(notes))
}

/// GET /api/credit-notes/:id/receipt - 获取退款凭证小票 ESC/POS 字节
pub async fn get_receipt(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Vec<u8>> {
    let service = credit_note_service(&state)?;
    let detail = service
        .get_detail(id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Credit note {} not found", id)))?;

    let renderer = crate::printing::CreditNoteReceiptRenderer::new(48, state.config.timezone);
    let bytes = renderer.render(&detail);
    Ok(bytes)
}

/// GET /api/credit-notes/refundable/:order_pk - 获取可退金额信息
pub async fn get_refundable_info(
    State(state): State<ServerState>,
    Path(order_pk): Path<i64>,
) -> AppResult<Json<RefundableInfo>> {
    let service = credit_note_service(&state)?;
    let info = service.get_refundable_info(order_pk).await?;
    Ok(Json(info))
}
