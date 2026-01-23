//! Product API Handlers

use axum::{
    Json,
    extract::{Path, State},
};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::api::convert::thing_to_string;
use crate::core::ServerState;
use crate::db::models::{AttributeBindingFull, Product, ProductCreate, ProductFull, ProductUpdate};
use crate::db::repository::{AttributeRepository, ProductRepository, TagRepository};
use crate::utils::{AppError, AppResult, ErrorCode};

const RESOURCE_PRODUCT: &str = "product";

/// 检查 external_id 是否已存在，返回重复的 ID 列表
async fn check_duplicate_external_ids(
    db: &Surreal<Db>,
    external_ids: &[i64],
    exclude_product_id: Option<&str>,
) -> Option<Vec<i64>> {
    let query_result = if let Some(exclude_id) = exclude_product_id {
        // Strip "product:" prefix if present, since type::thing() will add it
        let exclude_id = exclude_id
            .strip_prefix("product:")
            .unwrap_or(exclude_id)
            .to_string();
        db.query("SELECT external_id FROM product_spec WHERE external_id IN $ids AND product != type::thing('product', $exclude)")
            .bind(("ids", external_ids.to_vec()))
            .bind(("exclude", exclude_id))
            .await
    } else {
        db.query("SELECT external_id FROM product_spec WHERE external_id IN $ids")
            .bind(("ids", external_ids.to_vec()))
            .await
    };

    let query = query_result.ok()?;

    #[derive(serde::Deserialize)]
    struct Found {
        external_id: i64,
    }

    let found: Vec<Found> = query.check().ok()?.take(0).unwrap_or_default();
    if found.is_empty() {
        None
    } else {
        Some(found.into_iter().map(|f| f.external_id).collect())
    }
}

// =============================================================================
// Product Handlers
// =============================================================================

/// GET /api/products - 获取所有商品
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<Product>>> {
    let repo = ProductRepository::new(state.db.clone());
    let products = repo
        .find_all()
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    Ok(Json(products))
}

