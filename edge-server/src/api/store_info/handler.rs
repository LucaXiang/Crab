//! Store Info API Handlers

use axum::{Json, extract::State};

use crate::core::ServerState;
use crate::db::models::{StoreInfo, StoreInfoUpdate};
use crate::db::repository::StoreInfoRepository;
use crate::utils::{AppError, AppResult};

const RESOURCE: &str = "store_info";

/// Get current store info
pub async fn get(State(state): State<ServerState>) -> AppResult<Json<StoreInfo>> {
    let repo = StoreInfoRepository::new(state.db.clone());
    let store_info = repo
        .get_or_create()
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(store_info))
}

/// Update store info
pub async fn update(
    State(state): State<ServerState>,
    Json(payload): Json<StoreInfoUpdate>,
) -> AppResult<Json<StoreInfo>> {
    let repo = StoreInfoRepository::new(state.db.clone());
    let store_info = repo
        .update(payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    state
        .broadcast_sync(RESOURCE, "updated", "main", Some(&store_info))
        .await;

    Ok(Json(store_info))
}
