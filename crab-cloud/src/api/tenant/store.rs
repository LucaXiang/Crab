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
    let stores = tenant_queries::list_stores(&state.pool, &identity.tenant_id)
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
    pub business_day_cutoff: Option<String>,
}

pub async fn update_store(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(payload): Json<UpdateStoreRequest>,
) -> ApiResult<shared::models::store_info::StoreInfo> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

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
        logo_url: None,
        phone: payload.phone,
        email: payload.email,
        website: payload.website,
        business_day_cutoff: payload.business_day_cutoff,
    };

    let info = store::update_store_info_direct(&state.pool, store_id, &update)
        .await
        .map_err(|e| {
            tracing::error!("Update store info error: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    Ok(Json(info))
}
