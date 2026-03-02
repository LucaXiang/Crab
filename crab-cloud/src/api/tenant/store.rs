//! Store management endpoints: list, update

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use serde::Deserialize;
use shared::error::{AppError, ErrorCode};

use crate::auth::tenant_auth::TenantIdentity;
use crate::db::{store, tenant_queries};
use crate::state::AppState;

use super::{ApiResult, verify_store};

/// GET /api/tenant/stores
pub async fn list_stores(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
) -> ApiResult<Vec<shared::cloud::StoreDetailResponse>> {
    let stores = tenant_queries::list_stores(&state.pool, identity.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Stores query error: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    let result: Vec<_> = stores
        .into_iter()
        .map(|s| shared::cloud::StoreDetailResponse {
            id: s.id,
            entity_id: s.entity_id,
            alias: s.alias,
            name: s.name,
            address: s.address,
            phone: s.phone,
            nif: s.nif,
            email: s.email,
            website: s.website,
            business_day_cutoff: s.business_day_cutoff,
            device_id: s.device_id,
            is_online: state.edges.connected.contains_key(&s.id),
            last_sync_at: s.last_sync_at,
            registered_at: s.registered_at,
        })
        .collect();

    Ok(Json(result))
}

/// PATCH /api/tenant/stores/:id
#[derive(Deserialize)]
pub struct UpdateStoreRequest {
    pub alias: Option<String>,
    pub name: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub nif: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub business_day_cutoff: Option<i32>,
    pub currency_code: Option<String>,
    pub currency_symbol: Option<String>,
    pub currency_decimal_places: Option<i32>,
    pub timezone: Option<String>,
    pub receipt_locale: Option<String>,
    pub receipt_header: Option<String>,
    pub receipt_footer: Option<String>,
}

pub async fn update_store(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(payload): Json<UpdateStoreRequest>,
) -> ApiResult<shared::models::store_info::StoreInfo> {
    verify_store(&state, store_id, identity.tenant_id).await?;

    if let Some(alias) = &payload.alias {
        store::update_store_alias(&state.pool, store_id, alias)
            .await
            .map_err(|e| {
                tracing::error!("Update store alias error: {e}");
                AppError::new(ErrorCode::InternalError)
            })?;
    }

    let update = shared::models::store_info::StoreInfoUpdate {
        name: payload.name,
        address: payload.address,
        nif: payload.nif,
        phone: payload.phone,
        email: payload.email,
        website: payload.website,
        business_day_cutoff: payload.business_day_cutoff,
        currency_code: payload.currency_code,
        currency_symbol: payload.currency_symbol,
        currency_decimal_places: payload.currency_decimal_places,
        timezone: payload.timezone,
        receipt_locale: payload.receipt_locale,
        receipt_header: payload.receipt_header,
        receipt_footer: payload.receipt_footer,
        ..Default::default()
    };

    let info = store::update_store_info_direct(&state.pool, store_id, &update)
        .await
        .map_err(|e| {
            tracing::error!("Update store info error: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    // Push to edge + broadcast to consoles
    use shared::cloud::store_op::StoreOp;
    crate::api::store::push_to_edge(
        &state,
        store_id,
        identity.tenant_id,
        StoreOp::UpdateStoreInfo { data: update },
    )
    .await;
    state
        .live_orders
        .publish_store_info_updated(identity.tenant_id, store_id, info.clone());

    Ok(Json(info))
}

/// DELETE /api/tenant/stores/:id
pub async fn delete_store(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<()> {
    verify_store(&state, store_id, identity.tenant_id).await?;

    let now = shared::util::now_millis();
    tenant_queries::soft_delete_store(&state.pool, store_id, identity.tenant_id, now)
        .await
        .map_err(|e| {
            tracing::error!("Delete store error: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    Ok(Json(()))
}

/// GET /api/tenant/stores/:id/devices
pub async fn list_devices(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<store::devices::DeviceRecord>> {
    verify_store(&state, store_id, identity.tenant_id).await?;

    let entity_id = tenant_queries::get_store_entity_id(&state.pool, store_id, identity.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Get store entity_id error: {e}");
            AppError::new(ErrorCode::InternalError)
        })?
        .ok_or_else(|| AppError::with_message(ErrorCode::NotFound, "Store not found"))?;

    let devices =
        store::devices::list_devices_for_store(&state.pool, &entity_id, identity.tenant_id)
            .await
            .map_err(|e| {
                tracing::error!("List devices error: {e}");
                AppError::new(ErrorCode::InternalError)
            })?;

    Ok(Json(devices))
}
