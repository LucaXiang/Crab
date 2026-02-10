//! Product API Handlers

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde::Deserialize;

use crate::audit::{create_diff, create_snapshot, AuditAction};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::attribute;
use crate::utils::{AppError, AppResult, ErrorCode};
use shared::models::{AttributeBindingFull, ProductCreate, ProductFull, ProductUpdate};

const RESOURCE_PRODUCT: &str = "product";

/// 检查 external_id 是否已被其他商品使用
async fn check_duplicate_external_id(
    state: &ServerState,
    external_id: i64,
    exclude_product_id: Option<i64>,
) -> AppResult<bool> {
    let count: i64 = if let Some(exclude_id) = exclude_product_id {
        sqlx::query_scalar!(
            "SELECT COUNT(*) FROM product WHERE external_id = ?1 AND id != ?2 LIMIT 1",
            external_id, exclude_id,
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
    } else {
        sqlx::query_scalar!(
            "SELECT COUNT(*) FROM product WHERE external_id = ?1 LIMIT 1",
            external_id,
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
    };
    Ok(count > 0)
}

// =============================================================================
// Product Handlers
// =============================================================================

/// GET /api/products - 获取所有商品 (完整数据，含属性和标签)
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<ProductFull>>> {
    let products = state.catalog_service.list_products();
    Ok(Json(products))
}

/// GET /api/products/by-category/:category_id - 按分类获取商品 (完整数据)
pub async fn list_by_category(
    State(state): State<ServerState>,
    Path(category_id): Path<i64>,
) -> AppResult<Json<Vec<ProductFull>>> {
    let products = state
        .catalog_service
        .get_products_by_category(category_id);
    Ok(Json(products))
}

/// GET /api/products/:id - 获取单个商品 (完整数据)
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ProductFull>> {
    let product = state
        .catalog_service
        .get_product(id)
        .ok_or_else(|| AppError::not_found(format!("Product {}", id)))?;
    Ok(Json(product))
}

/// GET /api/products/:id/full - 获取商品完整信息 (含规格、属性、标签)
/// Note: Now same as get_by_id since CatalogService always returns ProductFull
pub async fn get_full(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ProductFull>> {
    let product = state
        .catalog_service
        .get_product(id)
        .ok_or_else(|| AppError::not_found(format!("Product {}", id)))?;
    Ok(Json(product))
}

/// POST /api/products - 创建商品
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<ProductCreate>,
) -> AppResult<Json<ProductFull>> {
    // 检查 external_id 是否已提供 (必填)
    let eid = payload.external_id.ok_or_else(|| {
        AppError::new(ErrorCode::ProductExternalIdRequired)
    })?;

    // 检查 external_id 是否已被其他商品使用
    if check_duplicate_external_id(&state, eid, None).await? {
        return Err(AppError::new(ErrorCode::ProductExternalIdExists)
            .with_detail("external_id", eid));
    }

    let product = state
        .catalog_service
        .create_product(payload)
        .await
        ?;

    let id_str = product.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::ProductCreated,
        "product", &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&product, "product")
    );

    state
        .broadcast_sync(RESOURCE_PRODUCT, "created", &id_str, Some(&product))
        .await;

    Ok(Json(product))
}

/// PUT /api/products/:id - 更新商品
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(payload): Json<ProductUpdate>,
) -> AppResult<Json<ProductFull>> {
    let id_str = id.to_string();

    tracing::debug!(
        "Product update - id: {}, tax_rate: {:?}, is_kitchen_print_enabled: {:?}",
        id,
        payload.tax_rate,
        payload.is_kitchen_print_enabled
    );

    // 查询旧值（用于审计 diff）
    let old_product = state
        .catalog_service
        .get_product(id)
        .ok_or_else(|| AppError::not_found(format!("Product {}", id)))?;

    // 检查 external_id 是否已被其他商品使用
    if let Some(eid) = payload.external_id
        && check_duplicate_external_id(&state, eid, Some(id)).await?
    {
        return Err(AppError::new(ErrorCode::ProductExternalIdExists)
            .with_detail("external_id", eid));
    }

    let product = state
        .catalog_service
        .update_product(id, payload)
        .await?;

    tracing::debug!(
        "Product updated - is_kitchen_print_enabled: {}, is_label_print_enabled: {}",
        product.is_kitchen_print_enabled,
        product.is_label_print_enabled
    );

    audit_log!(
        state.audit_service,
        AuditAction::ProductUpdated,
        "product", &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_product, &product, "product")
    );

    state
        .broadcast_sync(RESOURCE_PRODUCT, "updated", &id_str, Some(&product))
        .await;

    Ok(Json(product))
}

