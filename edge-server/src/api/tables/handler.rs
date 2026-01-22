//! Dining Table API Handlers

use axum::{
    Json,
    extract::{Path, State},
};

use crate::core::ServerState;
use crate::db::models::{DiningTableCreate, DiningTableUpdate};
use crate::db::repository::DiningTableRepository;
use crate::utils::{AppError, AppResult};
use shared::models::DiningTable as SharedDiningTable;

const RESOURCE: &str = "dining_table";

/// GET /api/tables - 获取所有桌台
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<SharedDiningTable>>> {
    let repo = DiningTableRepository::new(state.db.clone());
    let tables = repo
        .find_all()
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(tables.into_iter().map(Into::into).collect()))
}

/// GET /api/tables/:id - 获取单个桌台
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<SharedDiningTable>> {
    let repo = DiningTableRepository::new(state.db.clone());
    let table = repo
        .find_by_id_with_zone(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Table {} not found", id)))?;
    Ok(Json(table.into()))
}

/// POST /api/tables - 创建桌台
pub async fn create(
    State(state): State<ServerState>,
    Json(payload): Json<DiningTableCreate>,
) -> AppResult<Json<SharedDiningTable>> {
    let repo = DiningTableRepository::new(state.db.clone());
    let table = repo
        .create(payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    let id = table
        .id
        .as_ref()
        .map(|t| t.id.to_string())
        .unwrap_or_default();
    let api_table: SharedDiningTable = table.into();
    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&api_table))
        .await;

    Ok(Json(api_table))
}

/// PUT /api/tables/:id - 更新桌台
pub async fn update(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<DiningTableUpdate>,
) -> AppResult<Json<SharedDiningTable>> {
    let repo = DiningTableRepository::new(state.db.clone());
    let table = repo
        .update(&id, payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    let api_table: SharedDiningTable = table.into();
    state
        .broadcast_sync(RESOURCE, "updated", &id, Some(&api_table))
        .await;

    Ok(Json(api_table))
}

/// DELETE /api/tables/:id - 删除桌台 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = DiningTableRepository::new(state.db.clone());
    let result = repo
        .delete(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    if result {
        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id, None)
            .await;
    }

    Ok(Json(result))
}
