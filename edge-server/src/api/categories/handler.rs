//! Category API Handlers

use axum::{
    Json,
    extract::{Path, State},
};

use crate::core::ServerState;
use crate::db::models::{Category, CategoryCreate, CategoryUpdate};
use crate::db::repository::CategoryRepository;
use crate::utils::{AppError, AppResult};

const RESOURCE: &str = "category";

/// GET /api/categories - 获取所有分类
pub async fn list(
    State(state): State<ServerState>,
) -> AppResult<Json<Vec<Category>>> {
    let repo = CategoryRepository::new(state.db.clone());
    let categories = repo.find_all().await.map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(categories))
}

/// GET /api/categories/:id - 获取单个分类
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<Category>> {
    let repo = CategoryRepository::new(state.db.clone());
    let category = repo
        .find_by_id(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Category {} not found", id)))?;
    Ok(Json(category))
}

/// POST /api/categories - 创建分类
pub async fn create(
    State(state): State<ServerState>,
    Json(payload): Json<CategoryCreate>,
) -> AppResult<Json<Category>> {
    let repo = CategoryRepository::new(state.db.clone());
    let category = repo.create(payload).await.map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    let id = category.id.as_ref().map(|t| t.id.to_string());
    state.broadcast_sync(RESOURCE, id.as_deref(), "created", Some(&category)).await;

    Ok(Json(category))
}

/// PUT /api/categories/:id - 更新分类
pub async fn update(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<CategoryUpdate>,
) -> AppResult<Json<Category>> {
    let repo = CategoryRepository::new(state.db.clone());
    let category = repo.update(&id, payload).await.map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    state.broadcast_sync(RESOURCE, Some(&id), "updated", Some(&category)).await;

    Ok(Json(category))
}

/// DELETE /api/categories/:id - 删除分类 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = CategoryRepository::new(state.db.clone());
    let result = repo.delete(&id).await.map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    if result {
        state.broadcast_sync::<()>(RESOURCE, Some(&id), "deleted", None).await;
    }

    Ok(Json(result))
}
