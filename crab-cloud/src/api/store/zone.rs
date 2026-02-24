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

pub async fn list_zones(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<shared::models::zone::Zone>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let zones = store::list_zones(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(zones))
}

pub async fn create_zone(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::zone::ZoneCreate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let (source_id, op_data) =
        store::create_zone_direct(&state.pool, store_id, &identity.tenant_id, &data)
            .await
            .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        StoreOp::CreateZone {
            id: Some(source_id),
            data,
        },
    );

    Ok(Json(StoreOpResult::created(source_id).with_data(op_data)))
}

pub async fn update_zone(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, zone_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::zone::ZoneUpdate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let op_data = store::update_zone_direct(&state.pool, store_id, zone_id, &data)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(&state, store_id, StoreOp::UpdateZone { id: zone_id, data });

    Ok(Json(StoreOpResult::ok().with_data(op_data)))
}

pub async fn delete_zone(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, zone_id)): Path<(i64, i64)>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::delete_zone_direct(&state.pool, store_id, zone_id)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(&state, store_id, StoreOp::DeleteZone { id: zone_id });

    Ok(Json(StoreOpResult::ok()))
}
