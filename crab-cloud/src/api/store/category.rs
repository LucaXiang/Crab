use axum::{
    Extension, Json,
    extract::{Path, State},
};
use shared::cloud::store_op::{StoreOp, StoreOpResult};
use shared::error::AppError;

use crate::auth::tenant_auth::TenantIdentity;
use crate::db::store;
use crate::state::AppState;

use super::{internal, push_to_edge, verify_store};

type ApiResult<T> = Result<Json<T>, AppError>;

pub async fn list_categories(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<store::StoreCategory>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let categories = store::list_categories(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(categories))
}

pub async fn create_category(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::category::CategoryCreate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let (source_id, op_data) = store::create_category_direct(&state.pool, store_id, &data)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::CreateCategory {
            id: Some(source_id),
            data,
        },
    )
    .await;

    Ok(Json(StoreOpResult::created(source_id).with_data(op_data)))
}

pub async fn update_category(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, category_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::category::CategoryUpdate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::update_category_direct(&state.pool, store_id, category_id, &data)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::UpdateCategory {
            id: category_id,
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

pub async fn batch_update_category_sort_order(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(req): Json<BatchSortOrderRequest>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::batch_update_sort_order_categories(&state.pool, store_id, &req.items)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::BatchUpdateCategorySortOrder { items: req.items },
    )
    .await;

    Ok(Json(StoreOpResult::ok()))
}

pub async fn delete_category(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, category_id)): Path<(i64, i64)>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::delete_category_direct(&state.pool, store_id, category_id)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::DeleteCategory { id: category_id },
    )
    .await;

    Ok(Json(StoreOpResult::ok()))
}