/// DELETE /api/products/:id - 删除商品
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<bool>> {
    let id_str = id.to_string();

    // 删除前查名称用于审计
    let name_for_audit = state
        .catalog_service
        .get_product(id)
        .map(|p| p.name.clone())
        .unwrap_or_default();
    state
        .catalog_service
        .delete_product(id)
        .await
        ?;

    audit_log!(
        state.audit_service,
        AuditAction::ProductDeleted,
        "product", &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"name": name_for_audit})
    );

    state
        .broadcast_sync::<()>(RESOURCE_PRODUCT, "deleted", &id_str, None)
        .await;

    Ok(Json(true))
}

/// GET /api/products/:id/attributes - 获取商品的属性绑定列表
pub async fn list_product_attributes(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Vec<AttributeBindingFull>>> {
    // Get attribute bindings for this product
    let bindings = attribute::find_bindings_for_owner(&state.pool, "product", id).await?;

    // Convert to API type
    let result: Vec<AttributeBindingFull> = bindings
        .into_iter()
        .map(|(binding, attr)| AttributeBindingFull {
            id: binding.id,
            attribute: attr,
            is_required: binding.is_required,
            display_order: binding.display_order,
            default_option_ids: binding.default_option_ids,
            is_inherited: false,
        })
        .collect();

    Ok(Json(result))
}

// =============================================================================
// Product Tag Handlers
// =============================================================================

/// POST /api/products/:id/tags/:tag_id - 给商品添加标签
pub async fn add_product_tag(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path((product_id, tag_id)): Path<(i64, i64)>,
) -> AppResult<Json<ProductFull>> {
    let product = state
        .catalog_service
        .add_product_tag(product_id, tag_id)
        .await
        ?;

    let product_id_str = product_id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::ProductUpdated,
        "product", &product_id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"op": "add_tag", "tag_id": tag_id})
    );

    // 广播同步通知 (发送完整 ProductFull 数据)
    state
        .broadcast_sync(
            RESOURCE_PRODUCT,
            "updated",
            &product_id_str,
            Some(&product),
        )
        .await;

    Ok(Json(product))
}

/// DELETE /api/products/:id/tags/:tag_id - 从商品移除标签
pub async fn remove_product_tag(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path((product_id, tag_id)): Path<(i64, i64)>,
) -> AppResult<Json<ProductFull>> {
    let product = state
        .catalog_service
        .remove_product_tag(product_id, tag_id)
        .await
        ?;

    let product_id_str = product_id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::ProductUpdated,
        "product", &product_id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"op": "remove_tag", "tag_id": tag_id})
    );

    // 广播同步通知 (发送完整 ProductFull 数据)
    state
        .broadcast_sync(
            RESOURCE_PRODUCT,
            "updated",
            &product_id_str,
            Some(&product),
        )
        .await;

    Ok(Json(product))
}

// =============================================================================
// Batch Sort Order
// =============================================================================

/// Payload for batch sort order update
#[derive(Debug, Deserialize)]
pub struct SortOrderUpdate {
    pub id: i64,
    pub sort_order: i32,
}

/// Response for batch update operation
#[derive(Debug, serde::Serialize)]
pub struct BatchUpdateResponse {
    pub updated: usize,
}

/// PUT /api/products/sort-order - 批量更新商品排序
pub async fn batch_update_sort_order(
    State(state): State<ServerState>,
    Json(updates): Json<Vec<SortOrderUpdate>>,
) -> AppResult<Json<BatchUpdateResponse>> {
    tracing::info!(
        count = updates.len(),
        "Batch update product sort order request received"
    );

    let mut updated_count = 0;

    for update in &updates {
        tracing::debug!(
            id = %update.id,
            sort_order = update.sort_order,
            "Updating product sort order"
        );

        let result = state
            .catalog_service
            .update_product(
                update.id,
                ProductUpdate {
                    name: None,
                    sort_order: Some(update.sort_order),
                    image: None,
                    category_id: None,
                    tax_rate: None,
                    receipt_name: None,
                    kitchen_print_name: None,
                    is_kitchen_print_enabled: None,
                    is_label_print_enabled: None,
                    is_active: None,
                    external_id: None,
                    specs: None,
                    tags: None,
                },
            )
            .await;

        match &result {
            Ok(_) => {
                tracing::debug!(id = %update.id, "Product sort order updated successfully");
                updated_count += 1;
            }
            Err(e) => {
                tracing::error!(id = %update.id, error = %e, "Failed to update product sort order");
            }
        }
    }

    tracing::info!(
        updated = updated_count,
        total = updates.len(),
        "Batch update product sort order completed"
    );

    // 广播同步通知
    state
        .broadcast_sync::<()>(RESOURCE_PRODUCT, "updated", "batch", None)
        .await;

    Ok(Json(BatchUpdateResponse {
        updated: updated_count,
    }))
}
