use axum::{
    Extension, Json,
    extract::{Path, State},
};
use shared::cloud::store_op::{StoreOp, StoreOpData, StoreOpResult};
use shared::error::AppError;
use shared::models::store_info::{StoreInfo, StoreInfoUpdate};

use crate::auth::tenant_auth::TenantIdentity;
use crate::db::store;
use crate::state::AppState;

use super::{internal, push_to_edge, verify_store};

type ApiResult<T> = Result<Json<T>, AppError>;

pub async fn get_store_info(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Option<StoreInfo>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let info = store::get_store_info(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(info))
}

pub async fn update_store_info(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<StoreInfoUpdate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let info = store::update_store_info_direct(&state.pool, store_id, &data)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(&state, store_id, StoreOp::UpdateStoreInfo { data }).await;

    Ok(Json(
        StoreOpResult::ok().with_data(StoreOpData::StoreInfo(info)),
    ))
}
