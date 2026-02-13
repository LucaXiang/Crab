//! Category API Handlers

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::audit::{create_diff, create_snapshot, AuditAction};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::attribute;
use crate::utils::{AppError, AppResult};
use crate::utils::validation::{validate_required_text, validate_optional_text, MAX_NAME_LEN};
use shared::models::{Attribute, AttributeBinding, Category, CategoryCreate, CategoryUpdate};

const RESOURCE: &str = "category";

fn validate_create(payload: &CategoryCreate) -> AppResult<()> {
    validate_required_text(&payload.name, "name", MAX_NAME_LEN)?;
    validate_optional_text(&payload.match_mode, "match_mode", MAX_NAME_LEN)?;
    Ok(())
}

fn validate_update(payload: &CategoryUpdate) -> AppResult<()> {
    if let Some(name) = &payload.name {
        validate_required_text(name, "name", MAX_NAME_LEN)?;
    }
    validate_optional_text(&payload.match_mode, "match_mode", MAX_NAME_LEN)?;
    Ok(())
}

/// GET /api/categories - 获取所有分类
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<Category>>> {
    let categories = state.catalog_service.list_categories();
    Ok(Json(categories))
}

/// GET /api/categories/:id - 获取单个分类
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Category>> {
    let category = state
        .catalog_service
        .get_category(id)
        .ok_or_else(|| AppError::not_found(format!("Category {} not found", id)))?;
    Ok(Json(category))
}

/// POST /api/categories - 创建分类
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<CategoryCreate>,
) -> AppResult<Json<Category>> {
    validate_create(&payload)?;

    let category = state
        .catalog_service
        .create_category(payload)
        .await
        ?;

    let id = category.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::CategoryCreated,
        "category", &id,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&category, "category")
    );

    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&category))
        .await;

    Ok(Json(category))
}

/// PUT /api/categories/:id - 更新分类
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(payload): Json<CategoryUpdate>,
) -> AppResult<Json<Category>> {
    validate_update(&payload)?;

    let id_str = id.to_string();

    // 查询旧值（用于审计 diff）
    let old_category = state
        .catalog_service
        .get_category(id)
        .ok_or_else(|| AppError::not_found(format!("Category {}", id)))?;

    let category = state
        .catalog_service
        .update_category(id, payload)
        .await?;

    audit_log!(
        state.audit_service,
        AuditAction::CategoryUpdated,
        "category", &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_category, &category, "category")
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id_str, Some(&category))
        .await;

    Ok(Json(category))
}

/// DELETE /api/categories/:id - 删除分类
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<bool>> {
    let id_str = id.to_string();
    tracing::info!(id = %id, "Deleting category");

    let name_for_audit = state.catalog_service.get_category(id)
        .map(|c| c.name.clone()).unwrap_or_default();
    state
        .catalog_service
        .delete_category(id)
        .await
        ?;

    audit_log!(
        state.audit_service,
        AuditAction::CategoryDeleted,
        "category", &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"name": name_for_audit})
    );

    state
        .broadcast_sync::<()>(RESOURCE, "deleted", &id_str, None)
        .await;

    Ok(Json(true))
}

// =========================================================================
// Batch Sort Order Update
// =========================================================================

/// Payload for batch sort order update
#[derive(Debug, Deserialize)]
pub struct SortOrderUpdate {
    pub id: i64,
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

    let mut updated_count = 0;

    for update in &updates {
        tracing::debug!(
            id = %update.id,
            sort_order = update.sort_order,
            "Updating category sort order"
        );

        let result = state
            .catalog_service
            .update_category(
                update.id,
                CategoryUpdate {
                    name: None,
                    sort_order: Some(update.sort_order),
                    kitchen_print_destinations: None,
                    label_print_destinations: None,
                    is_kitchen_print_enabled: None,
                    is_label_print_enabled: None,
                    is_virtual: None,
                    tag_ids: None,
                    match_mode: None,
                    is_display: None,
                    is_active: None,
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
    pub default_option_ids: Option<Vec<i32>>,
}

/// GET /api/categories/:id/attributes - 获取分类关联的属性
pub async fn list_category_attributes(
    State(state): State<ServerState>,
    Path(category_id): Path<i64>,
) -> AppResult<Json<Vec<Attribute>>> {
    let bindings = attribute::find_bindings_for_owner(&state.pool, "category", category_id).await?;
    let attributes: Vec<Attribute> = bindings.into_iter().map(|(_, attr)| attr).collect();
    Ok(Json(attributes))
}

/// POST /api/categories/:id/attributes/:attr_id - 绑定属性到分类
pub async fn bind_category_attribute(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path((category_id, attr_id)): Path<(i64, i64)>,
    Json(payload): Json<BindAttributePayload>,
) -> AppResult<Json<AttributeBinding>> {
    let binding = attribute::link(
        &state.pool,
        "category",
        category_id,
        attr_id,
        payload.is_required.unwrap_or(false),
        payload.display_order.unwrap_or(0),
        payload.default_option_ids,
    )
    .await?;

    let category_id_str = category_id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::CategoryUpdated,
        "category", &category_id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"op": "bind_attribute", "attribute_id": attr_id})
    );

    // Refresh product cache for this category (inherited attributes changed)
    if let Err(e) = state.catalog_service.refresh_products_in_category(category_id).await {
        tracing::warn!("Failed to refresh products in category {}: {}", category_id, e);
    }

    // 广播同步通知
    state
        .broadcast_sync(
            "category_attribute",
            "created",
            &format!("{}:{}", category_id, attr_id),
            Some(&binding),
        )
        .await;

    Ok(Json(binding))
}

/// DELETE /api/categories/:id/attributes/:attr_id - 解绑属性与分类
pub async fn unbind_category_attribute(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path((category_id, attr_id)): Path<(i64, i64)>,
) -> AppResult<Json<bool>> {
    let deleted = attribute::unlink(&state.pool, "category", category_id, attr_id).await?;

    if deleted {
        let category_id_str = category_id.to_string();

        audit_log!(
            state.audit_service,
            AuditAction::CategoryUpdated,
            "category", &category_id_str,
            operator_id = Some(current_user.id),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({"op": "unbind_attribute", "attribute_id": attr_id})
        );

        // Refresh product cache for this category (inherited attributes changed)
        if let Err(e) = state.catalog_service.refresh_products_in_category(category_id).await {
            tracing::warn!("Failed to refresh products in category {}: {}", category_id, e);
        }

        // 广播同步通知
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
