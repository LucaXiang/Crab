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

    let mut result = Vec::new();
    for store in stores {
        let store_info = crate::db::store::get_store_info(&state.pool, store.id)
            .await
            .map_err(|e| {
                tracing::error!(store_id = store.id, "Failed to get store_info: {e}");
                AppError::new(ErrorCode::InternalError)
            })?
            .and_then(|info| serde_json::to_value(info).ok());

        result.push(shared::cloud::StoreDetailResponse {
            id: store.id,
            entity_id: store.entity_id,
            name: store.name,
            address: store.address,
            phone: store.phone,
            nif: store.nif,
            email: store.email,
            website: store.website,
            business_day_cutoff: store.business_day_cutoff,
            device_id: store.device_id,
            is_online: state.edges.connected.contains_key(&store.id),
            last_sync_at: store.last_sync_at,
            registered_at: store.registered_at,
            store_info,
        });
    }

    Ok(Json(result))
}

/// PATCH /api/tenant/stores/:id
#[derive(Deserialize)]
pub struct UpdateStoreRequest {
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
