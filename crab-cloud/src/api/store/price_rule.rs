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

pub async fn list_price_rules(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<shared::models::PriceRule>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let rules = store::list_price_rules(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(rules))
}

pub async fn create_price_rule(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::price_rule::PriceRuleCreate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let (source_id, op_data) = store::create_price_rule_direct(&state.pool, store_id, &data)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::CreatePriceRule {
            id: Some(source_id),
            data,
        },
    )
    .await;

    Ok(Json(StoreOpResult::created(source_id).with_data(op_data)))
}

pub async fn update_price_rule(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, rule_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::price_rule::PriceRuleUpdate>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::update_price_rule_direct(&state.pool, store_id, rule_id, &data)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(
        &state,
        store_id,
        StoreOp::UpdatePriceRule { id: rule_id, data },
    )
    .await;

    Ok(Json(StoreOpResult::ok()))
}

pub async fn delete_price_rule(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, rule_id)): Path<(i64, i64)>,
) -> ApiResult<StoreOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    store::delete_price_rule_direct(&state.pool, store_id, rule_id)
        .await
        .map_err(internal)?;
    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge(&state, store_id, StoreOp::DeletePriceRule { id: rule_id }).await;

    Ok(Json(StoreOpResult::ok()))
}
