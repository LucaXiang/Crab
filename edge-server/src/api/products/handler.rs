//! Product API Handlers

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde::Deserialize;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

use crate::audit::AuditAction;
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::models::{AttributeBindingFull, ProductCreate, ProductFull, ProductUpdate};
use crate::db::repository::AttributeRepository;
use crate::utils::{AppError, AppResult, ErrorCode};

const RESOURCE_PRODUCT: &str = "product";

/// 检查 external_id 是否已存在，返回重复的 ID 列表
async fn check_duplicate_external_ids(
    db: &Surreal<Db>,
    external_ids: &[i64],
    exclude_product_id: Option<&str>,
) -> AppResult<Option<Vec<i64>>> {
    let mut query = if let Some(exclude_id) = exclude_product_id {
        // Strip "product:" prefix if present, since type::thing() will add it
        let exclude_id = exclude_id
            .strip_prefix("product:")
            .unwrap_or(exclude_id)
            .to_string();
        db.query("SELECT external_id FROM product_spec WHERE external_id IN $ids AND product != type::thing('product', $exclude)")
            .bind(("ids", external_ids.to_vec()))
            .bind(("exclude", exclude_id))
            .await
            .map_err(crate::db::repository::surreal_err_to_app)?
    } else {
        db.query("SELECT external_id FROM product_spec WHERE external_id IN $ids")
            .bind(("ids", external_ids.to_vec()))
            .await
            .map_err(crate::db::repository::surreal_err_to_app)?
    };

    #[derive(serde::Deserialize)]
    struct Found {
        external_id: i64,
    }

    let found: Vec<Found> = query.take(0).unwrap_or_default();
    if found.is_empty() {
        Ok(None)
    } else {
        Ok(Some(found.into_iter().map(|f| f.external_id).collect()))
    }
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
    Path(category_id): Path<String>,
) -> AppResult<Json<Vec<ProductFull>>> {
    let products = state.catalog_service.get_products_by_category(&category_id);
    Ok(Json(products))
}

/// GET /api/products/:id - 获取单个商品 (完整数据)
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<ProductFull>> {
    let product = state
        .catalog_service
        .get_product(&id)
        .ok_or_else(|| AppError::not_found(format!("Product {}", id)))?;
    Ok(Json(product))
}

/// GET /api/products/:id/full - 获取商品完整信息 (含规格、属性、标签)
/// Note: Now same as get_by_id since CatalogService always returns ProductFull
pub async fn get_full(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<shared::models::ProductFull>> {
    let product = state
        .catalog_service
        .get_product(&id)
        .ok_or_else(|| AppError::not_found(format!("Product {}", id)))?;
    Ok(Json(product.into()))
}

/// POST /api/products - 创建商品
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<ProductCreate>,
) -> AppResult<Json<shared::models::ProductFull>> {
    // 检查 external_id 是否已存在
    let external_ids: Vec<i64> = payload.specs.iter().filter_map(|s| s.external_id).collect();
    if !external_ids.is_empty()
        && let Some(duplicates) = check_duplicate_external_ids(&state.db, &external_ids, None).await?
    {
        return Err(AppError::new(ErrorCode::SpecExternalIdExists)
            .with_detail("external_ids", duplicates));
    }

    let product = state
        .catalog_service
        .create_product(payload)
        .await
        ?;

    let id = product.id.as_ref().map(|id| id.to_string()).unwrap_or_default();
    let product_for_api: shared::models::ProductFull = product.into();

    audit_log!(
        state.audit_service,
        AuditAction::ProductCreated,
        "product", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"name": &product_for_api.name})
    );

    state
        .broadcast_sync(RESOURCE_PRODUCT, "created", &id, Some(&product_for_api))
        .await;

    Ok(Json(product_for_api))
}

