use axum::{
    Extension, Json,
    extract::{Path, State},
};
use shared::cloud::store_op::{StoreOp, StoreOpResult};
use shared::error::AppError;

use crate::auth::tenant_auth::TenantIdentity;
use crate::db::store;
use crate::state::AppState;

use super::{internal, push_to_edge_if_online, verify_store};

type ApiResult<T> = Result<Json<T>, AppError>;

pub async fn list_tags(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<store::StoreTag>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let tags = store::list_tags(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(tags))
}

pub async fn create_tag(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::tag::TagCreate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let (source_id, op_data) = store::create_tag_direct(&state.pool, store_id, &data)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        StoreOp::CreateTag {
            id: Some(source_id),
            data,
        },
    );

    Ok(Json(StoreOpResult::created(source_id).with_data(op_data)))
}

pub async fn update_tag(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, tag_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::tag::TagUpdate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::update_tag_direct(&state.pool, store_id, tag_id, &data)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(&state, store_id, StoreOp::UpdateTag { id: tag_id, data });

    Ok(Json(StoreOpResult::ok()))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, tag_id)): Path<(i64, i64)>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::delete_tag_direct(&state.pool, store_id, tag_id)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(&state, store_id, StoreOp::DeleteTag { id: tag_id });

    Ok(Json(StoreOpResult::ok()))
}
