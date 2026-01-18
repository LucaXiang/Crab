//! Product API Handlers

use axum::{
    Json,
    extract::{Path, State},
};

use crate::core::ServerState;
use crate::db::models::{
    Product, ProductCreate, ProductUpdate,
    ProductSpecification, ProductSpecificationCreate, ProductSpecificationUpdate,
};
use crate::db::repository::{ProductRepository, ProductSpecificationRepository};
use crate::utils::{AppError, AppResult};

const RESOURCE_PRODUCT: &str = "product";
const RESOURCE_SPEC: &str = "product_specification";

// =============================================================================
// Product Handlers
// =============================================================================

/// GET /api/products - 获取所有商品
pub async fn list(
    State(state): State<ServerState>,
) -> AppResult<Json<Vec<Product>>> {
    let repo = ProductRepository::new(state.db.clone());
    let products = repo.find_all().await.map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(products))
}

/// GET /api/products/by-category/:category_id - 按分类获取商品
pub async fn list_by_category(
    State(state): State<ServerState>,
    Path(category_id): Path<String>,
) -> AppResult<Json<Vec<Product>>> {
    let repo = ProductRepository::new(state.db.clone());
    let products = repo.find_by_category(&category_id).await.map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(products))
}

/// GET /api/products/:id - 获取单个商品
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<Product>> {
    let repo = ProductRepository::new(state.db.clone());
    let product = repo
        .find_by_id(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Product {} not found", id)))?;
    Ok(Json(product))
}

/// GET /api/products/:id/full - 获取商品完整信息 (含关联数据)
pub async fn get_full(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<Product>> {
    let repo = ProductRepository::new(state.db.clone());
    let product = repo
        .find_by_id_full(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Product {} not found", id)))?;
    Ok(Json(product))
}

/// GET /api/products/:id/specs - 获取商品的所有规格
pub async fn get_specs(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<(Product, Vec<ProductSpecification>)>> {
    let repo = ProductRepository::new(state.db.clone());
    let result = repo.find_with_specs(&id).await.map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(result))
}

/// POST /api/products - 创建商品
pub async fn create(
    State(state): State<ServerState>,
    Json(payload): Json<ProductCreate>,
) -> AppResult<Json<Product>> {
    let repo = ProductRepository::new(state.db.clone());
    let product = repo.create(payload).await.map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    let id = product.id.as_ref().map(|t| t.id.to_string()).unwrap_or_default();
    state.broadcast_sync(RESOURCE_PRODUCT, 1, "created", &id, Some(&product)).await;

    Ok(Json(product))
}

/// PUT /api/products/:id - 更新商品
pub async fn update(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<ProductUpdate>,
) -> AppResult<Json<Product>> {
    let repo = ProductRepository::new(state.db.clone());
    let product = repo.update(&id, payload).await.map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    state.broadcast_sync(RESOURCE_PRODUCT, 1, "updated", &id, Some(&product)).await;

    Ok(Json(product))
}

/// DELETE /api/products/:id - 删除商品 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = ProductRepository::new(state.db.clone());
    let result = repo.delete(&id).await.map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    if result {
        state.broadcast_sync::<()>(RESOURCE_PRODUCT, 1, "deleted", &id, None).await;
    }

    Ok(Json(result))
}

// =============================================================================
// ProductSpecification Handlers
// =============================================================================

/// GET /api/specs/:id - 获取单个规格
pub async fn get_spec(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<ProductSpecification>> {
    let repo = ProductSpecificationRepository::new(state.db.clone());
    let spec = repo
        .find_by_id_with_tags(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Specification {} not found", id)))?;
    Ok(Json(spec))
}

/// POST /api/specs - 创建规格
pub async fn create_spec(
    State(state): State<ServerState>,
    Json(payload): Json<ProductSpecificationCreate>,
) -> AppResult<Json<ProductSpecification>> {
    let repo = ProductSpecificationRepository::new(state.db.clone());
    let spec = repo.create(payload).await.map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    let id = spec.id.as_ref().map(|t| t.id.to_string()).unwrap_or_default();
    state.broadcast_sync(RESOURCE_SPEC, 1, "created", &id, Some(&spec)).await;

    Ok(Json(spec))
}

/// PUT /api/specs/:id - 更新规格
pub async fn update_spec(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<ProductSpecificationUpdate>,
) -> AppResult<Json<ProductSpecification>> {
    let repo = ProductSpecificationRepository::new(state.db.clone());
    let spec = repo.update(&id, payload).await.map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    state.broadcast_sync(RESOURCE_SPEC, 1, "updated", &id, Some(&spec)).await;

    Ok(Json(spec))
}

/// DELETE /api/specs/:id - 删除规格 (软删除)
pub async fn delete_spec(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = ProductSpecificationRepository::new(state.db.clone());
    let result = repo.delete(&id).await.map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    if result {
        state.broadcast_sync::<()>(RESOURCE_SPEC, 1, "deleted", &id, None).await;
    }

    Ok(Json(result))
}

/// POST /api/specs/:id/tags/:tag_id - 给规格添加标签
pub async fn add_tag(
    State(state): State<ServerState>,
    Path((spec_id, tag_id)): Path<(String, String)>,
) -> AppResult<Json<ProductSpecification>> {
    let repo = ProductSpecificationRepository::new(state.db.clone());
    let spec = repo.add_tag(&spec_id, &tag_id).await.map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    state.broadcast_sync(RESOURCE_SPEC, 1, "updated", &spec_id, Some(&spec)).await;

    Ok(Json(spec))
}

/// DELETE /api/specs/:id/tags/:tag_id - 从规格移除标签
pub async fn remove_tag(
    State(state): State<ServerState>,
    Path((spec_id, tag_id)): Path<(String, String)>,
) -> AppResult<Json<ProductSpecification>> {
    let repo = ProductSpecificationRepository::new(state.db.clone());
    let spec = repo.remove_tag(&spec_id, &tag_id).await.map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    state.broadcast_sync(RESOURCE_SPEC, 1, "updated", &spec_id, Some(&spec)).await;

    Ok(Json(spec))
}
