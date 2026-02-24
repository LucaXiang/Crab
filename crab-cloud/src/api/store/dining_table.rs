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

pub async fn list_tables(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<shared::models::dining_table::DiningTable>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let tables = store::list_tables(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(tables))
}

pub async fn create_table(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::dining_table::DiningTableCreate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let (source_id, op_data) =
        store::create_table_direct(&state.pool, store_id, &identity.tenant_id, &data)
            .await
            .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        StoreOp::CreateTable {
            id: Some(source_id),
            data,
        },
    );

    Ok(Json(StoreOpResult::created(source_id).with_data(op_data)))
}

pub async fn update_table(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, table_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::dining_table::DiningTableUpdate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let op_data = store::update_table_direct(&state.pool, store_id, table_id, &data)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        StoreOp::UpdateTable { id: table_id, data },
    );

    Ok(Json(StoreOpResult::ok().with_data(op_data)))
}

pub async fn delete_table(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, table_id)): Path<(i64, i64)>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::delete_table_direct(&state.pool, store_id, table_id)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(&state, store_id, StoreOp::DeleteTable { id: table_id });

    Ok(Json(StoreOpResult::ok()))
}
