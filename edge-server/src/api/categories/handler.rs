//! Category API Handlers

use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};

use crate::core::ServerState;
use crate::db::models::{CategoryCreate, CategoryUpdate};
use crate::db::repository::{AttributeRepository, CategoryRepository};
use crate::utils::{AppError, AppResult};
use shared::models::Attribute as SharedAttribute;
use shared::models::Category as SharedCategory;
use shared::models::AttributeBinding as SharedAttributeBinding;

const RESOURCE: &str = "category";

/// GET /api/categories - 获取所有分类
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<SharedCategory>>> {
    let repo = CategoryRepository::new(state.db.clone());
    let categories = repo
        .find_all()
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(categories.into_iter().map(|c| c.into()).collect()))
}

/// GET /api/categories/:id - 获取单个分类
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<SharedCategory>> {
    let repo = CategoryRepository::new(state.db.clone());
    let category = repo
        .find_by_id(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Category {} not found", id)))?;
    Ok(Json(category.into()))
}

/// POST /api/categories - 创建分类
pub async fn create(
    State(state): State<ServerState>,
    Json(payload): Json<CategoryCreate>,
) -> AppResult<Json<SharedCategory>> {
    let repo = CategoryRepository::new(state.db.clone());
    let category = repo
        .create(payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    let id = category
        .id
        .as_ref()
        .map(|t| t.id.to_string())
        .unwrap_or_default();
    let api_category: SharedCategory = category.into();
    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&api_category))
        .await;

    Ok(Json(api_category))
}

/// PUT /api/categories/:id - 更新分类
pub async fn update(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<CategoryUpdate>,
) -> AppResult<Json<SharedCategory>> {
    let repo = CategoryRepository::new(state.db.clone());
    let category = repo
        .update(&id, payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    let api_category: SharedCategory = category.into();
    state
        .broadcast_sync(RESOURCE, "updated", &id, Some(&api_category))
        .await;

    Ok(Json(api_category))
}

/// DELETE /api/categories/:id - 删除分类 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    tracing::info!(id = %id, "Deleting category");
    let repo = CategoryRepository::new(state.db.clone());
    let result = repo
        .delete(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    tracing::info!(id = %id, result = %result, "Category delete result");

    // 广播同步通知
    if result {
        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id, None)
            .await;
    } else {
        tracing::warn!(id = %id, "Category delete returned false, not broadcasting");
    }

    Ok(Json(result))
}

// =========================================================================
// Batch Sort Order Update
// =========================================================================

/// Payload for batch sort order update
#[derive(Debug, Deserialize)]
pub struct SortOrderUpdate {
    pub id: String,
    pub sort_order: i32,
}

/// Response for batch update operation
#[derive(Debug, Serialize)]
pub struct BatchUpdateResponse {
    pub updated: usize,
}

/// PUT /api/categories/sort-order - Batch update sort orders
pub async fn batch_update_sort_order(
    State(state): State<ServerState>,
    Json(updates): Json<Vec<SortOrderUpdate>>,
) -> AppResult<Json<BatchUpdateResponse>> {
    tracing::info!(
        count = updates.len(),
        "Batch update sort order request received"
    );

    let repo = CategoryRepository::new(state.db.clone());
    let mut updated_count = 0;

    for update in &updates {
        tracing::debug!(id = %update.id, sort_order = update.sort_order, "Updating category sort order");

        // Use existing update method with just sort_order
        let result = repo
            .update(
                &update.id,
                CategoryUpdate {
                    name: None,
                    sort_order: Some(update.sort_order),
                    print_destinations: None,
                    is_kitchen_print_enabled: None,
                    is_label_print_enabled: None,
                    is_active: None,
                    is_virtual: None,
                    tag_ids: None,
                    match_mode: None,
                },
            )
            .await;

        match &result {
            Ok(_) => {
                tracing::debug!(id = %update.id, "Category sort order updated successfully");
                updated_count += 1;
            }
            Err(e) => {
                tracing::error!(id = %update.id, error = %e, "Failed to update category sort order");
            }
        }
    }

    tracing::info!(
        updated = updated_count,
        total = updates.len(),
        "Batch update sort order completed"
    );

    // 广播同步通知
    state
        .broadcast_sync::<()>(RESOURCE, "updated", "batch", None)
        .await;

    Ok(Json(BatchUpdateResponse {
        updated: updated_count,
    }))
}

// =========================================================================
// Category-Attribute Binding
// =========================================================================

/// Payload for binding attribute to category
#[derive(Debug, Deserialize)]
pub struct BindAttributePayload {
    pub is_required: Option<bool>,
    pub display_order: Option<i32>,
    pub default_option_idx: Option<i32>,
}

/// GET /api/categories/:id/attributes - 获取分类关联的属性
pub async fn list_category_attributes(
    State(state): State<ServerState>,
    Path(category_id): Path<String>,
) -> AppResult<Json<Vec<SharedAttribute>>> {
    let repo = AttributeRepository::new(state.db.clone());
    let attributes = repo
        .find_by_category(&category_id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(attributes.into_iter().map(Into::into).collect()))
}

/// POST /api/categories/:id/attributes/:attr_id - 绑定属性到分类
pub async fn bind_category_attribute(
    State(state): State<ServerState>,
    Path((category_id, attr_id)): Path<(String, String)>,
    Json(payload): Json<BindAttributePayload>,
) -> AppResult<Json<SharedAttributeBinding>> {
    let repo = AttributeRepository::new(state.db.clone());
    let binding = repo
        .link_to_category(
            &category_id,
            &attr_id,
            payload.is_required.unwrap_or(false),
            payload.display_order.unwrap_or(0),
            payload.default_option_idx,
        )
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    state
        .broadcast_sync(
            "category_attribute",
            "created",
            &format!("{}:{}", category_id, attr_id),
            Some(&binding),
        )
        .await;

    Ok(Json(binding.into()))
}

/// DELETE /api/categories/:id/attributes/:attr_id - 解绑属性与分类
pub async fn unbind_category_attribute(
    State(state): State<ServerState>,
    Path((category_id, attr_id)): Path<(String, String)>,
) -> AppResult<Json<bool>> {
    let repo = AttributeRepository::new(state.db.clone());
    let deleted = repo
        .unlink_from_category(&category_id, &attr_id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    if deleted {
        state
            .broadcast_sync::<()>(
                "category_attribute",
                "deleted",
                &format!("{}:{}", category_id, attr_id),
                None,
            )
            .await;
    }

    Ok(Json(deleted))
}
