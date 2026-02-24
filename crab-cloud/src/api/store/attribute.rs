use axum::{
    Extension, Json,
    extract::{Path, State},
};
use shared::cloud::store_op::{BindingOwner, StoreOp, StoreOpResult};
use shared::error::AppError;

use crate::auth::tenant_auth::TenantIdentity;
use crate::db::store;
use crate::state::AppState;

use super::{internal, push_to_edge, verify_store};

type ApiResult<T> = Result<Json<T>, AppError>;

pub async fn list_attributes(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<store::StoreAttribute>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let attributes = store::list_attributes(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(attributes))
}

pub async fn create_attribute(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::attribute::AttributeCreate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let (source_id, op_data) = store::create_attribute_direct(&state.pool, store_id, &data)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::CreateAttribute {
            id: Some(source_id),
            data,
        },
    )
    .await;

    Ok(Json(StoreOpResult::created(source_id).with_data(op_data)))
}

pub async fn update_attribute(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, attr_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::attribute::AttributeUpdate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::update_attribute_direct(&state.pool, store_id, attr_id, &data)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::UpdateAttribute { id: attr_id, data },
    )
    .await;

    Ok(Json(StoreOpResult::ok()))
}

pub async fn delete_attribute(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, attr_id)): Path<(i64, i64)>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::delete_attribute_direct(&state.pool, store_id, attr_id)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(&state, store_id, StoreOp::DeleteAttribute { id: attr_id }).await;

    Ok(Json(StoreOpResult::ok()))
}

// ── Attribute Option Independent CRUD ──

pub async fn create_attribute_option(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, attr_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::attribute::AttributeOptionCreate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let source_id = store::create_option_direct(&state.pool, store_id, attr_id, &data)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::CreateAttributeOption {
            attribute_id: attr_id,
            id: Some(source_id),
            data,
        },
    )
    .await;

    Ok(Json(StoreOpResult::created(source_id)))
}

pub async fn update_attribute_option(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, _attr_id, option_id)): Path<(i64, i64, i64)>,
    Json(data): Json<shared::models::attribute::AttributeOptionUpdate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::update_option_direct(&state.pool, store_id, option_id, &data)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::UpdateAttributeOption {
            id: option_id,
            data,
        },
    )
    .await;

    Ok(Json(StoreOpResult::ok()))
}

pub async fn delete_attribute_option(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, _attr_id, option_id)): Path<(i64, i64, i64)>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::delete_option_direct(&state.pool, store_id, option_id)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::DeleteAttributeOption { id: option_id },
    )
    .await;

    Ok(Json(StoreOpResult::ok()))
}

#[derive(serde::Deserialize)]
pub struct BatchOptionSortOrderRequest {
    pub items: Vec<shared::cloud::store_op::SortOrderItem>,
}

pub async fn batch_update_option_sort_order(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, attr_id)): Path<(i64, i64)>,
    Json(req): Json<BatchOptionSortOrderRequest>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::batch_update_option_sort_order(&state.pool, store_id, &req.items)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::BatchUpdateOptionSortOrder {
            attribute_id: attr_id,
            items: req.items,
        },
    )
    .await;

    Ok(Json(StoreOpResult::ok()))
}

// ── Attribute Binding ──

#[derive(serde::Deserialize)]
pub struct BindAttributeRequest {
    pub owner: BindingOwner,
    pub attribute_id: i64,
    #[serde(default)]
    pub is_required: bool,
    #[serde(default)]
    pub display_order: i32,
    pub default_option_ids: Option<Vec<i32>>,
}

#[derive(serde::Deserialize)]
pub struct UnbindAttributeRequest {
    pub binding_id: i64,
}

pub async fn bind_attribute(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(req): Json<BindAttributeRequest>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let binding_id = store::bind_attribute_direct(
        &state.pool,
        store_id,
        store::BindAttributeParams {
            owner_type: req.owner.owner_type(),
            owner_id: req.owner.owner_id(),
            attribute_id: req.attribute_id,
            is_required: req.is_required,
            display_order: req.display_order,
            default_option_ids: req.default_option_ids.clone(),
        },
    )
    .await
    .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::BindAttribute {
            owner: req.owner,
            attribute_id: req.attribute_id,
            is_required: req.is_required,
            display_order: req.display_order,
            default_option_ids: req.default_option_ids,
        },
    )
    .await;

    Ok(Json(StoreOpResult::created(binding_id)))
}

pub async fn unbind_attribute(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(req): Json<UnbindAttributeRequest>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::unbind_attribute_direct(&state.pool, store_id, req.binding_id)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::UnbindAttribute {
            binding_id: req.binding_id,
        },
    )
    .await;

    Ok(Json(StoreOpResult::ok()))
}
