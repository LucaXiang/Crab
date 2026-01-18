//! Dining Table API Handlers

use axum::{
    Json,
    extract::{Path, State},
};

use crate::core::ServerState;
use crate::db::models::{DiningTable, DiningTableCreate, DiningTableUpdate};
use crate::db::repository::DiningTableRepository;
use crate::utils::{AppError, AppResult};

const RESOURCE: &str = "dining_table";

/// GET /api/tables - 获取所有桌台
pub async fn list(
    State(state): State<ServerState>,
) -> AppResult<Json<Vec<DiningTable>>> {
    let repo = DiningTableRepository::new(state.db.clone());
    let tables = repo.find_all().await.map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(tables))
}

/// GET /api/tables/:id - 获取单个桌台
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<DiningTable>> {
    let repo = DiningTableRepository::new(state.db.clone());
    let table = repo
        .find_by_id_with_zone(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Table {} not found", id)))?;
    Ok(Json(table))
}

/// POST /api/tables - 创建桌台
pub async fn create(
    State(state): State<ServerState>,
    Json(payload): Json<DiningTableCreate>,
) -> AppResult<Json<DiningTable>> {
    let repo = DiningTableRepository::new(state.db.clone());
    let table = repo.create(payload).await.map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    let id = table.id.as_ref().map(|t| t.id.to_string());
    state.broadcast_sync(RESOURCE, id.as_deref(), "created", Some(&table)).await;

    Ok(Json(table))
}

/// PUT /api/tables/:id - 更新桌台
pub async fn update(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<DiningTableUpdate>,
) -> AppResult<Json<DiningTable>> {
    let repo = DiningTableRepository::new(state.db.clone());
    let table = repo.update(&id, payload).await.map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    state.broadcast_sync(RESOURCE, Some(&id), "updated", Some(&table)).await;

    Ok(Json(table))
}

/// DELETE /api/tables/:id - 删除桌台 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = DiningTableRepository::new(state.db.clone());
    let result = repo.delete(&id).await.map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    if result {
        state.broadcast_sync::<()>(RESOURCE, Some(&id), "deleted", None).await;
    }

    Ok(Json(result))
}
