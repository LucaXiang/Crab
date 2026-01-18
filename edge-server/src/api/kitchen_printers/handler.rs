//! Kitchen Printer API Handlers

use axum::{
    Json,
    extract::{Path, State},
};

use crate::core::ServerState;
use crate::db::models::{KitchenPrinter, KitchenPrinterCreate, KitchenPrinterUpdate};
use crate::db::repository::KitchenPrinterRepository;
use crate::utils::{AppError, AppResult};

const RESOURCE: &str = "kitchen_printer";

/// GET /api/kitchen-printers - 获取所有厨打
pub async fn list(
    State(state): State<ServerState>,
) -> AppResult<Json<Vec<KitchenPrinter>>> {
    let repo = KitchenPrinterRepository::new(state.db.clone());
    let printers = repo.find_all().await.map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(printers))
}

/// GET /api/kitchen-printers/:id - 获取单个厨打
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<KitchenPrinter>> {
    let repo = KitchenPrinterRepository::new(state.db.clone());
    let printer = repo
        .find_by_id(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Kitchen printer {} not found", id)))?;
    Ok(Json(printer))
}

/// POST /api/kitchen-printers - 创建厨打
pub async fn create(
    State(state): State<ServerState>,
    Json(payload): Json<KitchenPrinterCreate>,
) -> AppResult<Json<KitchenPrinter>> {
    let repo = KitchenPrinterRepository::new(state.db.clone());
    let printer = repo.create(payload).await.map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    let id = printer.id.as_ref().map(|t| t.id.to_string()).unwrap_or_default();
    state.broadcast_sync(RESOURCE, 1, "created", &id, Some(&printer)).await;

    Ok(Json(printer))
}

/// PUT /api/kitchen-printers/:id - 更新厨打
pub async fn update(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<KitchenPrinterUpdate>,
) -> AppResult<Json<KitchenPrinter>> {
    let repo = KitchenPrinterRepository::new(state.db.clone());
    let printer = repo.update(&id, payload).await.map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    state.broadcast_sync(RESOURCE, 1, "updated", &id, Some(&printer)).await;

    Ok(Json(printer))
}

/// DELETE /api/kitchen-printers/:id - 删除厨打 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = KitchenPrinterRepository::new(state.db.clone());
    let result = repo.delete(&id).await.map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    if result {
        state.broadcast_sync::<()>(RESOURCE, 1, "deleted", &id, None).await;
    }

    Ok(Json(result))
}