/// PUT /api/products/:id - 更新商品
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
    Json(payload): Json<ProductUpdate>,
) -> AppResult<Json<shared::models::ProductFull>> {
    tracing::debug!(
        "Product update - id: {}, tax_rate: {:?}, is_kitchen_print_enabled: {:?}",
        id,
        payload.tax_rate,
        payload.is_kitchen_print_enabled
    );

    // 检查 external_id 是否已存在 (排除当前产品)
    if let Some(ref specs) = payload.specs {
        let external_ids: Vec<i64> = specs.iter().filter_map(|s| s.external_id).collect();
        if !external_ids.is_empty()
            && let Some(duplicates) =
                check_duplicate_external_ids(&state.db, &external_ids, Some(&id)).await?
        {
            return Err(AppError::new(ErrorCode::SpecExternalIdExists)
                .with_detail("external_ids", duplicates));
        }
    }

    let product = state
        .catalog_service
        .update_product(&id, payload)
        .await
        ?;

    tracing::debug!(
        "Product updated - is_kitchen_print_enabled: {}, is_label_print_enabled: {}",
        product.is_kitchen_print_enabled,
        product.is_label_print_enabled
    );

    let product_for_api: shared::models::ProductFull = product.into();

    audit_log!(
        state.audit_service,
        AuditAction::ProductUpdated,
        "product", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"name": &product_for_api.name})
    );

    state
        .broadcast_sync(RESOURCE_PRODUCT, "updated", &id, Some(&product_for_api))
        .await;

    Ok(Json(product_for_api))
}

/// DELETE /api/products/:id - 删除商品
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    // 删除前查名称用于审计
    let name_for_audit = state.catalog_service.get_product(&id)
        .map(|p| p.name.clone()).unwrap_or_default();
    state
        .catalog_service
        .delete_product(&id)
        .await
        ?;

    audit_log!(
        state.audit_service,
        AuditAction::ProductDeleted,
        "product", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"name": name_for_audit})
    );

    state
        .broadcast_sync::<()>(RESOURCE_PRODUCT, "deleted", &id, None)
        .await;

    Ok(Json(true))
}

/// GET /api/products/:id/attributes - 获取商品的属性绑定列表
pub async fn list_product_attributes(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<Vec<AttributeBindingFull>>> {
    let attr_repo = AttributeRepository::new(state.db.clone());

    // Get attribute bindings for this product
    let bindings = attr_repo
        .find_bindings_for_product(&id)
        .await
        ?;

    // Convert to API type
    let result: Vec<AttributeBindingFull> = bindings
        .into_iter()
        .map(|(binding, attr)| AttributeBindingFull {
            id: binding.id,
            attribute: attr,
            is_required: binding.is_required,
            display_order: binding.display_order,
            default_option_idx: binding.default_option_idx,
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
    Path((product_id, tag_id)): Path<(String, String)>,
) -> AppResult<Json<shared::models::ProductFull>> {
    let product = state
        .catalog_service
        .add_product_tag(&product_id, &tag_id)
        .await
        ?;

    // 广播同步通知 (发送完整 ProductFull 数据)
    let product_for_api: shared::models::ProductFull = product.into();
    state
        .broadcast_sync(RESOURCE_PRODUCT, "updated", &product_id, Some(&product_for_api))
        .await;

    Ok(Json(product_for_api))
}

/// DELETE /api/products/:id/tags/:tag_id - 从商品移除标签
pub async fn remove_product_tag(
    State(state): State<ServerState>,
    Path((product_id, tag_id)): Path<(String, String)>,
) -> AppResult<Json<shared::models::ProductFull>> {
    let product = state
        .catalog_service
        .remove_product_tag(&product_id, &tag_id)
        .await
        ?;

    // 广播同步通知 (发送完整 ProductFull 数据)
    let product_for_api: shared::models::ProductFull = product.into();
    state
        .broadcast_sync(RESOURCE_PRODUCT, "updated", &product_id, Some(&product_for_api))
        .await;

    Ok(Json(product_for_api))
}

// =============================================================================
// Batch Sort Order
// =============================================================================

/// Payload for batch sort order update
#[derive(Debug, Deserialize)]
pub struct SortOrderUpdate {
    pub id: String,
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
                &update.id,
                ProductUpdate {
                    name: None,
                    sort_order: Some(update.sort_order),
                    image: None,
                    category: None,
                    tax_rate: None,
                    receipt_name: None,
                    kitchen_print_name: None,
                    kitchen_print_destinations: None,
                    label_print_destinations: None,
                    is_kitchen_print_enabled: None,
                    is_label_print_enabled: None,
                    is_active: None,
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