/// GET /api/products/by-category/:category_id - 按分类获取商品
pub async fn list_by_category(
    State(state): State<ServerState>,
    Path(category_id): Path<String>,
) -> AppResult<Json<Vec<Product>>> {
    let repo = ProductRepository::new(state.db.clone());
    let products = repo
        .find_by_category(&category_id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
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
        .ok_or_else(|| AppError::not_found(format!("Product {}", id)))?;
    Ok(Json(product))
}

/// GET /api/products/:id/full - 获取商品完整信息 (含规格、属性、标签)
pub async fn get_full(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<ProductFull>> {
    let product_repo = ProductRepository::new(state.db.clone());
    let attr_repo = AttributeRepository::new(state.db.clone());
    let tag_repo = TagRepository::new(state.db.clone());

    // Get product (specs are now embedded)
    let product = product_repo
        .find_by_id(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Product {}", id)))?;

    // Get attribute bindings
    let bindings = attr_repo
        .find_bindings_for_product(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // Get full tag objects
    let tag_ids: Vec<String> = product.tags.iter().map(|t| t.to_string()).collect();
    let mut tags = Vec::new();
    for tag_id in tag_ids {
        if let Some(tag) = tag_repo
            .find_by_id(&tag_id)
            .await
            .map_err(|e| AppError::database(e.to_string()))?
        {
            tags.push(tag);
        }
    }

    // Convert attribute bindings
    let attr_bindings: Vec<AttributeBindingFull> = bindings
        .into_iter()
        .map(|(binding, attr)| AttributeBindingFull {
            id: binding.id,
            attribute: attr,
            is_required: binding.is_required,
            display_order: binding.display_order,
            default_option_idx: binding.default_option_idx,
        })
        .collect();

    // Build ProductFull
    let product_full = ProductFull {
        id: product.id,
        name: product.name,
        image: product.image,
        category: product.category,
        sort_order: product.sort_order,
        tax_rate: product.tax_rate,
        receipt_name: product.receipt_name,
        kitchen_print_name: product.kitchen_print_name,
        kitchen_print_destinations: product.kitchen_print_destinations,
        label_print_destinations: product.label_print_destinations,
        is_kitchen_print_enabled: product.is_kitchen_print_enabled,
        is_label_print_enabled: product.is_label_print_enabled,
        is_active: product.is_active,
        specs: product.specs,
        attributes: attr_bindings,
        tags,
    };

    Ok(Json(product_full))
}

/// POST /api/products - 创建商品
pub async fn create(
    State(state): State<ServerState>,
    Json(payload): Json<ProductCreate>,
) -> AppResult<Json<Product>> {
    // 检查 external_id 是否已存在
    let external_ids: Vec<i64> = payload.specs.iter().filter_map(|s| s.external_id).collect();
    if !external_ids.is_empty() {
        if let Some(duplicates) = check_duplicate_external_ids(&state.db, &external_ids, None).await {
            return Err(AppError::new(ErrorCode::SpecExternalIdExists)
                .with_detail("external_ids", duplicates));
        }
    }

    let repo = ProductRepository::new(state.db.clone());
    let product = repo
        .create(payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    let id = product.id.as_ref().map(thing_to_string).unwrap_or_default();
    state
        .broadcast_sync(RESOURCE_PRODUCT, "created", &id, Some(&product))
        .await;

    Ok(Json(product))
}

/// PUT /api/products/:id - 更新商品
pub async fn update(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<ProductUpdate>,
) -> AppResult<Json<Product>> {
    // Debug: log the received payload
    tracing::error!("!!! Product update - id: {}, tax_rate: {:?}, is_kitchen_print_enabled: {:?}, is_label_print_enabled: {:?}",
        id, payload.tax_rate, payload.is_kitchen_print_enabled, payload.is_label_print_enabled);

    // 检查 external_id 是否已存在 (排除当前产品)
    if let Some(ref specs) = payload.specs {
        let external_ids: Vec<i64> = specs.iter().filter_map(|s| s.external_id).collect();
        if !external_ids.is_empty() {
            if let Some(duplicates) = check_duplicate_external_ids(&state.db, &external_ids, Some(&id)).await {
                return Err(AppError::new(ErrorCode::SpecExternalIdExists)
                    .with_detail("external_ids", duplicates));
            }
        }
    }

    let repo = ProductRepository::new(state.db.clone());
    let product = repo
        .update(&id, payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // Debug: log the updated product
    tracing::error!("!!! Product updated - is_kitchen_print_enabled: {}, is_label_print_enabled: {}",
        product.is_kitchen_print_enabled, product.is_label_print_enabled);

    // 广播同步通知
    state
        .broadcast_sync(RESOURCE_PRODUCT, "updated", &id, Some(&product))
        .await;

    Ok(Json(product))
}

/// DELETE /api/products/:id - 删除商品
pub async fn delete(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = ProductRepository::new(state.db.clone());
    repo.delete(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
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
        .map_err(|e| AppError::database(e.to_string()))?;

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
) -> AppResult<Json<Product>> {
    let repo = ProductRepository::new(state.db.clone());
    let product = repo
        .add_tag(&product_id, &tag_id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    state
        .broadcast_sync(RESOURCE_PRODUCT, "updated", &product_id, Some(&product))
        .await;

    Ok(Json(product))
}

/// DELETE /api/products/:id/tags/:tag_id - 从商品移除标签
pub async fn remove_product_tag(
    State(state): State<ServerState>,
    Path((product_id, tag_id)): Path<(String, String)>,
) -> AppResult<Json<Product>> {
    let repo = ProductRepository::new(state.db.clone());
    let product = repo
        .remove_tag(&product_id, &tag_id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    state
        .broadcast_sync(RESOURCE_PRODUCT, "updated", &product_id, Some(&product))
        .await;

    Ok(Json(product))
}

// =============================================================================
// Batch Sort Order
// =============================================================================

use serde::Deserialize;

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

    let repo = ProductRepository::new(state.db.clone());
    let mut updated_count = 0;

    for update in &updates {
        tracing::debug!(id = %update.id, sort_order = update.sort_order, "Updating product sort order");

        let result = repo
            .update(
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
