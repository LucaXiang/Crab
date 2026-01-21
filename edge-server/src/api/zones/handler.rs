//! Zone API Handlers

use axum::{
    Json,
    extract::{Path, State},
};

use crate::core::ServerState;
use crate::db::models::{DiningTable, Zone, ZoneCreate, ZoneUpdate};
use crate::db::repository::{DiningTableRepository, ZoneRepository};
use crate::utils::{AppError, AppResult};

const RESOURCE: &str = "zone";

/// GET /api/zones - 获取所有区域
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<Zone>>> {
    let repo = ZoneRepository::new(state.db.clone());
    let zones = repo
        .find_all()
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(zones))
}

/// GET /api/zones/:id - 获取单个区域
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<Zone>> {
    let repo = ZoneRepository::new(state.db.clone());
    let zone = repo
        .find_by_id(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Zone {} not found", id)))?;
    Ok(Json(zone))
}

/// POST /api/zones - 创建区域
pub async fn create(
    State(state): State<ServerState>,
    Json(payload): Json<ZoneCreate>,
) -> AppResult<Json<Zone>> {
    let repo = ZoneRepository::new(state.db.clone());
    let zone = repo
        .create(payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    let id = zone
        .id
        .as_ref()
        .map(|t| t.id.to_string())
        .unwrap_or_default();
    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&zone))
        .await;

    Ok(Json(zone))
}

/// PUT /api/zones/:id - 更新区域
pub async fn update(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<ZoneUpdate>,
) -> AppResult<Json<Zone>> {
    let repo = ZoneRepository::new(state.db.clone());
    let zone = repo
        .update(&id, payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    state
        .broadcast_sync(RESOURCE, "updated", &id, Some(&zone))
        .await;

    Ok(Json(zone))
}

/// DELETE /api/zones/:id - 删除区域 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = ZoneRepository::new(state.db.clone());
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

/// GET /api/zones/:id/tables - 获取区域内的所有桌台
pub async fn list_tables(
    State(state): State<ServerState>,
    Path(zone_id): Path<String>,
) -> AppResult<Json<Vec<DiningTable>>> {
    let repo = DiningTableRepository::new(state.db.clone());
    let tables = repo
        .find_by_zone(&zone_id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(tables))
}
