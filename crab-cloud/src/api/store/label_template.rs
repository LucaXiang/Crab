use axum::{
    Extension, Json,
    extract::{Path, State},
};
use shared::cloud::store_op::{StoreOp, StoreOpResult};
use shared::error::AppError;
use shared::models::label_template::LabelTemplate;

use crate::auth::tenant_auth::TenantIdentity;
use crate::db::store;
use crate::state::AppState;

use super::{internal, push_to_edge_if_online, verify_store};

type ApiResult<T> = Result<Json<T>, AppError>;

pub async fn list_label_templates(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<LabelTemplate>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let templates = store::list_label_templates(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(templates))
}

pub async fn create_label_template(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::label_template::LabelTemplateCreate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let (pg_id, op_data) =
        store::create_label_template_direct(&state.pool, store_id, &identity.tenant_id, &data)
            .await
            .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        StoreOp::CreateLabelTemplate {
            id: Some(pg_id),
            data,
        },
    );

    Ok(Json(StoreOpResult::created(pg_id).with_data(op_data)))
}

pub async fn update_label_template(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, template_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::label_template::LabelTemplateUpdate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let op_data = store::update_label_template_direct(&state.pool, store_id, template_id, &data)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        StoreOp::UpdateLabelTemplate {
            id: template_id,
            data,
        },
    );

    Ok(Json(StoreOpResult::ok().with_data(op_data)))
}

pub async fn delete_label_template(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, template_id)): Path<(i64, i64)>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::delete_label_template_direct(&state.pool, store_id, template_id)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        StoreOp::DeleteLabelTemplate { id: template_id },
    );

    Ok(Json(StoreOpResult::ok()))
}
