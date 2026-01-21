//! Tag API Handlers

use axum::{
    Json,
    extract::{Path, State},
};

use crate::core::ServerState;
use crate::db::models::{Tag, TagCreate, TagUpdate};
use crate::db::repository::TagRepository;
use crate::utils::{AppError, AppResult};

const RESOURCE: &str = "tag";

/// GET /api/tags - 获取所有标签
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<Tag>>> {
    let repo = TagRepository::new(state.db.clone());
    let tags = repo
        .find_all()
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(tags))
}

/// GET /api/tags/:id - 获取单个标签
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<Tag>> {
    let repo = TagRepository::new(state.db.clone());
    let tag = repo
        .find_by_id(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Tag {} not found", id)))?;
    Ok(Json(tag))
}

/// POST /api/tags - 创建标签
pub async fn create(
    State(state): State<ServerState>,
    Json(payload): Json<TagCreate>,
) -> AppResult<Json<Tag>> {
    let repo = TagRepository::new(state.db.clone());
    let tag = repo
        .create(payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    let id = tag
        .id
        .as_ref()
        .map(|t| t.id.to_string())
        .unwrap_or_default();
    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&tag))
        .await;

    Ok(Json(tag))
}

/// PUT /api/tags/:id - 更新标签
pub async fn update(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<TagUpdate>,
) -> AppResult<Json<Tag>> {
    let repo = TagRepository::new(state.db.clone());
    let tag = repo
        .update(&id, payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    state
        .broadcast_sync(RESOURCE, "updated", &id, Some(&tag))
        .await;

    Ok(Json(tag))
}

/// DELETE /api/tags/:id - 删除标签 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = TagRepository::new(state.db.clone());
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
