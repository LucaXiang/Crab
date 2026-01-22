//! Product API Handlers

use axum::{
    Json,
    extract::{Path, State},
};

use crate::api::convert::{option_thing_to_string, thing_to_string, things_to_strings};
use crate::core::ServerState;
use crate::db::models::{ProductCreate, ProductUpdate};
use crate::db::repository::{AttributeRepository, ProductRepository, TagRepository};
use crate::utils::{AppError, AppResult};

// API 返回类型使用 shared::models (String ID)
use shared::models::{AttributeBindingFull, EmbeddedSpec, Product, ProductFull};

const RESOURCE_PRODUCT: &str = "product";

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
    // 转换为 API 类型 (Thing -> String)
    Ok(Json(products.into_iter().map(Into::into).collect()))
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
    Ok(Json(products.into_iter().map(Into::into).collect()))
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
    Ok(Json(product.into()))
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
    let tag_ids: Vec<String> = product.tags.iter().map(|t| t.id.to_string()).collect();
    let mut tags = Vec::new();
    for tag_id in tag_ids {
        if let Some(tag) = tag_repo
            .find_by_id(&tag_id)
            .await
            .map_err(|e| AppError::database(e.to_string()))?
        {
            tags.push(tag.into());
        }
    }

    // Convert embedded specs to API type
    let specs: Vec<EmbeddedSpec> = product.specs.into_iter().map(Into::into).collect();

    // Convert attribute bindings
    let attr_bindings: Vec<AttributeBindingFull> = bindings
        .into_iter()
        .map(|(binding, attr)| AttributeBindingFull {
            id: binding.id.map(|t| t.to_raw()),
            attribute: attr.into(),
            is_required: binding.is_required,
            display_order: binding.display_order,
            default_option_idx: binding.default_option_idx,
        })
        .collect();

    // Build ProductFull
    let product_full = ProductFull {
        id: option_thing_to_string(&product.id),
        name: product.name,
        image: product.image,
        category: thing_to_string(&product.category),
        sort_order: product.sort_order,
        tax_rate: product.tax_rate,
        receipt_name: product.receipt_name,
        kitchen_print_name: product.kitchen_print_name,
        kitchen_print_destinations: things_to_strings(&product.kitchen_print_destinations),
        label_print_destinations: things_to_strings(&product.label_print_destinations),
        is_kitchen_print_enabled: product.is_kitchen_print_enabled,
        is_label_print_enabled: product.is_label_print_enabled,
        is_active: product.is_active,
        specs,
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
    let repo = ProductRepository::new(state.db.clone());
    let product = repo
        .create(payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // Get product ID for broadcast
    let product_id = product
        .id
        .clone()
        .ok_or_else(|| AppError::internal("Product created without ID"))?;

    // 广播同步通知
    let id = product_id.id.to_string();
    let api_product: Product = product.into();
    state
        .broadcast_sync(RESOURCE_PRODUCT, "created", &id, Some(&api_product))
        .await;

    Ok(Json(api_product))
}

/// PUT /api/products/:id - 更新商品
pub async fn update(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<ProductUpdate>,
) -> AppResult<Json<Product>> {
    let repo = ProductRepository::new(state.db.clone());
    let product = repo
        .update(&id, payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    let api_product: Product = product.into();
    state
        .broadcast_sync(RESOURCE_PRODUCT, "updated", &id, Some(&api_product))
        .await;

    Ok(Json(api_product))
}

/// DELETE /api/products/:id - 删除商品 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = ProductRepository::new(state.db.clone());
    let result = repo
        .delete(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // 广播同步通知
    if result {
        state
            .broadcast_sync::<()>(RESOURCE_PRODUCT, "deleted", &id, None)
            .await;
    }

    Ok(Json(result))
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
            id: binding.id.map(|t| t.to_raw()),
            attribute: attr.into(),
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
    let api_product: Product = product.into();
    state
        .broadcast_sync(RESOURCE_PRODUCT, "updated", &product_id, Some(&api_product))
        .await;

    Ok(Json(api_product))
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
    let api_product: Product = product.into();
    state
        .broadcast_sync(RESOURCE_PRODUCT, "updated", &product_id, Some(&api_product))
        .await;

    Ok(Json(api_product))
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
