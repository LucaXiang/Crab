use axum::{
    Extension, Json,
    extract::{Path, State},
};
use shared::cloud::store_op::{StoreOp, StoreOpResult};
use shared::error::{AppError, ErrorCode};

use crate::auth::tenant_auth::TenantIdentity;
use crate::db::{store, tenant_images};
use crate::state::AppState;

use super::{fire_ensure_image, internal, push_to_edge, verify_store};

type ApiResult<T> = Result<Json<T>, AppError>;

pub async fn list_products(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<store::StoreProduct>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let products = store::list_products(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(products))
}

pub async fn create_product(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::product::ProductCreate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    fire_ensure_image(&state, store_id, &identity.tenant_id, data.image.as_deref()).await;

    let (source_id, op_data) = store::create_product_direct(&state.pool, store_id, &data)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    // Track image reference
    if let Some(hash) = data.image.as_deref().filter(|h| !h.is_empty()) {
        let _ = tenant_images::increment_ref(&state.pool, &identity.tenant_id, hash).await;
    }

    push_to_edge(
        &state,
        store_id,
        StoreOp::CreateProduct {
            id: Some(source_id),
            data,
        },
    )
    .await;

    Ok(Json(StoreOpResult::created(source_id).with_data(op_data)))
}

pub async fn update_product(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, product_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::product::ProductUpdate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    fire_ensure_image(&state, store_id, &identity.tenant_id, data.image.as_deref()).await;

    // Capture old image hash before update
    let old_hash = tenant_images::get_product_image(&state.pool, store_id, product_id)
        .await
        .unwrap_or(None);

    store::update_product_direct(&state.pool, store_id, product_id, &data)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    // Update image references if image changed
    let new_hash = data.image.as_deref().filter(|h| !h.is_empty());
    if new_hash != old_hash.as_deref() {
        let now = shared::util::now_millis();
        if let Some(h) = new_hash {
            let _ = tenant_images::increment_ref(&state.pool, &identity.tenant_id, h).await;
        }
        if let Some(h) = &old_hash {
            let _ = tenant_images::decrement_ref(&state.pool, &identity.tenant_id, h, now).await;
        }
    }

    push_to_edge(
        &state,
        store_id,
        StoreOp::UpdateProduct {
            id: product_id,
            data,
        },
    )
    .await;

    Ok(Json(StoreOpResult::ok()))
}

#[derive(serde::Deserialize)]
pub struct BatchSortOrderRequest {
    pub items: Vec<shared::cloud::store_op::SortOrderItem>,
}

pub async fn batch_update_product_sort_order(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(req): Json<BatchSortOrderRequest>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::batch_update_sort_order_products(&state.pool, store_id, &req.items)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::BatchUpdateProductSortOrder { items: req.items },
    )
    .await;

    Ok(Json(StoreOpResult::ok()))
}

#[derive(serde::Deserialize)]
pub struct BulkDeleteRequest {
    pub ids: Vec<i64>,
}

pub async fn bulk_delete_products(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(req): Json<BulkDeleteRequest>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    if req.ids.len() > 200 {
        return Err(AppError::with_message(
            ErrorCode::ValidationFailed,
            "Too many IDs (max 200)",
        ));
    }

    // Capture image hashes before deletion
    let old_hashes = tenant_images::get_product_images_bulk(&state.pool, store_id, &req.ids)
        .await
        .unwrap_or_default();

    store::bulk_delete_products(&state.pool, store_id, &req.ids)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    // Decrement image references
    let now = shared::util::now_millis();
    for hash in &old_hashes {
        let _ = tenant_images::decrement_ref(&state.pool, &identity.tenant_id, hash, now).await;
    }

    for id in &req.ids {
        push_to_edge(&state, store_id, StoreOp::DeleteProduct { id: *id }).await;
    }

    Ok(Json(StoreOpResult::ok()))
}

pub async fn delete_product(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, product_id)): Path<(i64, i64)>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    // Capture image hash before deletion
    let old_hash = tenant_images::get_product_image(&state.pool, store_id, product_id)
        .await
        .unwrap_or(None);

    store::delete_product_direct(&state.pool, store_id, product_id)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    // Decrement image reference
    if let Some(hash) = &old_hash {
        let now = shared::util::now_millis();
        let _ = tenant_images::decrement_ref(&state.pool, &identity.tenant_id, hash, now).await;
    }

    push_to_edge(&state, store_id, StoreOp::DeleteProduct { id: product_id }).await;

    Ok(Json(StoreOpResult::ok()))
}
