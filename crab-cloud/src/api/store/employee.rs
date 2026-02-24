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

pub async fn list_employees(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<shared::models::employee::Employee>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let employees = store::list_employees(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(employees))
}

pub async fn create_employee(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::employee::EmployeeCreate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let (source_id, op_data) =
        store::create_employee_direct(&state.pool, store_id, &identity.tenant_id, &data)
            .await
            .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::CreateEmployee {
            id: Some(source_id),
            data,
        },
    )
    .await;

    Ok(Json(StoreOpResult::created(source_id).with_data(op_data)))
}

pub async fn update_employee(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, employee_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::employee::EmployeeUpdate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let op_data = store::update_employee_direct(&state.pool, store_id, employee_id, &data)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::UpdateEmployee {
            id: employee_id,
            data,
        },
    )
    .await;

    Ok(Json(StoreOpResult::ok().with_data(op_data)))
}

pub async fn delete_employee(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, employee_id)): Path<(i64, i64)>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::delete_employee_direct(&state.pool, store_id, employee_id)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::DeleteEmployee { id: employee_id },
    )
    .await;

    Ok(Json(StoreOpResult::ok()))
}
