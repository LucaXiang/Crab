//! Print Destination API Handlers

use axum::{
    Json,
    extract::{Path, State},
};

use crate::core::ServerState;
use crate::db::models::{PrintDestination, PrintDestinationCreate, PrintDestinationUpdate};
use crate::db::repository::PrintDestinationRepository;
use crate::utils::{AppError, AppResult};

const RESOURCE: &str = "print_destination";

/// GET /api/print-destinations - 获取所有打印目的地
pub async fn list(
    State(state): State<ServerState>,
) -> AppResult<Json<Vec<PrintDestination>>> {
    let repo = PrintDestinationRepository::new(state.db.clone());
    let items = repo
        .find_all()
        .await
        ?;
    Ok(Json(items))
}

/// GET /api/print-destinations/:id - 获取单个打印目的地
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<PrintDestination>> {
    let repo = PrintDestinationRepository::new(state.db.clone());
    let item = repo
        .find_by_id(&id)
        .await
        ?
        .ok_or_else(|| AppError::not_found(format!("Print destination {} not found", id)))?;
    Ok(Json(item))
}

/// POST /api/print-destinations - 创建打印目的地
pub async fn create(
    State(state): State<ServerState>,
    Json(payload): Json<PrintDestinationCreate>,
) -> AppResult<Json<PrintDestination>> {
    let repo = PrintDestinationRepository::new(state.db.clone());
    let item = repo
        .create(payload)
        .await
        ?;

    // 广播同步通知
    let id = item.id.as_ref().map(|id| id.to_string()).unwrap_or_default();
    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&item))
        .await;

    Ok(Json(item))
}

/// PUT /api/print-destinations/:id - 更新打印目的地
pub async fn update(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<PrintDestinationUpdate>,
) -> AppResult<Json<PrintDestination>> {
    let repo = PrintDestinationRepository::new(state.db.clone());
    let item = repo
        .update(&id, payload)
        .await
        ?;

    // 广播同步通知
    state
        .broadcast_sync(RESOURCE, "updated", &id, Some(&item))
        .await;

    Ok(Json(item))
}

/// DELETE /api/print-destinations/:id - 删除打印目的地
pub async fn delete(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    tracing::info!(id = %id, "Deleting print destination");
    let repo = PrintDestinationRepository::new(state.db.clone());
    let result = repo
        .delete(&id)
        .await
        ?;

    tracing::info!(id = %id, result = %result, "Print destination delete result");

    // 广播同步通知
    if result {
        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id, None)
            .await;
    } else {
        tracing::warn!(id = %id, "Print destination delete returned false, not broadcasting");
    }

    Ok(Json(result))
}
